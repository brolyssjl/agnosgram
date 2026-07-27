#!/usr/bin/env node
/**
 * Tier-2 end-to-end token evals. Tier-1 (bench.mjs) measures serialized
 * payload shapes in isolation; this measures what an agent actually receives
 * from the built CLI: spawn `dist/cli.js` against two fixture stores
 * (`bench/fixtures/store-small`, ~6 records; `bench/fixtures/store-large`,
 * ~40 records) across a task x format matrix and count real tokens
 * (gpt-tokenizer, same o200k proxy as Tier-1).
 *
 * Requires `npm run build` first (spawns the compiled CLI).
 *
 * Usage:
 *   node bench/tier2.mjs            # print a table
 *   node bench/tier2.mjs --json     # machine-readable, for CI gating
 *   node bench/tier2.mjs --check    # exit non-zero if a regression threshold is crossed
 */
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { encode } from "gpt-tokenizer/model/gpt-4o";

const HERE = dirname(fileURLToPath(import.meta.url));
const CLI = join(HERE, "..", "dist", "cli.js");

const STORES = {
  "store-small": join(HERE, "fixtures", "store-small"),
  "store-large": join(HERE, "fixtures", "store-large"),
};

/** Matches the CLI's default pack budget (`src/commands/pack.ts`); fixtures set no `pack_budget` override. */
const DEFAULT_PACK_BUDGET = 2000;

const TASKS = {
  "session-start-pack": ["pack"],
  "scoped-pack": ["pack", "--scope", "core"],
  show: ["show", "core"],
  "advise-prep": ["advise", "plan.md"],
};

const FORMATS = ["human", "json", "toon"];

function run(cwd, args, format) {
  const argv = format === "human" ? args : [...args, "--format", format];
  const res = spawnSync(process.execPath, [CLI, ...argv], { cwd, encoding: "utf8" });
  // Commands may legitimately exit 0 or 1 (e.g. `show` with no match); only a
  // crash (anything else, or a thrown exception) should fail the bench run.
  if (res.error || (res.status !== 0 && res.status !== 1)) {
    throw new Error(
      `spawn failed for [${argv.join(" ")}] in ${cwd} (status ${res.status}): ${res.stderr || res.error}`,
    );
  }
  return res.stdout;
}

function count(text) {
  return encode(text).length;
}

function round1(n) {
  return Number(n.toFixed(1));
}

const rows = [];
for (const [store, storeRoot] of Object.entries(STORES)) {
  for (const [task, args] of Object.entries(TASKS)) {
    for (const format of FORMATS) {
      const out = run(storeRoot, args, format);
      rows.push({ store, task, format, tokens: count(out) });
    }
  }
}

function find(store, task, format) {
  return rows.find((r) => r.store === store && r.task === task && r.format === format);
}

const args = new Set(process.argv.slice(2));

if (args.has("--json")) {
  process.stdout.write(JSON.stringify(rows, null, 2) + "\n");
} else {
  process.stdout.write("store        task                  format  tokens\n");
  process.stdout.write("--------------------------------------------------\n");
  for (const r of rows) {
    process.stdout.write(
      `${r.store.padEnd(12)} ${r.task.padEnd(21)} ${r.format.padEnd(7)} ${r.tokens}\n`,
    );
  }
}

if (args.has("--check")) {
  const problems = [];

  // Gate 1: TOON must meaningfully save tokens vs compact JSON for the
  // record-array-shaped tasks (pack, show) on both fixture stores. Excludes
  // advise-prep, which is a prose prompt wrapped in a single JSON/TOON string
  // field - format cannot help there, by design.
  const TABULAR_TASKS = ["session-start-pack", "scoped-pack", "show"];
  for (const store of Object.keys(STORES)) {
    for (const task of TABULAR_TASKS) {
      const j = find(store, task, "json");
      const t = find(store, task, "toon");
      if (!j || !t) {
        problems.push(`missing rows for ${store}/${task}`);
        continue;
      }
      const deltaPct = round1(((t.tokens - j.tokens) / j.tokens) * 100);
      if (deltaPct > -10) {
        problems.push(
          `expected TOON to save >=10% vs JSON on ${store}/${task}, got ${deltaPct}%`,
        );
      }
    }
  }

  // Gate 2: pack's whole-record greedy budget must actually cap growth - a
  // ~40-record store should not balloon pack's real token count without
  // bound. Checked against the `human` (Markdown) format, since that is the
  // artifact whole-record dropping is actually budgeted against - `--format
  // json` is a structurally different, more verbose representation of the
  // same records and was never the budgeted shape.
  //
  // The tolerance is generous (not ~1.05x) because `pack`'s budget is
  // enforced against `estimateTokens` (src/core/tokens.ts), a dependency-free
  // heuristic that is a proxy, not a real tokenizer (DEC-0001). Measured here:
  // it under-counts this Markdown-heavy content by roughly 35-40% relative to
  // real GPT tokenization (a ~40-record store's unscoped pack measured ~1874
  // estimated tokens internally against a 2000 budget - nothing got dropped -
  // but cost ~2579 real tokens). That gap is expected given the estimator's
  // documented trade-off, not a bug this eval fixes, but a real budget-vs-
  // real-tokens delta worth catching if it grows much further.
  const BUDGET_TOLERANCE = 1.4;
  for (const store of Object.keys(STORES)) {
    for (const task of ["session-start-pack", "scoped-pack"]) {
      const h = find(store, task, "human");
      if (!h) {
        problems.push(`missing row for ${store}/${task}`);
        continue;
      }
      const ceiling = DEFAULT_PACK_BUDGET * BUDGET_TOLERANCE;
      if (h.tokens > ceiling) {
        problems.push(
          `expected ${store}/${task} to stay under ~${Math.round(ceiling)} real tokens (budget ${DEFAULT_PACK_BUDGET} x ${BUDGET_TOLERANCE}), got ${h.tokens}`,
        );
      }
    }
  }

  if (problems.length > 0) {
    process.stderr.write("tier2 regression:\n- " + problems.join("\n- ") + "\n");
    process.exit(1);
  }
}
