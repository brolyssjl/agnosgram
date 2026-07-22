#!/usr/bin/env node
/**
 * Tier-1 token benchmark (deterministic, free, CI-friendly).
 *
 * Serializes representative Agnosgram payloads in each candidate output format and
 * counts tokens with gpt-tokenizer (o200k proxy, same as TOON's own benchmarks).
 * Purpose: catch format regressions and confirm the design-session finding that
 * TOON only wins on uniform arrays, and loses on small non-uniform objects.
 *
 * Usage:
 *   node bench/bench.mjs            # print a table
 *   node bench/bench.mjs --json     # machine-readable, for CI gating
 *   node bench/bench.mjs --check    # exit non-zero if a regression threshold is crossed
 */
import { encode } from "gpt-tokenizer/model/gpt-4o";

/** Payloads that mirror what `pack`/`show` will emit once those land. */
const PAYLOADS = {
  // Small, non-uniform object — TOON is expected to LOSE here.
  status: {
    focus: "Milestone 1 CLI",
    inFlight: ["feat/adapt-idempotence"],
    next: "distill prompt emission",
    blockedOn: null,
  },
  // Uniform array of records — TOON is expected to WIN here.
  lessons: Array.from({ length: 12 }, (_, i) => ({
    id: `LES-${String(i + 1).padStart(3, "0")}`,
    type: i % 2 ? "pitfall" : "convention",
    scope: "backend",
    confidence: "high",
    last_verified: "2026-07-21",
  })),
};

function toCompactJson(v) {
  return JSON.stringify(v);
}

/**
 * Tiny TOON-ish serializer, enough to size the tabular (uniform-array) case
 * without pulling a dependency. It is deliberately NOT a faithful TOON encoder —
 * the small-object overhead the design session measured (+5% vs compact JSON)
 * comes from real TOON's quoting/structure rules this toy skips, so we do not
 * assert that finding here. Faithful small-object numbers need the real
 * `@toon-format` encoder, which arrives as a dev-dep when the serializer lands
 * (Milestone 2). Never TOON for storage; this only measures the wire format.
 */
function toToon(v) {
  if (Array.isArray(v) && v.length > 0 && v.every((x) => x && typeof x === "object")) {
    const keys = Object.keys(v[0]);
    const header = `[${v.length}]{${keys.join(",")}}:`;
    const rows = v.map((row) => "  " + keys.map((k) => fmt(row[k])).join(","));
    return [header, ...rows].join("\n");
  }
  if (v && typeof v === "object") {
    return Object.entries(v)
      .map(([k, val]) => `${k}: ${fmt(val)}`)
      .join("\n");
  }
  return fmt(v);
}

function fmt(v) {
  if (v === null) return "null";
  if (Array.isArray(v)) return v.join("|");
  return String(v);
}

function count(text) {
  return encode(text).length;
}

const rows = [];
for (const [name, payload] of Object.entries(PAYLOADS)) {
  const json = count(toCompactJson(payload));
  const toon = count(toToon(payload));
  const deltaPct = ((toon - json) / json) * 100;
  rows.push({ payload: name, json, toon, toonDeltaPct: Number(deltaPct.toFixed(1)) });
}

const args = new Set(process.argv.slice(2));

if (args.has("--json")) {
  process.stdout.write(JSON.stringify(rows, null, 2) + "\n");
} else {
  process.stdout.write("payload      json   toon   toon Δ%\n");
  process.stdout.write("------------------------------------\n");
  for (const r of rows) {
    process.stdout.write(
      `${r.payload.padEnd(12)} ${String(r.json).padEnd(6)} ${String(r.toon).padEnd(6)} ${r.toonDeltaPct > 0 ? "+" : ""}${r.toonDeltaPct}%\n`,
    );
  }
}

if (args.has("--check")) {
  // Regression guard for what this toy serializer can honestly measure: the
  // uniform-array (tabular) case must keep saving enough to justify `--format toon`
  // for those sections. Threshold is conservative vs the measured ~-26%..-50%.
  const lessons = rows.find((r) => r.payload === "lessons");
  const problems = [];
  if (!lessons || lessons.toonDeltaPct > -15) {
    problems.push(
      `expected TOON to save >=15% on the uniform \`lessons\` array, got ${lessons ? lessons.toonDeltaPct : "n/a"}%`,
    );
  }
  if (problems.length > 0) {
    process.stderr.write("bench regression:\n- " + problems.join("\n- ") + "\n");
    process.exit(1);
  }
}
