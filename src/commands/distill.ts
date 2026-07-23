import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync } from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { loadConfig, type AgnosgramConfig } from "../core/config.js";
import { extractRecords, KNOWN_CONFIDENCE, KNOWN_TYPES, validateRecord } from "../core/frontmatter.js";
import { info, printJson, UserError } from "../core/output.js";
import { findProjectRoot, hasStore, memoryDir } from "../core/paths.js";
import { readStore } from "../core/store.js";
import { estimateTokens } from "../core/tokens.js";

/** Journal months present in the store (`journal/YYYY-MM.md`), oldest first. */
function journalMonths(root: string): string[] {
  const dir = join(memoryDir(root), "journal");
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((f) => /^\d{4}-\d{2}\.md$/.test(f))
    .map((f) => f.slice(0, 7))
    .sort();
}

/** Every record id already in the store, so the distiller allocates fresh ones. */
function existingIds(root: string, exclude?: string): string[] {
  const ids: string[] = [];
  for (const file of readStore(root)) {
    if (!file.recordBearing || file.rel === exclude) continue;
    for (const raw of extractRecords(file.text)) {
      if (typeof raw.data.id === "string") ids.push(raw.data.id);
    }
  }
  return ids.sort();
}

function buildPrompt(root: string, config: AgnosgramConfig): string {
  const months = journalMonths(root);
  const ids = existingIds(root);
  const budgetLines = Object.entries(config.budgets)
    .map(([f, b]) => `  - ${f}: ${b} tokens`)
    .join("\n");

  return `# Agnosgram distillation task

You are curating this project's memory store at \`.agnosgram/\`. Capture is cheap
(the append-only journal); distillation is deliberate. Turn raw journal entries
into a small set of durable, non-overlapping lessons and decisions - and shrink
what is already there. Do NOT invent facts; only distill what the sources support.

## Sources to read
- Journal months: ${months.length ? months.map((m) => `journal/${m}.md`).join(", ") : "(none yet)"}
  \`Learned\`/\`Avoid\` lines are pitfall candidates; \`Decided\` lines are decision candidates.
- Existing curated memory: lessons/pitfalls.md, lessons/conventions.md, decisions/*.md.

## Output rules
1. Write into: lessons/pitfalls.md (type: pitfall), lessons/conventions.md
   (type: convention), decisions/NNNN-slug.md (type: decision).
2. Every record carries this frontmatter, fenced by \`---\` lines:
   \`\`\`
   ---
   id: <PREFIX-NNN>          # LES-*/CON-* for lessons, DEC-* for decisions; unique
   type: <${KNOWN_TYPES.join(" | ")}>
   scope: [area, ...]        # non-empty list
   confidence: <${KNOWN_CONFIDENCE.join(" | ")}>
   created: YYYY-MM-DD
   last_verified: YYYY-MM-DD
   source: journal/YYYY-MM.md
   supersedes: [OLD-ID, ...] # optional; REQUIRED when you merge/replace records
   ---
   <one tight paragraph of body>
   \`\`\`
3. **Merge, do not append.** If a new insight overlaps an existing record, rewrite
   the existing one and list the ids it replaces under \`supersedes:\`. Never leave
   two near-duplicate records side by side.
4. Do not reuse an existing id. Ids already taken: ${ids.length ? ids.join(", ") : "(none)"}.
5. Stay within per-file token budgets:
${budgetLines}
6. The human reviews this in a PR. Keep bodies terse and factual.

## Validate your result (mechanical, no LLM)
Run these and fix anything they report before finishing:
- \`agnosgram distill --validate lessons/pitfalls.md\` (repeat per file you touched)
- \`agnosgram doctor --strict\`

## After the human accepts the distilled records
Archive the journal months you fully absorbed so they stop counting against budgets
and re-distillation: \`agnosgram distill --archive <YYYY-MM>\`.
`;
}

interface ValidateResult {
  file: string;
  ok: boolean;
  errors: number;
  warnings: number;
  issues: Array<{ level: string; code: string; line?: number; message: string }>;
}

function validateFile(root: string, relArg: string): ValidateResult {
  const storeRel = relArg.replace(/^\.agnosgram\//, "");
  const abs = join(memoryDir(root), storeRel);
  if (!existsSync(abs)) {
    throw new UserError(`No such file: .agnosgram/${storeRel}`);
  }
  const text = readFileSync(abs, "utf8");
  const issues: ValidateResult["issues"] = [];

  const records = extractRecords(text);
  const seen = new Map<string, number>();
  const otherIds = new Set(existingIds(root, join(".agnosgram", storeRel)));

  for (const raw of records) {
    const { issues: recIssues } = validateRecord(raw);
    for (const i of recIssues) {
      issues.push({ level: i.level, code: `schema.${i.code}`, line: i.line, message: i.message });
    }
    const id = typeof raw.data.id === "string" ? raw.data.id : undefined;
    if (id) {
      if (seen.has(id)) {
        issues.push({ level: "error", code: "id.duplicate", line: raw.line, message: `id "${id}" repeats in this file (line ${seen.get(id)})` });
      } else {
        seen.set(id, raw.line);
      }
      if (otherIds.has(id)) {
        issues.push({ level: "error", code: "id.collision", line: raw.line, message: `id "${id}" already exists elsewhere in the store` });
      }
    }
  }

  // Budget check when this file has a configured budget.
  const config = loadConfig(root);
  const budget = config.budgets[storeRel];
  if (budget !== undefined) {
    const tokens = estimateTokens(text);
    if (tokens > budget) {
      issues.push({ level: "error", code: "budget.over", message: `~${tokens} tokens over the ${budget}-token budget` });
    }
  }

  const errors = issues.filter((i) => i.level === "error").length;
  const warnings = issues.filter((i) => i.level === "warn").length;
  return { file: join(".agnosgram", storeRel), ok: errors === 0, errors, warnings, issues };
}

function archiveMonth(root: string, month: string): string {
  if (!/^\d{4}-\d{2}$/.test(month)) {
    throw new UserError(`--archive expects a YYYY-MM month, got "${month}".`);
  }
  const src = join(memoryDir(root), "journal", `${month}.md`);
  if (!existsSync(src)) {
    throw new UserError(`No journal for ${month} (looked for .agnosgram/journal/${month}.md).`);
  }
  const archiveDir = join(memoryDir(root), "journal", "archive");
  mkdirSync(archiveDir, { recursive: true });
  const dest = join(archiveDir, `${month}.md`);
  if (existsSync(dest)) {
    throw new UserError(`Already archived: .agnosgram/journal/archive/${month}.md exists.`);
  }
  renameSync(src, dest);
  return join(".agnosgram", "journal", "archive", `${month}.md`);
}

export function runDistill(argv: string[]): void {
  const { values } = parseArgs({
    args: argv,
    allowPositionals: false,
    options: {
      validate: { type: "string" },
      archive: { type: "string" },
      json: { type: "boolean", default: false },
    },
  });

  const root = findProjectRoot();
  if (!hasStore(root)) {
    throw new UserError("No .agnosgram/ store found. Run `agnosgram init` first.");
  }

  if (values.archive !== undefined) {
    const dest = archiveMonth(root, values.archive.trim());
    if (values.json) printJson({ archived: dest });
    else info(`Archived to ${dest}`);
    return;
  }

  if (values.validate !== undefined) {
    const result = validateFile(root, values.validate.trim());
    if (values.json) {
      printJson(result);
    } else if (result.issues.length === 0) {
      info(`${result.file}: valid.`);
    } else {
      for (const i of result.issues) {
        const where = i.line ? `:${i.line}` : "";
        info(`  ${i.level === "error" ? "error" : "warn "} ${i.code}${where}  ${i.message}`);
      }
      info(`\n${result.file}: ${result.errors} error(s), ${result.warnings} warning(s).`);
    }
    if (result.errors > 0) process.exitCode = 1;
    return;
  }

  const prompt = buildPrompt(root, loadConfig(root));
  if (values.json) printJson({ prompt });
  else process.stdout.write(prompt);
}
