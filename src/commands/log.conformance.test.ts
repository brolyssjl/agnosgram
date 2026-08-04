import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runCli } from "../conformance/harness.js";

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "agnos-log-"));
  assert.equal(runCli(["init", "--adapt", "none"], { cwd: root }).status, 0);
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

test("log appends an entry to the current month's journal", () => {
  const res = runCli(["log", "--did", "wired the CLI", "--learned", "parseArgs is enough", "--json"], { cwd: root });
  assert.equal(res.status, 0);
  const month = new Date().toISOString().slice(0, 7);
  const journal = readFileSync(join(root, ".agnosgram", "journal", `${month}.md`), "utf8");
  assert.ok(journal.includes("- **Did:** wired the CLI"));
  assert.ok(journal.includes("- **Learned:** parseArgs is enough"));
});

test("log with no slots throws a helpful error", () => {
  const res = runCli(["log"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /Nothing to log/);
});

test('FRI-001: log --learned "--x" stores the literal value instead of crashing', () => {
  const res = runCli(["log", "--learned", "--x", "--json"], { cwd: root });
  assert.equal(res.status, 0);
  assert.ok(!/at Object|node:internal|ERR_PARSE_ARGS/.test(res.stderr));
  const month = new Date().toISOString().slice(0, 7);
  const journal = readFileSync(join(root, ".agnosgram", "journal", `${month}.md`), "utf8");
  assert.ok(journal.includes("- **Learned:** --x"));
});
