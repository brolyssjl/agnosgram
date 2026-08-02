import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runCli } from "../conformance/harness.js";

let root: string;

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
  root = mkdtempSync(join(tmpdir(), "agnos-advise-"));
  assert.equal(runCli(["init", "--adapt", "none"], { cwd: root }).status, 0);
  writeLesson();
  writeFileSync(join(root, "plan.md"), "We propose to use require() everywhere for simplicity.\n");
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

test("advise emits a prompt naming the plan, the digest, and the precedence rule verbatim", () => {
  const res = runCli(["advise", "plan.md"], { cwd: root });
  assert.equal(res.status, 0);
  assert.ok(res.stdout.includes("advise task"));
  assert.ok(res.stdout.includes("plan.md"));
  assert.ok(res.stdout.includes("LES-001"));
  assert.ok(res.stdout.includes("Normative contradiction"));
  assert.ok(res.stdout.includes("Empirical contradiction"));
  assert.ok(res.stdout.includes("agnosgram advise --validate plan.md.advise.json"));
});

test("advise --out changes the report path referenced in the prompt", () => {
  const res = runCli(["advise", "plan.md", "--out", "custom.json"], { cwd: root });
  assert.ok(res.stdout.includes("custom.json"));
});

test("FRI-001: advise --out -custom.json accepts a dash-leading path, not a crash", () => {
  const res = runCli(["advise", "plan.md", "--out", "-custom.json"], { cwd: root });
  assert.equal(res.status, 0);
  assert.ok(res.stdout.includes("-custom.json"));
  assert.ok(!/at Object|node:internal|ERR_PARSE_ARGS/.test(res.stderr));
});

test("advise --validate passes a well-formed, provenance-correct report", () => {
  writeFileSync(join(root, "report.json"), JSON.stringify(validReport()));
  const res = runCli(["advise", "--validate", "report.json"], { cwd: root });
  assert.ok(res.stdout.includes("valid"));
  assert.notEqual(res.status, 1);
});

test("advise --validate fails on a confidence provenance mismatch", () => {
  writeFileSync(join(root, "report.json"), JSON.stringify(validReport({
    contradictions: [{ ...(validReport().contradictions as unknown[])[0] as object, confidence: "low" }],
  })));
  const res = runCli(["advise", "--validate", "report.json"], { cwd: root });
  assert.equal(res.status, 1);
  assert.ok(res.stdout.includes("provenance.confidence.mismatch"));
});

test("advise --validate fails when a cited record_id does not exist", () => {
  writeFileSync(join(root, "report.json"), JSON.stringify(validReport({
    contradictions: [{ ...(validReport().contradictions as unknown[])[0] as object, record_id: "LES-999" }],
  })));
  const res = runCli(["advise", "--validate", "report.json"], { cwd: root });
  assert.equal(res.status, 1);
  assert.ok(res.stdout.includes("provenance.record_id.unknown"));
});

test("advise --validate warns (does not error) on an excerpt substring mismatch", () => {
  writeFileSync(join(root, "report.json"), JSON.stringify(validReport({
    contradictions: [{ ...(validReport().contradictions as unknown[])[0] as object, plan_excerpt: "not in the plan" }],
  })));
  const res = runCli(["advise", "--validate", "report.json"], { cwd: root });
  assert.notEqual(res.status, 1);
  assert.ok(res.stdout.includes("excerpt.plan_mismatch"));
});

test("advise --validate: exit 0 without --strict, exit 1 with --strict, when clear is false but valid", () => {
  writeFileSync(
    join(root, "report.json"),
    JSON.stringify(validReport({ contradictions: [], clear: false })),
  );
  let res = runCli(["advise", "--validate", "report.json"], { cwd: root });
  assert.notEqual(res.status, 1);

  res = runCli(["advise", "--validate", "report.json", "--strict"], { cwd: root });
  assert.equal(res.status, 1);
});

test("advise --validate --json returns {file, ok, errors, warnings, issues, report}", () => {
  writeFileSync(join(root, "report.json"), JSON.stringify(validReport()));
  const res = runCli(["advise", "--validate", "report.json", "--json"], { cwd: root });
  const parsed = JSON.parse(res.stdout);
  assert.equal(parsed.file, "report.json");
  assert.equal(parsed.ok, true);
  assert.equal(parsed.errors, 0);
  assert.equal(typeof parsed.warnings, "number");
  assert.ok(Array.isArray(parsed.issues));
  assert.equal(parsed.report.agnosgram_advise, 1);
});

test("advise --validate rejects malformed JSON", () => {
  writeFileSync(join(root, "bad.json"), "{not json");
  const res = runCli(["advise", "--validate", "bad.json"], { cwd: root });
  assert.equal(res.status, 1);
  assert.ok(res.stdout.includes("json.parse"));
});

test("advise --validate on a missing file throws a usage error", () => {
  const res = runCli(["advise", "--validate", "nope.json"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /No such file/);
});

test("advise with no plan path throws a usage error", () => {
  const res = runCli(["advise"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /Usage: agnosgram advise/);
});

test("advise --validate errors (even without --strict) when clear:true coexists with a blocker", () => {
  writeFileSync(join(root, "report.json"), JSON.stringify(validReport({ clear: true })));
  const res = runCli(["advise", "--validate", "report.json"], { cwd: root });
  assert.equal(res.status, 1);
  assert.ok(res.stdout.includes("consistency.clear"));
  assert.ok(res.stdout.includes("blocker contradiction is present"));
});

test("advise --validate --strict derives exit mechanically: blocker + clear:true still exits 1", () => {
  writeFileSync(join(root, "report.json"), JSON.stringify(validReport({ clear: true })));
  const res = runCli(["advise", "--validate", "report.json", "--strict"], { cwd: root });
  assert.equal(res.status, 1);
});

test("advise --validate fails when checked_ids contains a non-string entry", () => {
  writeFileSync(
    join(root, "report.json"),
    JSON.stringify(validReport({ checked_ids: ["LES-001", 42] })),
  );
  const res = runCli(["advise", "--validate", "report.json"], { cwd: root });
  assert.equal(res.status, 1);
  assert.ok(res.stdout.includes("schema.checked_ids"));
});

test("advise --validate falls back to a root-relative plan path from a subdirectory", () => {
  // The report's `plan` field is project-root-relative (the normal shape,
  // matching what `buildPrompt` documents); validating from a subdirectory
  // means the cwd-relative lookup misses and must fall back to root-relative.
  mkdirSync(join(root, "sub"), { recursive: true });
  writeFileSync(join(root, "sub", "report.json"), JSON.stringify(validReport()));
  const res = runCli(["advise", "--validate", "report.json"], { cwd: join(root, "sub") });
  assert.ok(res.stdout.includes("valid"));
  assert.ok(!res.stdout.includes("coverage.plan_missing"));
});

test("advise --validate resolves an absolute plan path", () => {
  writeFileSync(
    join(root, "report.json"),
    JSON.stringify(validReport({ plan: join(root, "plan.md") })),
  );
  const res = runCli(["advise", "--validate", "report.json"], { cwd: root });
  assert.ok(res.stdout.includes("valid"));
  assert.ok(!res.stdout.includes("coverage.plan_missing"));
});

test("advise's digest never includes meta/friction.md content", () => {
  mkdirSync(join(root, ".agnosgram", "meta"), { recursive: true });
  writeFileSync(
    join(root, ".agnosgram", "meta", "friction.md"),
    `# Friction\n\n---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: meta/friction.md\n---\nThis is tool friction, not host-project memory.\n`,
  );
  const res = runCli(["advise", "plan.md"], { cwd: root });
  assert.ok(!res.stdout.includes("FRI-001"));
  assert.ok(!res.stdout.includes("tool friction, not host-project memory"));
});

test("advise digest table escapes pipes in body excerpts and only appends ... when truncated", () => {
  writeFileSync(
    join(root, ".agnosgram", "lessons", "pitfalls.md"),
    `# Pitfalls\n\n---\nid: LES-002\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nShort body with a | pipe in it.\n`,
  );
  const res = runCli(["advise", "plan.md"], { cwd: root });
  // The literal "|" in the body must be escaped so it does not fracture the
  // Markdown table into extra columns.
  assert.ok(res.stdout.includes("a \\| pipe in it."));
  // A short body (well under 80 chars after normalizing) must not get a
  // trailing "..." - the old bug appended it whenever the slice happened
  // to land at exactly 80 chars, even when nothing was actually cut.
  const row = res.stdout.split("\n").find((l) => l.includes("LES-002"));
  assert.ok(row);
  assert.ok(!row!.includes("..."));
});

test("advise digest table appends ... only when the body actually exceeds 80 chars", () => {
  writeFileSync(
    join(root, ".agnosgram", "lessons", "pitfalls.md"),
    `# Pitfalls\n\n---\nid: LES-003\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\n${"x".repeat(120)}\n`,
  );
  const res = runCli(["advise", "plan.md"], { cwd: root });
  const row = res.stdout.split("\n").find((l) => l.includes("LES-003"));
  assert.ok(row);
  assert.ok(row!.includes("..."));
});
