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
import { DEFAULT_PACK_BUDGET } from "../dist/commands/pack.js";

const HERE = dirname(fileURLToPath(import.meta.url));
const CLI = join(HERE, "..", "dist", "cli.js");

const STORES = {
  "store-small": join(HERE, "fixtures", "store-small"),
  "store-large": join(HERE, "fixtures", "store-large"),
};

const TASKS = {
  "session-start-pack": ["pack"],
  "scoped-pack": ["pack", "--scope", "core"],
  show: ["show", "core"],
  "advise-prep": ["advise", "plan.md"],
};

const FORMATS = ["human", "json", "toon"];

// None of the tasks above are expected to exit 1 against these fixtures
// (`show core` matches in both) - every measured row must be a clean
// success with real output. A crash that happens to exit 1 (a thrown
// UserError, say) must fail the bench run, not silently produce an
// empty/0-token row that later divides its way into a vacuous NaN "pass".
function run(cwd, args, format) {
  const argv = format === "human" ? args : [...args, "--format", format];
  const res = spawnSync(process.execPath, [CLI, ...argv], { cwd, encoding: "utf8" });
  if (res.error) {
    throw new Error(`spawn failed for [${argv.join(" ")}] in ${cwd}: ${res.error}`);
  }
  if (res.status !== 0 && res.status !== 1) {
    throw new Error(
      `CLI exited ${res.status} (crash) for [${argv.join(" ")}] in ${cwd}: ${res.stderr || "(no stderr)"}`,
    );
  }
  if (res.status === 1) {
    throw new Error(
      `CLI exited 1 for [${argv.join(" ")}] in ${cwd}, but none of tier2's tasks are expected to fail ` +
        `against these fixtures: ${res.stderr || "(no stderr)"}`,
    );
  }
  if (!res.stdout || res.stdout.trim() === "") {
    throw new Error(
      `CLI produced empty stdout for [${argv.join(" ")}] in ${cwd} (status ${res.status}) - ` +
        `an empty response from a task that isn't expected to fail means it crashed silently, not that it "measured 0 tokens".`,
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
// Raw stdout per row, keyed by "store|task|format" - used internally by the
// gates below (e.g. Gate 1 needs to re-serialize the JSON row compactly);
// kept out of the public `rows` shape so the table/--json output stays lean.
const rawText = new Map();

for (const [store, storeRoot] of Object.entries(STORES)) {
  for (const [task, args] of Object.entries(TASKS)) {
    for (const format of FORMATS) {
      const out = run(storeRoot, args, format);
      const tokens = count(out);
      if (tokens === 0) {
        throw new Error(`measured 0 tokens for ${store}/${task}/${format} - treating as a measurement failure`);
      }
      rawText.set(`${store}|${task}|${format}`, out);
      rows.push({ store, task, format, tokens });
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

  // Gate 1: TOON must meaningfully save tokens vs *compact* JSON for the
  // record-array-shaped tasks (pack, show) on both fixture stores. Excludes
  // advise-prep, which is a prose prompt wrapped in a single JSON/TOON string
  // field - format cannot help there, by design.
  //
  // The baseline is the JSON row's stdout re-serialized compactly
  // (`JSON.stringify(JSON.parse(stdout))`), not the CLI's own `--format
  // json` output, which is pretty-printed (`JSON.stringify(value, null,
  // 2)`) and therefore inflated by indentation/newlines that no real caller
  // would actually send an LLM - comparing TOON against that overstates the
  // win. Measured against the compact baseline (2026-07-27, this fixture
  // set): session-start-pack -16.1%/-23.1% (small/large), scoped-pack
  // -10.7%/-18.1%, show -14.0%/-20.2%. The tightest of those is
  // scoped-pack/store-small at -10.7%; the threshold below (-8%) keeps
  // ~2.7 points of headroom under it while still gating a real win.
  const TABULAR_TASKS = ["session-start-pack", "scoped-pack", "show"];
  for (const store of Object.keys(STORES)) {
    for (const task of TABULAR_TASKS) {
      const j = find(store, task, "json");
      const t = find(store, task, "toon");
      if (!j || !t) {
        problems.push(`missing rows for ${store}/${task}`);
        continue;
      }
      const compactJsonTokens = count(JSON.stringify(JSON.parse(rawText.get(`${store}|${task}|json`))));
      const deltaPct = round1(((t.tokens - compactJsonTokens) / compactJsonTokens) * 100);
      if (deltaPct > -8) {
        problems.push(
          `expected TOON to save >=8% vs compact JSON on ${store}/${task}, got ${deltaPct}%`,
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
