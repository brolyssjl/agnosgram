/**
 * `agnosgram advise <plan-path>` - the landmine-catcher. Cross-checks a plan
 * (or spec) against everything the store has learned (`lessons/`,
 * `decisions/`) and flags contradictions before the plan becomes code.
 *
 * Two-step pattern, same shape as `distill` (see `src/commands/distill.ts`):
 * step 1 emits a prompt for whatever agent is present; step 2 (`--validate`)
 * mechanically checks the JSON report the agent wrote back, against the
 * pinned `agnosgram_advise` schema (a public contract shared with Gate - see
 * docs/advise.md). The CLI itself never calls an LLM and never judges content,
 * only shape and provenance.
 */
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { isValidIsoDate } from "../core/dates.js";
import { info, printStructured, UserError } from "../core/output.js";
import { findProjectRoot, hasStore } from "../core/paths.js";
import { loadRecords, type StoreRecord } from "../core/records.js";
import { resolveFormat } from "../core/serialize.js";

const KNOWN_KINDS = ["empirical", "normative"] as const;
const KNOWN_SEVERITIES = ["blocker", "caution"] as const;
const KNOWN_CONFIDENCE = ["low", "medium", "high"] as const;

/** Current schema version of the pinned `agnosgram_advise` report contract. */
export const ADVISE_SCHEMA_VERSION = 1;

function digestTable(records: StoreRecord[]): string {
  const rows = records
    .slice()
    .sort((a, b) => a.frontmatter.id.localeCompare(b.frontmatter.id))
    .map((r) => {
      const normalized = r.body.replace(/\s+/g, " ").trim();
      const truncated = normalized.length > 80;
      const snippet = normalized.slice(0, 80).replace(/\|/g, "\\|");
      return `| ${r.frontmatter.id} | ${r.frontmatter.type} | ${r.frontmatter.scope.join(",")} | ${r.frontmatter.confidence} | ${r.frontmatter.last_verified} | ${snippet}${truncated ? "..." : ""} |`;
    });
  const header = "| id | type | scope | confidence | last_verified | excerpt |\n|---|---|---|---|---|---|";
  return rows.length > 0 ? `${header}\n${rows.join("\n")}` : `${header}\n_(store has no records yet)_`;
}

function buildPrompt(root: string, planPath: string, outPath: string): string {
  const records = loadRecords(root);
  return `# Agnosgram advise task

You are reviewing a plan for contradictions against this project's memory
store at \`.agnosgram/\`. Do NOT invent facts; only flag a contradiction when a
stored record actually conflicts with something the plan says or assumes.

## Plan to review
\`${planPath}\`

## Digest: every lesson and decision currently in the store
${digestTable(records)}

## Precedence rule (verbatim - apply exactly, do not reinterpret)
- Normative contradiction (the plan proposes a different convention/policy than
  a stored decision or convention): the spec (the plan) wins by default; record
  the exception only if a stored record explicitly permits deviating.
- Empirical contradiction (the plan assumes a fact a stored pitfall/lesson
  directly contradicts): the contradiction wins - flag it; a human arbitrates.

## Output rules
1. Write a JSON report to \`${outPath}\` (this exact path, or the \`--out\` path
   you were given) matching this schema exactly:
   \`\`\`json
   {
     "agnosgram_advise": 1,
     "plan": "${planPath}",
     "generated": "YYYY-MM-DD",
     "checked_ids": ["LES-001", "..."],
     "contradictions": [{
       "record_id": "LES-002",
       "kind": "empirical | normative",
       "severity": "blocker | caution",
       "plan_excerpt": "...",
       "record_excerpt": "...",
       "confidence": "high",
       "last_verified": "YYYY-MM-DD",
       "explanation": "one sentence"
     }],
     "clear": false
   }
   \`\`\`
2. \`checked_ids\` lists every record id from the digest above that you actually
   considered - check all of them, not a sample.
3. For each contradiction, \`confidence\` and \`last_verified\` must copy the
   cited record's actual frontmatter exactly (this is checked mechanically).
4. \`plan_excerpt\` must be a real substring of the plan file; \`record_excerpt\`
   must be a real substring of the cited record's body. Do not paraphrase them.
5. \`clear\` is \`true\` only when \`contradictions\` is empty or contains no
   \`"severity": "blocker"\` entries.
6. Keep \`explanation\` to one tight sentence per contradiction.

## Validate your result (mechanical, no LLM)
Run this and fix anything it reports before finishing:
\`agnosgram advise --validate ${outPath}\`
`;
}

interface Contradiction {
  record_id?: unknown;
  kind?: unknown;
  severity?: unknown;
  plan_excerpt?: unknown;
  record_excerpt?: unknown;
  confidence?: unknown;
  last_verified?: unknown;
  explanation?: unknown;
}

interface AdviseReport {
  agnosgram_advise?: unknown;
  plan?: unknown;
  generated?: unknown;
  checked_ids?: unknown;
  contradictions?: unknown;
  clear?: unknown;
}

interface ValidateIssue {
  level: "error" | "warn";
  code: string;
  message: string;
}

interface ValidateResult {
  file: string;
  ok: boolean;
  errors: number;
  warnings: number;
  issues: ValidateIssue[];
  report: unknown;
  /**
   * Mechanically-derived clearness: true only when no blocker-severity
   * contradiction was validated AND the report's own `clear` field says
   * `true`. This is the trustworthy signal for `--strict` - never the
   * report's self-declared `clear` alone, since a report can lie about it.
   */
  clear: boolean;
}

/** Resolve a plan path the same way a person running the CLI would find it:
 * cwd-relative first (mirrors how `reportPath` below resolves via bare
 * `existsSync`/`readFileSync`, and honors absolute paths via `resolve`),
 * then falling back to root-relative so `--validate` still works from a
 * subdirectory of the project. */
function resolvePlanFile(root: string, planField: string): string | undefined {
  const cwdRelative = resolve(planField);
  if (existsSync(cwdRelative)) return cwdRelative;
  const rootRelative = join(root, planField);
  if (existsSync(rootRelative)) return rootRelative;
  return undefined;
}

/** `typeof v === "string" ? v : undefined`, spelled once. */
function asString(v: unknown): string | undefined {
  return typeof v === "string" ? v : undefined;
}

function validateReport(root: string, reportPath: string): ValidateResult {
  const issues: ValidateIssue[] = [];
  const err = (code: string, message: string) => issues.push({ level: "error", code, message });
  const warnIssue = (code: string, message: string) => issues.push({ level: "warn", code, message });

  if (!existsSync(reportPath)) {
    throw new UserError(`No such file: ${reportPath}`);
  }

  let report: AdviseReport;
  try {
    report = JSON.parse(readFileSync(reportPath, "utf8"));
  } catch (e) {
    err("json.parse", `report is not valid JSON: ${e instanceof Error ? e.message : String(e)}`);
    return { file: reportPath, ok: false, errors: 1, warnings: 0, issues, report: null, clear: false };
  }

  if (report === null || typeof report !== "object" || Array.isArray(report)) {
    err("schema.shape", "report must be a JSON object");
    return { file: reportPath, ok: false, errors: 1, warnings: 0, issues, report, clear: false };
  }

  if (report.agnosgram_advise !== ADVISE_SCHEMA_VERSION) {
    err(
      "schema.version",
      `"agnosgram_advise" must be ${ADVISE_SCHEMA_VERSION}, got ${JSON.stringify(report.agnosgram_advise)}`,
    );
  }

  const planField = asString(report.plan);
  if (planField === undefined) err("schema.plan", 'missing or non-string "plan"');

  const generated = asString(report.generated);
  if (generated === undefined || !isValidIsoDate(generated)) {
    err("schema.generated", '"generated" must be a YYYY-MM-DD date');
  }

  const checkedIds: string[] = Array.isArray(report.checked_ids)
    ? report.checked_ids.filter((v): v is string => typeof v === "string")
    : [];
  if (!Array.isArray(report.checked_ids)) {
    err("schema.checked_ids", '"checked_ids" must be an array of record ids');
  } else if (report.checked_ids.some((v) => typeof v !== "string")) {
    err("schema.checked_ids", '"checked_ids" must contain only strings; found a non-string entry');
  }

  const records = loadRecords(root);
  const byId = new Map(records.map((r) => [r.frontmatter.id, r]));

  for (const id of checkedIds) {
    if (!byId.has(id)) {
      err("provenance.checked_id.unknown", `checked_ids references "${id}", which no record defines`);
    }
  }

  const contradictions: Contradiction[] = Array.isArray(report.contradictions)
    ? (report.contradictions as Contradiction[])
    : [];
  if (!Array.isArray(report.contradictions)) {
    err("schema.contradictions", '"contradictions" must be an array');
  }

  // Plan file, for excerpt substring checks (best-effort: excerpt mismatches
  // are warnings, never errors, since the plan may have moved or the CLI may
  // be run from a different cwd than the report's provenance).
  let planText: string | undefined;
  if (planField !== undefined) {
    const resolvedPlan = resolvePlanFile(root, planField);
    if (resolvedPlan) {
      planText = readFileSync(resolvedPlan, "utf8");
    } else {
      warnIssue("coverage.plan_missing", `plan file "${planField}" was not found on disk; excerpt checks skipped`);
    }
  }

  let blockerSeen = false;
  for (const [i, c] of contradictions.entries()) {
    const where = `contradictions[${i}]`;
    const recordId = asString(c.record_id);
    if (recordId === undefined) {
      err(`schema.${where}.record_id`, `${where}: missing or non-string "record_id"`);
    }
    const record = recordId !== undefined ? byId.get(recordId) : undefined;
    if (recordId !== undefined && !record) {
      err("provenance.record_id.unknown", `${where}: record_id "${recordId}" does not exist in the store`);
    }
    if (recordId !== undefined && record && !checkedIds.includes(recordId)) {
      warnIssue("coverage.uncovered", `${where}: record_id "${recordId}" is cited but not listed in checked_ids`);
    }

    if (!(KNOWN_KINDS as readonly string[]).includes(c.kind as string)) {
      err(`schema.${where}.kind`, `${where}: "kind" must be one of ${KNOWN_KINDS.join(", ")}`);
    }
    const severity = c.severity as string;
    if (!(KNOWN_SEVERITIES as readonly string[]).includes(severity)) {
      err(`schema.${where}.severity`, `${where}: "severity" must be one of ${KNOWN_SEVERITIES.join(", ")}`);
    } else if (severity === "blocker") {
      blockerSeen = true;
    }

    const planExcerpt = asString(c.plan_excerpt);
    if (!planExcerpt) {
      err(`schema.${where}.plan_excerpt`, `${where}: missing or non-string "plan_excerpt"`);
    } else if (planText !== undefined && !planText.includes(planExcerpt)) {
      warnIssue("excerpt.plan_mismatch", `${where}: plan_excerpt is not a substring of "${planField}"`);
    }

    const recordExcerpt = asString(c.record_excerpt);
    if (!recordExcerpt) {
      err(`schema.${where}.record_excerpt`, `${where}: missing or non-string "record_excerpt"`);
    } else if (record && !record.body.includes(recordExcerpt)) {
      warnIssue(
        "excerpt.record_mismatch",
        `${where}: record_excerpt is not a substring of ${recordId}'s body`,
      );
    }

    const confidence = asString(c.confidence);
    if (!confidence || !(KNOWN_CONFIDENCE as readonly string[]).includes(confidence)) {
      err(`schema.${where}.confidence`, `${where}: "confidence" must be one of ${KNOWN_CONFIDENCE.join(", ")}`);
    } else if (record && confidence !== record.frontmatter.confidence) {
      err(
        "provenance.confidence.mismatch",
        `${where}: confidence "${confidence}" does not match ${recordId}'s actual confidence "${record.frontmatter.confidence}"`,
      );
    }

    const lastVerified = asString(c.last_verified);
    if (!lastVerified || !isValidIsoDate(lastVerified)) {
      err(`schema.${where}.last_verified`, `${where}: "last_verified" must be a YYYY-MM-DD date`);
    } else if (record && lastVerified !== record.frontmatter.last_verified) {
      err(
        "provenance.last_verified.mismatch",
        `${where}: last_verified "${lastVerified}" does not match ${recordId}'s actual last_verified "${record.frontmatter.last_verified}"`,
      );
    }

    if (typeof c.explanation !== "string" || c.explanation.trim() === "") {
      err(`schema.${where}.explanation`, `${where}: missing or empty "explanation"`);
    }
  }

  const declaredClear = typeof report.clear === "boolean" ? report.clear : undefined;
  if (declaredClear === undefined) {
    err("schema.clear", '"clear" must be a boolean');
  } else {
    // A blocker alongside a self-declared `clear: true` is not just
    // inconsistent, it is the exact shape a Gate consumer trusting `clear`
    // would be fooled by - promote it to an error so `--validate` (without
    // even `--strict`) already catches it.
    if (declaredClear === true && blockerSeen) {
      err("consistency.clear", '"clear" is true but a blocker contradiction is present');
    }
    if (declaredClear === false && contradictions.length === 0) {
      warnIssue("consistency.clear", '"clear" is false but no contradictions were reported');
    }
  }

  // Coverage: did the report actually look at (close to) everything on offer?
  const storeIds = new Set(records.map((r) => r.frontmatter.id));
  const uncheckedCount = [...storeIds].filter((id) => !checkedIds.includes(id)).length;
  if (checkedIds.length === 0 && storeIds.size > 0) {
    warnIssue("coverage.empty", "no records were checked; the report examined nothing");
  } else if (uncheckedCount > 0) {
    warnIssue(
      "coverage.partial",
      `${uncheckedCount} of ${storeIds.size} store record(s) were not listed in checked_ids`,
    );
  }

  const errors = issues.filter((i) => i.level === "error").length;
  const warnings = issues.filter((i) => i.level === "warn").length;
  // Mechanical clearness: never trust the report's self-declared `clear`
  // alone - a blocker-severity contradiction always means "not clear",
  // regardless of what the field says (that inconsistency is already an
  // error above, but `--strict` must not depend on catching it).
  const clear = declaredClear === true && !blockerSeen;
  return { file: reportPath, ok: errors === 0, errors, warnings, issues, report, clear };
}

export function runAdvise(argv: string[]): void {
  const { values, positionals } = parseArgs({
    args: argv,
    allowPositionals: true,
    options: {
      validate: { type: "boolean", default: false },
      out: { type: "string" },
      strict: { type: "boolean", default: false },
      json: { type: "boolean", default: false },
      format: { type: "string" },
    },
  });

  const format = resolveFormat(values);
  const root = findProjectRoot();
  if (!hasStore(root)) {
    throw new UserError("No .agnosgram/ store found. Run `agnosgram init` first.");
  }

  if (values.validate) {
    const reportArg = positionals[0] ?? values.out;
    if (!reportArg) {
      throw new UserError("Usage: agnosgram advise --validate <report-file>");
    }
    const result = validateReport(root, reportArg);

    if (format !== "human") {
      printStructured(result, format);
    } else if (result.issues.length === 0) {
      info(`${result.file}: valid.`);
    } else {
      for (const i of result.issues) {
        info(`  ${i.level === "error" ? "error" : "warn "} ${i.code}  ${i.message}`);
      }
      info(`\n${result.file}: ${result.errors} error(s), ${result.warnings} warning(s).`);
    }

    if (result.errors > 0 || (values.strict && !result.clear)) {
      process.exitCode = 1;
    }
    return;
  }

  const planPath = positionals[0];
  if (!planPath || planPath.trim() === "") {
    throw new UserError("Usage: agnosgram advise <plan-path> [--out <report-file>]");
  }
  const outPath = values.out?.trim() || `${planPath}.advise.json`;
  const prompt = buildPrompt(root, planPath.trim(), outPath);

  if (format !== "human") {
    printStructured({ prompt }, format);
  } else {
    process.stdout.write(prompt);
  }
}
