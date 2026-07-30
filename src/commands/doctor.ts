import { dirname, join } from "node:path";
import { parseArgs } from "node:util";
import { loadConfig, type AgnosgramConfig } from "../core/config.js";
import { daysBetween, todayIso } from "../core/dates.js";
import { extractRecords, validateRecord, type Frontmatter } from "../core/frontmatter.js";
import { parseFreshnessTable } from "../core/freshness.js";
import { INJECTION_PATTERNS, scanPatterns, SECRET_PATTERNS } from "../core/lint.js";
import { KNOWN_META_TYPES } from "../core/meta.js";
import { info, printStructured, UserError } from "../core/output.js";
import { findProjectRoot, hasStore, memoryDir } from "../core/paths.js";
import { resolveFormat } from "../core/serialize.js";
import { pathExists, readStore, type StoreFile } from "../core/store.js";
import { estimateTokens } from "../core/tokens.js";

export type Level = "error" | "warn";

export interface Finding {
  level: Level;
  code: string;
  /** Project-relative path the finding is about. */
  file: string;
  message: string;
  line?: number;
  id?: string;
}

/** Similarity above which two same-type records are flagged as near-duplicates. */
const NEAR_DUP_THRESHOLD = 0.6;
const MIN_DUP_WORDS = 6;

interface KnownRecord {
  file: string;
  line: number;
  frontmatter: Frontmatter;
  bodyWords: Set<string>;
}

function wordSet(text: string): Set<string> {
  return new Set(text.toLowerCase().match(/[a-z0-9]+/g) ?? []);
}

function jaccard(a: Set<string>, b: Set<string>): number {
  if (a.size === 0 || b.size === 0) return 0;
  let inter = 0;
  for (const w of a) if (b.has(w)) inter++;
  return inter / (a.size + b.size - inter);
}

/** Markdown links to local files that do not exist, relative to the file's dir. */
function checkBrokenLinks(file: StoreFile, findings: Finding[]): void {
  const linkRe = /\[[^\]]*\]\(([^)]+)\)/g;
  const lines = file.text.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    let m: RegExpExecArray | null;
    linkRe.lastIndex = 0;
    while ((m = linkRe.exec(lines[i]!)) !== null) {
      const rawTarget = m[1]!.trim().split(/\s+/)[0]!; // drop optional "title"
      if (/^(?:[a-z]+:)?\/\//i.test(rawTarget) || rawTarget.startsWith("#") || rawTarget.startsWith("mailto:")) {
        continue;
      }
      const target = rawTarget.split("#")[0]!;
      if (target === "") continue;
      const resolved = join(dirname(file.path), target);
      if (!pathExists(resolved)) {
        findings.push({
          level: "warn",
          code: "link.broken",
          file: file.rel,
          line: i + 1,
          message: `broken link to "${target}"`,
        });
      }
    }
  }
}

export function collectFindings(root: string, config: AgnosgramConfig): Finding[] {
  const findings: Finding[] = [];
  const today = todayIso();

  // 0. Format version. The freeze (DEC-0002) promises a breaking change would
  // arrive as `version: 2`; validating a newer store as if it were v1 would
  // defeat that, so refuse loudly instead of guessing.
  if (config.version !== 1) {
    findings.push({
      level: "error",
      code: "config.version.unsupported",
      file: ".agnosgram/config.yml",
      message: `config declares format version ${config.version}; this release only understands version 1`,
    });
  }
  const store = readStore(root);
  const known: KnownRecord[] = [];
  const idLocations = new Map<string, Array<{ file: string; line: number }>>();

  // 1. Schema validation + record inventory. meta/ (tool-friction, see
  // core/meta.ts) validates against its own type enum - an additive
  // namespace, not part of the frozen lessons/decisions contract - but
  // shares every check below (duplicate ids, staleness, budgets, ...).
  for (const file of store) {
    if (!file.recordBearing) continue;
    const isMeta = file.storeRel.startsWith("meta/");
    for (const raw of extractRecords(file.text)) {
      const { frontmatter, issues } = validateRecord(raw, isMeta ? KNOWN_META_TYPES : undefined);
      for (const issue of issues) {
        findings.push({
          level: issue.level,
          code: `schema.${issue.code}`,
          file: file.rel,
          line: issue.line,
          message: issue.message,
        });
      }
      const idValue = typeof raw.data.id === "string" ? raw.data.id : undefined;
      if (idValue) {
        const list = idLocations.get(idValue) ?? [];
        list.push({ file: file.rel, line: raw.line });
        idLocations.set(idValue, list);
      }
      if (frontmatter) {
        known.push({
          file: file.rel,
          line: raw.line,
          frontmatter,
          bodyWords: wordSet(raw.body),
        });
      }
    }
  }

  // 2. Duplicate ids.
  for (const [id, locs] of idLocations) {
    if (locs.length > 1) {
      for (const loc of locs) {
        findings.push({
          level: "error",
          code: "id.duplicate",
          file: loc.file,
          line: loc.line,
          id,
          message: `duplicate id "${id}" (also at ${locs
            .filter((l) => l !== loc)
            .map((l) => `${l.file}:${l.line}`)
            .join(", ")})`,
        });
      }
    }
  }

  // 3. Orphan supersedes references.
  const allIds = new Set(idLocations.keys());
  for (const rec of known) {
    for (const ref of rec.frontmatter.supersedes ?? []) {
      if (!allIds.has(ref)) {
        findings.push({
          level: "warn",
          code: "supersedes.orphan",
          file: rec.file,
          line: rec.line,
          id: rec.frontmatter.id,
          message: `supersedes "${ref}", which no record defines`,
        });
      }
    }
  }

  // 4. Near-duplicate heuristic (same type, high body overlap).
  for (let i = 0; i < known.length; i++) {
    for (let j = i + 1; j < known.length; j++) {
      const a = known[i]!;
      const b = known[j]!;
      if (a.frontmatter.type !== b.frontmatter.type) continue;
      if (a.bodyWords.size < MIN_DUP_WORDS || b.bodyWords.size < MIN_DUP_WORDS) continue;
      const sim = jaccard(a.bodyWords, b.bodyWords);
      if (sim >= NEAR_DUP_THRESHOLD) {
        findings.push({
          level: "warn",
          code: "record.near-duplicate",
          file: b.file,
          line: b.line,
          id: b.frontmatter.id,
          message: `near-duplicate of ${a.frontmatter.id} (${Math.round(sim * 100)}% overlap); merge via supersedes: instead of keeping both`,
        });
      }
    }
  }

  // 5. Record staleness.
  for (const rec of known) {
    const age = daysBetween(rec.frontmatter.last_verified, today);
    if (age > config.staleness_days) {
      findings.push({
        level: "warn",
        code: "record.stale",
        file: rec.file,
        line: rec.line,
        id: rec.frontmatter.id,
        message: `not verified in ${age} days (limit ${config.staleness_days}); re-check and bump last_verified`,
      });
    }
    // source path integrity for records. Line anchors (#L88) break on the next
    // append, so they are flagged; the path part must exist under .agnosgram/,
    // where an archived journal month (journal/archive/) still counts.
    const src = rec.frontmatter.source;
    const hash = src.indexOf("#");
    const srcPath = hash === -1 ? src : src.slice(0, hash);
    if (hash !== -1) {
      findings.push({
        level: "warn",
        code: "source.anchor",
        file: rec.file,
        line: rec.line,
        id: rec.frontmatter.id,
        message: `source "${src}" uses a line anchor, which breaks on the next append; reference the whole file`,
      });
    }
    const candidates = [srcPath, srcPath.replace(/^journal\//, "journal/archive/")];
    if (!candidates.some((c) => c !== "" && pathExists(join(memoryDir(root), c)))) {
      findings.push({
        level: "warn",
        code: "source.missing",
        file: rec.file,
        line: rec.line,
        id: rec.frontmatter.id,
        message: `source "${src}" does not exist under .agnosgram/ (journal/archive/ was also checked)`,
      });
    }
  }

  // 6. Budgets (per-file token limits from config).
  for (const [rel, budget] of Object.entries(config.budgets)) {
    const abs = join(memoryDir(root), rel);
    if (!pathExists(abs)) continue;
    const found = store.find((f) => f.storeRel === rel);
    const tokens = found ? estimateTokens(found.text) : 0;
    if (tokens > budget) {
      findings.push({
        level: "warn",
        code: "budget.over",
        file: `.agnosgram/${rel}`,
        message: `~${tokens} tokens over the ${budget}-token budget; distill or split`,
      });
    }
  }

  // 7. File-level freshness table.
  const memoryFile = store.find((f) => f.storeRel === "MEMORY.md");
  if (memoryFile) {
    for (const row of parseFreshnessTable(memoryFile.text)) {
      const age = daysBetween(row.lastVerified, today);
      if (age > config.staleness_days) {
        findings.push({
          level: "warn",
          code: "file.stale",
          file: `.agnosgram/${row.file}`,
          message: `freshness table: not verified in ${age} days (limit ${config.staleness_days})`,
        });
      }
    }
  }

  // 8. Safety lints: secrets (error) + prompt-injection imperatives (warn).
  for (const file of store) {
    for (const hit of scanPatterns(file.text, SECRET_PATTERNS)) {
      findings.push({
        level: "error",
        code: `secret.${hit.code}`,
        file: file.rel,
        line: hit.line,
        message: `possible ${hit.label} committed to memory; remove and rotate it`,
      });
    }
    for (const hit of scanPatterns(file.text, INJECTION_PATTERNS)) {
      findings.push({
        level: "warn",
        code: `injection.${hit.code}`,
        file: file.rel,
        line: hit.line,
        message: `${hit.label} in stored memory ("${hit.match}"); memory must not command the agent`,
      });
    }
    checkBrokenLinks(file, findings);
  }

  findings.sort(
    (a, b) => a.file.localeCompare(b.file) || (a.line ?? 0) - (b.line ?? 0) || a.code.localeCompare(b.code),
  );
  return findings;
}

export interface DoctorReport {
  ok: boolean;
  errors: number;
  warnings: number;
  findings: Finding[];
}

export function runDoctorChecks(root: string): DoctorReport {
  const config = loadConfig(root);
  const findings = collectFindings(root, config);
  const errors = findings.filter((f) => f.level === "error").length;
  const warnings = findings.filter((f) => f.level === "warn").length;
  return { ok: errors === 0, errors, warnings, findings };
}

export function runDoctor(argv: string[]): void {
  const { values } = parseArgs({
    args: argv,
    allowPositionals: false,
    options: {
      json: { type: "boolean", default: false },
      format: { type: "string" },
      strict: { type: "boolean", default: false },
    },
  });

  const root = findProjectRoot();
  if (!hasStore(root)) {
    throw new UserError("No .agnosgram/ store found. Run `agnosgram init` first.");
  }

  const report = runDoctorChecks(root);
  const format = resolveFormat(values);

  if (format === "human") {
    renderReport(report);
  } else {
    printStructured(report, format);
  }

  if (report.errors > 0 || (values.strict && report.warnings > 0)) {
    process.exitCode = 1;
  }
}

function renderReport(report: DoctorReport): void {
  if (report.findings.length === 0) {
    info("doctor: no issues found. The store is healthy.");
    return;
  }

  let currentFile = "";
  for (const f of report.findings) {
    if (f.file !== currentFile) {
      currentFile = f.file;
      info(`\n${currentFile}`);
    }
    const where = f.line ? `:${f.line}` : "";
    const tag = f.level === "error" ? "error" : "warn ";
    info(`  ${tag} ${f.code}${where}  ${f.message}`);
  }

  info("");
  info(`doctor: ${report.errors} error(s), ${report.warnings} warning(s).`);
}
