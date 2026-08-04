/**
 * Regression test for the human-in-the-loop guarantee in
 * `.agnosgram/decisions/0003-human-in-the-loop-reflexive-loop.md`:
 * `reflect` leaves the entire working tree untouched, and `feedback` writes
 * only under `.agnosgram/meta/`. Deliberately its own file so the guarantee
 * has one obvious place to check, separate from each command's own
 * behavioral tests.
 */
import assert from "node:assert/strict";
import { mkdtempSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runCli } from "../conformance/harness.js";

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "agnos-reflexive-"));
  assert.equal(runCli(["init", "--adapt", "none"], { cwd: root }).status, 0);
  // A host-project file alongside .agnosgram/, so "the working tree" means
  // more than just the store.
  writeFileSync(join(root, "ROADMAP.md"), "# Roadmap\n- [ ] Milestone 5\n");
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

/** Every regular file under `dir`, recursively, as a project-root-relative
 * path -> mtime, so a before/after diff catches creates, deletes, and edits. */
function snapshot(dir: string, base: string = dir): Map<string, number> {
  const out = new Map<string, number>();
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      for (const [k, v] of snapshot(full, base)) out.set(k, v);
    } else if (entry.isFile()) {
      out.set(relative(base, full), statSync(full).mtimeMs);
    }
  }
  return out;
}

test("reflect leaves the entire working tree untouched", () => {
  runCli(["feedback", "seed friction so reflect has something to digest"], { cwd: root });
  const before = snapshot(root);
  runCli(["reflect"], { cwd: root });
  runCli(["reflect", "--json", "--months", "12"], { cwd: root });
  const after = snapshot(root);
  assert.deepEqual([...after.keys()].sort(), [...before.keys()].sort(), "reflect must not create or delete any file");
  for (const [file, mtime] of before) {
    assert.equal(after.get(file), mtime, `reflect must not modify ${file}`);
  }
});

test("feedback writes only under .agnosgram/meta/", () => {
  const before = snapshot(root);
  runCli(["feedback", "only meta/ should change"], { cwd: root });
  runCli(["feedback", "a second entry, still only meta/"], { cwd: root });
  const after = snapshot(root);

  const beforeKeys = new Set(before.keys());
  const changed = [...after.keys()].filter((k) => after.get(k) !== before.get(k) || !beforeKeys.has(k));
  const removed = [...before.keys()].filter((k) => !after.has(k));

  assert.deepEqual(removed, [], "feedback must never delete a file");
  assert.ok(changed.length > 0, "expected feedback to write at least meta/friction.md");
  for (const rel of changed) {
    assert.ok(
      rel === join(".agnosgram", "meta", "friction.md"),
      `feedback wrote outside .agnosgram/meta/: ${rel}`,
    );
  }
});
