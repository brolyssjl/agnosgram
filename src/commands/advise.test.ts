import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runAdvise } from "./advise.js";
import { runInit } from "./init.js";

let root: string;
let cwd: string;
let out: string;
let origWrite: typeof process.stdout.write;

function writeLesson(): void {
  writeFileSync(
    join(root, ".agnosgram", "lessons", "pitfalls.md"),
    `# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nDo not use require() in this ESM package; it fails at runtime.\n`,
  );
}

function validReport(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    agnosgram_advise: 1,
    plan: "plan.md",
    generated: "2026-07-27",
    checked_ids: ["LES-001"],
    contradictions: [
      {
        record_id: "LES-001",
        kind: "empirical",
        severity: "blocker",
        plan_excerpt: "use require() everywhere",
        record_excerpt: "Do not use require() in this ESM package",
        confidence: "high",
        last_verified: "2026-07-21",
        explanation: "Plan proposes require() but LES-001 forbids it.",
      },
    ],
    clear: false,
    ...overrides,
  };
}

beforeEach(() => {
  cwd = process.cwd();
  root = mkdtempSync(join(tmpdir(), "agnos-advise-"));
  process.chdir(root);
  runInit(["--adapt", "none"]);
  writeLesson();
  writeFileSync(join(root, "plan.md"), "We propose to use require() everywhere for simplicity.\n");

  out = "";
  origWrite = process.stdout.write.bind(process.stdout);
  process.stdout.write = ((chunk: string | Uint8Array) => {
    out += chunk.toString();
    return true;
  }) as typeof process.stdout.write;
});
afterEach(() => {
  process.stdout.write = origWrite;
  process.chdir(cwd);
  rmSync(root, { recursive: true, force: true });
  process.exitCode = 0;
});

test("advise emits a prompt naming the plan, the digest, and the precedence rule verbatim", () => {
  runAdvise(["plan.md"]);
  assert.ok(out.includes("advise task"));
  assert.ok(out.includes("plan.md"));
  assert.ok(out.includes("LES-001"));
  assert.ok(out.includes("Normative contradiction"));
  assert.ok(out.includes("Empirical contradiction"));
  assert.ok(out.includes("agnosgram advise --validate plan.md.advise.json"));
});

test("advise --out changes the report path referenced in the prompt", () => {
  runAdvise(["plan.md", "--out", "custom.json"]);
  assert.ok(out.includes("custom.json"));
});

test("advise --validate passes a well-formed, provenance-correct report", () => {
  writeFileSync(join(root, "report.json"), JSON.stringify(validReport()));
  runAdvise(["--validate", "report.json"]);
  assert.ok(out.includes("valid"));
  assert.notEqual(process.exitCode, 1);
});

test("advise --validate fails on a confidence provenance mismatch", () => {
  writeFileSync(join(root, "report.json"), JSON.stringify(validReport({
    contradictions: [{ ...(validReport().contradictions as unknown[])[0] as object, confidence: "low" }],
  })));
  runAdvise(["--validate", "report.json"]);
  assert.equal(process.exitCode, 1);
  assert.ok(out.includes("provenance.confidence.mismatch"));
});

test("advise --validate fails when a cited record_id does not exist", () => {
  writeFileSync(join(root, "report.json"), JSON.stringify(validReport({
    contradictions: [{ ...(validReport().contradictions as unknown[])[0] as object, record_id: "LES-999" }],
  })));
  runAdvise(["--validate", "report.json"]);
  assert.equal(process.exitCode, 1);
  assert.ok(out.includes("provenance.record_id.unknown"));
});

test("advise --validate warns (does not error) on an excerpt substring mismatch", () => {
  writeFileSync(join(root, "report.json"), JSON.stringify(validReport({
    contradictions: [{ ...(validReport().contradictions as unknown[])[0] as object, plan_excerpt: "not in the plan" }],
  })));
  runAdvise(["--validate", "report.json"]);
  assert.notEqual(process.exitCode, 1);
  assert.ok(out.includes("excerpt.plan_mismatch"));
});

test("advise --validate: exit 0 without --strict, exit 1 with --strict, when clear is false but valid", () => {
  writeFileSync(
    join(root, "report.json"),
    JSON.stringify(validReport({ contradictions: [], clear: false })),
  );
  runAdvise(["--validate", "report.json"]);
  assert.notEqual(process.exitCode, 1);

  process.exitCode = 0;
  out = "";
  runAdvise(["--validate", "report.json", "--strict"]);
  assert.equal(process.exitCode, 1);
});

test("advise --validate --json returns {file, ok, errors, warnings, issues, report}", () => {
  writeFileSync(join(root, "report.json"), JSON.stringify(validReport()));
  runAdvise(["--validate", "report.json", "--json"]);
  const parsed = JSON.parse(out);
  assert.equal(parsed.file, "report.json");
  assert.equal(parsed.ok, true);
  assert.equal(parsed.errors, 0);
  assert.equal(typeof parsed.warnings, "number");
  assert.ok(Array.isArray(parsed.issues));
  assert.equal(parsed.report.agnosgram_advise, 1);
});

test("advise --validate rejects malformed JSON", () => {
  writeFileSync(join(root, "bad.json"), "{not json");
  runAdvise(["--validate", "bad.json"]);
  assert.equal(process.exitCode, 1);
  assert.ok(out.includes("json.parse"));
});

test("advise --validate on a missing file throws a usage error", () => {
  assert.throws(() => runAdvise(["--validate", "nope.json"]), /No such file/);
});

test("advise with no plan path throws a usage error", () => {
  assert.throws(() => runAdvise([]), /Usage: agnosgram advise/);
});
