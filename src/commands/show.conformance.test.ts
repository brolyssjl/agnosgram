import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runCli } from "../conformance/harness.js";

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "agnos-show-"));
  assert.equal(runCli(["init", "--adapt", "none"], { cwd: root }).status, 0);

  writeFileSync(
    join(root, ".agnosgram", "lessons", "pitfalls.md"),
    `# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [core, tooling]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nDo not use require().\n`,
  );
  writeFileSync(
    join(root, ".agnosgram", "lessons", "conventions.md"),
    `# Conventions\n\n---\nid: CON-001\ntype: convention\nscope: [tooling]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nAlways use node built-ins.\n`,
  );
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

test("show matches an exact record id first", () => {
  const res = runCli(["show", "LES-001"], { cwd: root });
  assert.ok(res.stdout.includes("id: LES-001"));
  assert.ok(res.stdout.includes("Do not use require()."));
  assert.ok(!res.stdout.includes("CON-001"));
});

test("show matches a case-insensitive scope tag when no id matches", () => {
  const res = runCli(["show", "Tooling"], { cwd: root });
  assert.ok(res.stdout.includes("LES-001"));
  assert.ok(res.stdout.includes("CON-001"));
});

test("show matches a type name as a last resort", () => {
  const res = runCli(["show", "convention"], { cwd: root });
  assert.ok(res.stdout.includes("CON-001"));
  assert.ok(!res.stdout.includes("LES-001"));
});

test("--type filters the pool before matching", () => {
  const res = runCli(["show", "tooling", "--type", "convention"], { cwd: root });
  assert.ok(res.stdout.includes("CON-001"));
  assert.ok(!res.stdout.includes("LES-001"));
});

test("no match exits 1 and hints known scopes on stderr", () => {
  const res = runCli(["show", "nonexistent-topic"], { cwd: root });
  assert.equal(res.status, 1);
  assert.ok(res.stderr.includes("No records match"));
  assert.ok(res.stderr.includes("core"));
});

test("--format json prints a uniform flat array", () => {
  const res = runCli(["show", "LES-001", "--format", "json"], { cwd: root });
  const parsed = JSON.parse(res.stdout);
  assert.equal(Array.isArray(parsed), true);
  assert.equal(parsed[0].id, "LES-001");
  assert.equal(parsed[0].scope, "core,tooling");
});

test("--format toon renders a tabular header for multiple matches", () => {
  const res = runCli(["show", "tooling", "--format", "toon"], { cwd: root });
  assert.ok(res.stdout.startsWith("[2]{"));
});

test("missing topic argument throws a usage error", () => {
  const res = runCli(["show"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /Usage: agnosgram show/);
});

test("invalid --type throws a usage error", () => {
  const res = runCli(["show", "tooling", "--type", "bogus"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /--type must be one of/);
});

test("FRI-001: show --type -bogus gives the existing validation error, not a raw parseArgs crash", () => {
  const res = runCli(["show", "tooling", "--type", "-bogus"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /--type must be one of .*, got "-bogus"/);
  assert.ok(!/at Object|node:internal|ERR_PARSE_ARGS/.test(res.stderr));
});

test("a bare -- terminator: everything after it stays a literal positional, not a rewritten option", () => {
  // Empirically pinned against the 0.10.0 binary: `show -- --type -x` must
  // treat "--type" as the (unmatched) topic positional, not fold it and the
  // trailing "-x" into a single --type=-x rewrite.
  const res = runCli(["show", "--", "--type", "-x"], { cwd: root });
  assert.equal(res.status, 1);
  assert.match(res.stderr, /No records match "--type"/);
});

test("--type -- pitfall: -- is never swallowed as --type's value", () => {
  const res = runCli(["show", "tooling", "--type", "--", "pitfall"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /Option '--type' argument is ambiguous/);
  assert.ok(!/at Object|node:internal|ERR_PARSE_ARGS/.test(res.stderr));
});

test("--format json on no match prints an empty JSON array to stdout and still exits 1", () => {
  const res = runCli(["show", "nonexistent-topic", "--format", "json"], { cwd: root });
  assert.equal(res.status, 1);
  const parsed = JSON.parse(res.stdout);
  assert.deepEqual(parsed, []);
  // Structured mode never writes the human hints to stderr.
  assert.equal(res.stderr, "");
});

test("show never surfaces a meta/ friction entry, even by exact id or its scope tag", () => {
  mkdirSync(join(root, ".agnosgram", "meta"), { recursive: true });
  writeFileSync(
    join(root, ".agnosgram", "meta", "friction.md"),
    `# Friction\n\n---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: meta/friction.md\n---\nTool friction, not host-project memory.\n`,
  );
  const res = runCli(["show", "FRI-001", "--format", "json"], { cwd: root });
  assert.deepEqual(JSON.parse(res.stdout), []);
  assert.equal(res.status, 1);
});

test("--format toon on no match prints an empty TOON array to stdout and still exits 1", () => {
  const res = runCli(["show", "nonexistent-topic", "--format", "toon"], { cwd: root });
  assert.equal(res.status, 1);
  assert.ok(res.stdout.trim().length > 0, "expected a non-empty structured payload on stdout");
  assert.equal(res.stderr, "");
});
