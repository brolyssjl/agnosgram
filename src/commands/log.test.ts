import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runInit } from "./init.js";
import { currentBranch, formatEntry, runLog } from "./log.js";

let root: string;
let cwd: string;

beforeEach(() => {
  cwd = process.cwd();
  root = mkdtempSync(join(tmpdir(), "agnos-log-"));
  process.chdir(root);
  runInit(["--adapt", "none"]);
});
afterEach(() => {
  process.chdir(cwd);
  rmSync(root, { recursive: true, force: true });
});

test("formatEntry only emits provided slots", () => {
  const entry = formatEntry(
    { did: "shipped", next: "monitor" },
    { agent: "claude", branch: "feat/x", when: new Date("2026-07-21T14:20:00") },
  );
  assert.ok(entry.includes("· claude · feat/x"));
  assert.ok(entry.includes("- **Did:** shipped"));
  assert.ok(entry.includes("- **Next:** monitor"));
  assert.ok(!entry.includes("Learned"));
});

test("log appends an entry to the current month's journal", () => {
  runLog(["--did", "wired the CLI", "--learned", "parseArgs is enough", "--json"]);
  const month = new Date().toISOString().slice(0, 7);
  const journal = readFileSync(join(root, ".agnosgram", "journal", `${month}.md`), "utf8");
  assert.ok(journal.includes("- **Did:** wired the CLI"));
  assert.ok(journal.includes("- **Learned:** parseArgs is enough"));
});

test("log with no slots throws a helpful error", () => {
  assert.throws(() => runLog([]), /Nothing to log/);
});

test("currentBranch returns null outside a git repo", () => {
  assert.equal(currentBranch(root), null);
});
