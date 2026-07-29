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
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { loadConfig } from "../core/config.js";
import { info, printStructured, UserError } from "../core/output.js";
import { findProjectRoot, hasStore, memoryDir } from "../core/paths.js";
import {
  compareRecords,
  loadRecords,
  matchesScope,
  renderRecordBlock,
  toFlatRecord,
  type StoreRecord,
} from "../core/records.js";
import { resolveFormat } from "../core/serialize.js";
import { estimateTokens } from "../core/tokens.js";

/** Output-schema version for `pack --json` (independent of the store format version). */
const PACK_SCHEMA_VERSION = 1;
/** Default token budget `pack` targets when neither `--budget` nor config's `pack_budget` is given. */
export const DEFAULT_PACK_BUDGET = 2000;

/** At most this many omitted records are named in the "Omitted (budget)"
 * footer; the rest are summarized as "...and N more" so the footer itself
 * cannot grow without bound on a large store. */
const FOOTER_SHOW = 5;

const TYPE_HEADING: Record<string, string> = {
  pitfall: "## Pitfalls",
  convention: "## Conventions",
  decision: "## Decisions",
};

/** Records of one type, scope-filtered (when `scope` is given) and sorted by priority. */
function byType(records: StoreRecord[], type: string, scope?: string): StoreRecord[] {
  const pool = records.filter((r) => r.frontmatter.type === type);
  const scoped = scope ? pool.filter((r) => matchesScope(r, scope)) : pool;
  return scoped.sort(compareRecords);
}

function renderOmittedFooter(omitted: StoreRecord[]): string {
  if (omitted.length === 0) return "";
  const shown = omitted.slice(0, FOOTER_SHOW).map((r) => `- ${r.frontmatter.id} (${r.frontmatter.type})`);
  const rest = omitted.length - Math.min(omitted.length, FOOTER_SHOW);
  if (rest > 0) shown.push(`- ...and ${rest} more`);
  return `## Omitted (budget)\n\n${shown.join("\n")}`;
}

/**
 * Worst-case token cost of the omitted footer over any subset of `candidates`.
 * The footer is capped (`FOOTER_SHOW` entries + an "...and N more" tail), but
 * its exact size still depends on which records end up omitted - which is
 * what the greedy loop below is deciding. Reserving this upper bound up
 * front (built from the single heaviest "id (type)" line, repeated, with the
 * full candidate count for the tail) means the loop never has to overshoot
 * to make room for the footer later: any real omitted subset is
 * shorter-or-equal in both id/type text and count, and `estimateTokens` is
 * monotonic in chars and words, so its real cost never exceeds this reserve.
 */
function maxFooterReserve(candidates: StoreRecord[]): number {
  if (candidates.length === 0) return 0;
  const heaviest = candidates.reduce((a, b) =>
    a.frontmatter.id.length + a.frontmatter.type.length >= b.frontmatter.id.length + b.frontmatter.type.length
      ? a
      : b,
  );
  const worstCase = new Array<StoreRecord>(candidates.length).fill(heaviest);
  return estimateTokens(renderOmittedFooter(worstCase));
}

interface SimResult {
  included: StoreRecord[];
  omitted: StoreRecord[];
  sections: string[];
}

/**
 * Greedily admit candidates in priority order, simulating the *actual*
 * section assembly (type headings inserted on change, "\n\n" joiners) so the
 * budget check reflects what will really be rendered - not just the sum of
 * bare record bodies. `footerReserve` is subtracted from the ceiling so
 * there is always room left for the omitted-footer this loop's own output
 * may require.
 */
function simulatePack(
  candidates: StoreRecord[],
  blockText: Map<StoreRecord, string>,
  header: string,
  statusBlock: string,
  budget: number,
  footerReserve: number,
): SimResult {
  const included: StoreRecord[] = [];
  const omitted: StoreRecord[] = [];
  const sections: string[] = [header, statusBlock];
  let currentType = "";

  for (const rec of candidates) {
    const trial = sections.slice();
    if (rec.frontmatter.type !== currentType) {
      trial.push(TYPE_HEADING[rec.frontmatter.type] ?? `## ${rec.frontmatter.type}`);
    }
    trial.push(blockText.get(rec)!);
    const cost = estimateTokens(trial.join("\n\n"));
    if (cost + footerReserve <= budget) {
      sections.length = 0;
      sections.push(...trial);
      currentType = rec.frontmatter.type;
      included.push(rec);
    } else {
      omitted.push(rec);
    }
  }

  return { included, omitted, sections };
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
  if (!existsSync(statusPath)) {
    throw new UserError(
      "Missing .agnosgram/state/status.md. Run `agnosgram doctor` to see what else is missing, " +
        "or `agnosgram init` to scaffold a fresh store.",
    );
  }
  const status = readFileSync(statusPath, "utf8").trim();

  const records = loadRecords(root);
  const pitfalls = byType(records, "pitfall", scope);
  const conventions = byType(records, "convention", scope);
  // Decisions are only ever candidates when the pack is scoped - an unscoped
  // pack stays lean for every session; decisions are architectural detail an
  // agent needs only when it is about to work in that scoped area.
  const decisions = scope ? byType(records, "decision", scope) : [];

  const candidates = [...pitfalls, ...conventions, ...decisions];

  const header = `# Agnosgram pack${scope ? ` (scope: ${scope})` : ""}\n`;
  const statusBlock = `## state/status.md\n\n${status}\n`;

  // Render each record block exactly once; the cached string funds both the
  // budget simulation and the final assembly below.
  const blockText = new Map<StoreRecord, string>();
  for (const rec of candidates) blockText.set(rec, renderRecordBlock(rec));

  // First pass: no footer reserve. If everything fits, there is no footer to
  // account for, so this is also the final answer - keeps the common case
  // (a store that fits within budget) from losing headroom to a footer it
  // will never render.
  let sim = simulatePack(candidates, blockText, header, statusBlock, budget, 0);
  if (sim.omitted.length > 0) {
    const reserve = maxFooterReserve(candidates);
    sim = simulatePack(candidates, blockText, header, statusBlock, budget, reserve);
  }

  const { included, omitted, sections } = sim;
  if (omitted.length > 0) {
    sections.push(renderOmittedFooter(omitted));
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
