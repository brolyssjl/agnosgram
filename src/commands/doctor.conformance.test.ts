import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runCli } from "../conformance/harness.js";

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "agnos-doctor-conf-"));
  assert.equal(runCli(["init", "--adapt", "none"], { cwd: root }).status, 0);
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

test("doctor on a healthy store exits 0 and reports no issues", () => {
  const res = runCli(["doctor"], { cwd: root });
  assert.equal(res.status, 0);
  assert.match(res.stdout, /no issues found/);
});

test("doctor --json prints the report shape", () => {
  const res = runCli(["doctor", "--json"], { cwd: root });
  assert.equal(res.status, 0);
  const parsed = JSON.parse(res.stdout);
  assert.equal(parsed.ok, true);
  assert.equal(parsed.errors, 0);
  assert.ok(Array.isArray(parsed.findings));
});

test("warnings alone keep exit 0 without --strict", () => {
  writeFileSync(
    join(root, ".agnosgram", "lessons", "pitfalls.md"),
    `# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [x]\nconfidence: high\ncreated: 2000-01-01\nlast_verified: 2000-01-01\nsource: journal/2026-07.md\n---\nvery old lesson\n`,
  );
  const res = runCli(["doctor"], { cwd: root });
  assert.equal(res.status, 0);
  assert.match(res.stdout, /warning/);
});

test("--strict turns warnings into a failing exit code", () => {
  writeFileSync(
    join(root, ".agnosgram", "lessons", "pitfalls.md"),
    `# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [x]\nconfidence: high\ncreated: 2000-01-01\nlast_verified: 2000-01-01\nsource: journal/2026-07.md\n---\nvery old lesson\n`,
  );
  const res = runCli(["doctor", "--strict"], { cwd: root });
  assert.equal(res.status, 1);
});

test("doctor without a store gives a clean UserError", () => {
  const empty = mkdtempSync(join(tmpdir(), "agnos-doctor-nostore-"));
  try {
    const res = runCli(["doctor"], { cwd: empty });
    assert.notEqual(res.status, 0);
    assert.match(res.stderr, /No \.agnosgram\/ store found/);
  } finally {
    rmSync(empty, { recursive: true, force: true });
  }
});

test("FRI-001: doctor --format -json gives the existing validation error, not a raw parseArgs crash", () => {
  const res = runCli(["doctor", "--format", "-json"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /unknown --format "-json"/);
  assert.ok(!/at Object|node:internal|ERR_PARSE_ARGS/.test(res.stderr));
});
