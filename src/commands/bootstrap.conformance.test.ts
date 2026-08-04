import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runCli } from "../conformance/harness.js";

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "agnos-bootstrap-"));
  writeFileSync(join(root, "package.json"), `{ "name": "demo" }\n`);
  assert.equal(runCli(["init", "--adapt", "none"], { cwd: root }).status, 0);
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

test("bootstrap emits a prompt targeting context files and detected stack", () => {
  const res = runCli(["bootstrap"], { cwd: root });
  assert.equal(res.status, 0);
  assert.ok(res.stdout.includes("bootstrap task"));
  assert.ok(res.stdout.includes("context/architecture.md"));
  assert.ok(res.stdout.includes("context/domain.md"));
  assert.ok(res.stdout.includes("package.json")); // top-level entry surfaced
  assert.ok(res.stdout.includes("Node")); // stack signal surfaced
});

test("FRI-001: an unknown flag gives a clean UserError, not a raw parseArgs crash", () => {
  const res = runCli(["bootstrap", "--nope"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /Unknown option/);
  assert.ok(!/at Object|node:internal|ERR_PARSE_ARGS/.test(res.stderr));
});
