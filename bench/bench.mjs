#!/usr/bin/env node
/**
 * Tier-1 token benchmark (deterministic, free, CI-friendly).
 *
 * Serializes representative Agnosgram payloads in each candidate output format
 * and counts tokens with gpt-tokenizer (o200k proxy, same as TOON's own
 * benchmarks). Three columns: compact JSON, the faithful `@toon-format/toon`
 * encoder (exact-pinned dev dep - the real spec), and our hand-rolled runtime
 * encoder (`src/core/toon.ts`, compiled to `dist/core/toon.js`). Closes
 * LES-002: the old toy serializer here could not reproduce TOON's real
 * small-object overhead, so this bench could only honestly guard the
 * uniform-array (tabular) win. With the faithful encoder in the loop we can
 * now also check the small-object loss and how closely our own encoder
 * tracks the real one.
 *
 * Requires `npm run build` first (imports the compiled runtime encoder).
 *
 * Usage:
 *   node bench/bench.mjs            # print a table
 *   node bench/bench.mjs --json     # machine-readable, for CI gating
 *   node bench/bench.mjs --check    # exit non-zero if a regression threshold is crossed
 */
import { encode } from "gpt-tokenizer/model/gpt-4o";
import { encode as encodeToonFaithful } from "@toon-format/toon";
import { encodeToon as encodeToonRuntime } from "../dist/core/toon.js";

/** Payloads that mirror what `pack`/`show` emit. */
const PAYLOADS = {
  // Small, non-uniform object with plain field values. Informational: with
  // the faithful encoder this one can go either way depending on whether any
  // value needs quoting - it happens to win slightly here, which is itself
  // the honest-numbers point (the old toy serializer could not tell you
  // that). Not used for the honesty gate below; see `record`.
  status: {
    focus: "Milestone 1 CLI",
    inFlight: ["feat/adapt-idempotence"],
    next: "distill prompt emission",
    blockedOn: null,
  },
  // Small, non-uniform object shaped like a single `show`/`pack` record: a
  // realistic body carries punctuation (quotes, colons, backticks) that
  // forces TOON's quoting/escaping rules to fire - this is where the
  // design-session "+5% on small objects" overhead actually shows up.
  record: {
    id: "LES-001",
    type: "pitfall",
    scope: "core,tooling",
    confidence: "high",
    last_verified: "2026-07-21",
    body: 'Do not use CommonJS require() in this package - it is ESM (`"type": "module"`) with verbatimModuleSyntax. `require` fails at runtime.',
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

function count(text) {
  return encode(text).length;
}

function round1(n) {
  return Number(n.toFixed(1));
}

const rows = [];
for (const [name, payload] of Object.entries(PAYLOADS)) {
  const json = count(toCompactJson(payload));
  const toonFaithful = count(encodeToonFaithful(payload));
  const toonRuntime = count(encodeToonRuntime(payload));
  const faithfulDeltaPct = round1(((toonFaithful - json) / json) * 100);
  const runtimeDeltaPct = round1(((toonRuntime - json) / json) * 100);
  const driftPct = round1(((toonRuntime - toonFaithful) / toonFaithful) * 100);
  rows.push({
    payload: name,
    json,
    toonFaithful,
    toonRuntime,
    faithfulDeltaPct,
    runtimeDeltaPct,
    driftPct,
  });
}

const args = new Set(process.argv.slice(2));

if (args.has("--json")) {
  process.stdout.write(JSON.stringify(rows, null, 2) + "\n");
} else {
  process.stdout.write(
    "payload      json   toonFaithful  toonRuntime  faithfulΔ%  runtimeΔ%  drift%\n",
  );
  process.stdout.write(
    "----------------------------------------------------------------------------\n",
  );
  for (const r of rows) {
    const pct = (n) => `${n > 0 ? "+" : ""}${n}%`;
    process.stdout.write(
      `${r.payload.padEnd(12)} ${String(r.json).padEnd(6)} ${String(r.toonFaithful).padEnd(13)} ${String(r.toonRuntime).padEnd(12)} ${pct(r.faithfulDeltaPct).padEnd(11)} ${pct(r.runtimeDeltaPct).padEnd(10)} ${pct(r.driftPct)}\n`,
    );
  }
}

if (args.has("--check")) {
  const lessons = rows.find((r) => r.payload === "lessons");
  const record = rows.find((r) => r.payload === "record");
  const problems = [];

  // Gate 1: the faithful encoder must still save >=15% on the uniform,
  // tabular `lessons` array - the core TOON value proposition.
  if (!lessons || lessons.faithfulDeltaPct > -15) {
    problems.push(
      `expected faithful TOON to save >=15% on the uniform \`lessons\` array, got ${lessons ? lessons.faithfulDeltaPct : "n/a"}%`,
    );
  }

  // Gate 2 (small-object honesty): the faithful encoder must not show a net
  // win on a realistic small, non-uniform `record` object (prose body with
  // punctuation that forces TOON's quoting rules) - this is the
  // design-session finding the old toy serializer could not reproduce
  // (LES-002). If this ever turns negative, the "TOON is opt-in, never
  // default" decision needs revisiting.
  if (!record || record.faithfulDeltaPct < 0) {
    problems.push(
      `expected faithful TOON to not win on the small \`record\` object, got ${record ? record.faithfulDeltaPct : "n/a"}%`,
    );
  }

  // Gate 3 (drift guard): our hand-rolled runtime encoder targets uniform
  // arrays specifically (kickoff decision) and should track the faithful
  // encoder's token economy there within +-10%, or the port has drifted.
  if (!lessons || Math.abs(lessons.driftPct) > 10) {
    problems.push(
      `hand-rolled toon.ts drifted >10% from the faithful encoder on \`lessons\`, got ${lessons ? lessons.driftPct : "n/a"}%`,
    );
  }

  if (problems.length > 0) {
    process.stderr.write("bench regression:\n- " + problems.join("\n- ") + "\n");
    process.exit(1);
  }
}
