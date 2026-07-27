/**
 * `agnosgram pack` - a token-budgeted context bundle for the start of an agent
 * session: status + lessons (+ decisions when scoped), most important first,
 * dropped whole-record when the budget runs out. This is the agent hot path,
 * so the default output is the human-readable Markdown bundle itself, not JSON.
 *
 * Priority order (also the greedy-drop order): header, `state/status.md`
 * verbatim (always kept - never dropped), lessons (pitfalls then conventions,
 * each sorted confidence desc / last_verified desc / id asc), decisions (only
 * included when `--scope` is given). Context files are never packed - they are
 * meant to be read directly by an agent working in that area, not bundled into
 * every session start.
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { loadConfig } from "../core/config.js";
import { info, printStructured, UserError } from "../core/output.js";
import { findProjectRoot, hasStore, memoryDir } from "../core/paths.js";
import {
  compareRecords,
  loadRecords,
  renderRecordBlock,
  toFlatRecord,
  type StoreRecord,
} from "../core/records.js";
import { resolveFormat } from "../core/serialize.js";
import { estimateTokens } from "../core/tokens.js";

/** Output-schema version for `pack --json` (independent of the store format version). */
const PACK_SCHEMA_VERSION = 1;
const DEFAULT_PACK_BUDGET = 2000;

function matchesScope(rec: StoreRecord, scope: string): boolean {
  const lower = scope.toLowerCase();
  return rec.frontmatter.scope.some((s) => s.toLowerCase() === lower);
}

interface PackResult {
  budget: number;
  tokens: number;
  status: string;
  markdown: string;
  included: StoreRecord[];
  omitted: StoreRecord[];
}

function buildPack(root: string, budget: number, scope?: string): PackResult {
  const statusPath = join(memoryDir(root), "state", "status.md");
  const status = readFileSync(statusPath, "utf8").trim();

  const records = loadRecords(root);
  const scoped = (recs: StoreRecord[]) => (scope ? recs.filter((r) => matchesScope(r, scope)) : recs);

  const pitfalls = scoped(records.filter((r) => r.frontmatter.type === "pitfall")).sort(compareRecords);
  const conventions = scoped(records.filter((r) => r.frontmatter.type === "convention")).sort(
    compareRecords,
  );
  // Decisions are only ever candidates when the pack is scoped - an unscoped
  // pack stays lean for every session; decisions are architectural detail an
  // agent needs only when it is about to work in that scoped area.
  const decisions = scope
    ? scoped(records.filter((r) => r.frontmatter.type === "decision")).sort(compareRecords)
    : [];

  const candidates = [...pitfalls, ...conventions, ...decisions];

  const header = `# Agnosgram pack${scope ? ` (scope: ${scope})` : ""}\n`;
  const statusBlock = `## state/status.md\n\n${status}\n`;

  let used = estimateTokens(header) + estimateTokens(statusBlock);
  const included: StoreRecord[] = [];
  const omitted: StoreRecord[] = [];
  for (const rec of candidates) {
    const cost = estimateTokens(renderRecordBlock(rec));
    if (used + cost <= budget) {
      included.push(rec);
      used += cost;
    } else {
      omitted.push(rec);
    }
  }

  const sections: string[] = [header, statusBlock];
  let currentType = "";
  const typeHeading: Record<string, string> = {
    pitfall: "## Pitfalls",
    convention: "## Conventions",
    decision: "## Decisions",
  };
  for (const rec of included) {
    if (rec.frontmatter.type !== currentType) {
      currentType = rec.frontmatter.type;
      sections.push(typeHeading[currentType] ?? `## ${currentType}`);
    }
    sections.push(renderRecordBlock(rec));
  }
  if (omitted.length > 0) {
    sections.push(
      `## Omitted (budget)\n\n${omitted.map((r) => `- ${r.frontmatter.id} (${r.frontmatter.type})`).join("\n")}`,
    );
  }

  const markdown = sections.join("\n\n").trimEnd() + "\n";
  return { budget, tokens: estimateTokens(markdown), status, markdown, included, omitted };
}

export function runPack(argv: string[]): void {
  const { values } = parseArgs({
    args: argv,
    allowPositionals: false,
    options: {
      scope: { type: "string" },
      budget: { type: "string" },
      json: { type: "boolean", default: false },
      format: { type: "string" },
    },
  });

  const format = resolveFormat(values);

  const root = findProjectRoot();
  if (!hasStore(root)) {
    throw new UserError("No .agnosgram/ store found. Run `agnosgram init` first.");
  }

  let budget = DEFAULT_PACK_BUDGET;
  const config = loadConfig(root);
  if (config.pack_budget !== undefined) budget = config.pack_budget;
  if (values.budget !== undefined) {
    const parsed = Number.parseInt(values.budget, 10);
    if (!Number.isFinite(parsed) || parsed <= 0) {
      throw new UserError(`--budget must be a positive integer, got "${values.budget}"`);
    }
    budget = parsed;
  }

  const result = buildPack(root, budget, values.scope?.trim() || undefined);

  if (format === "human") {
    process.stdout.write(result.markdown);
  } else {
    printStructured(
      {
        version: PACK_SCHEMA_VERSION,
        budget: result.budget,
        tokens: result.tokens,
        status: result.status,
        records: result.included.map(toFlatRecord),
        omitted: result.omitted.map((r) => r.frontmatter.id),
      },
      format,
    );
  }
}
