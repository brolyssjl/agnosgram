import assert from "node:assert/strict";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runCli } from "./conformance/harness.js";

let cwd: string;

beforeEach(() => {
  cwd = mkdtempSync(join(tmpdir(), "agnos-cli-"));
});
afterEach(() => {
  rmSync(cwd, { recursive: true, force: true });
});

test("--version prints a bare semver-looking string", () => {
  const res = runCli(["--version"], { cwd });
  assert.equal(res.status, 0);
  assert.match(res.stdout.trim(), /^\d+\.\d+\.\d+$/);
});

test("--help prints usage", () => {
  const res = runCli(["--help"], { cwd });
  assert.equal(res.status, 0);
  assert.match(res.stdout, /Usage:/);
});

test("an unknown command exits non-zero without a raw stack trace", () => {
  const res = runCli(["not-a-command"], { cwd });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr + res.stdout, /Unknown command/);
  assert.ok(!/at Object|node:internal/.test(res.stderr));
});
