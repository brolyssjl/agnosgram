import assert from "node:assert/strict";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { currentBranch, formatEntry } from "./log.js";

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "agnos-log-"));
});
afterEach(() => {
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

test("currentBranch returns null outside a git repo", () => {
  assert.equal(currentBranch(root), null);
});
