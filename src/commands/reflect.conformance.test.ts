import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runCli } from "../conformance/harness.js";

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "agnos-reflect-"));
  assert.equal(runCli(["init", "--adapt", "none"], { cwd: root }).status, 0);
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

function writeFriction(root_: string, body: string): void {
  mkdirSync(join(root_, ".agnosgram", "meta"), { recursive: true });
  writeFileSync(join(root_, ".agnosgram", "meta", "friction.md"), `# Friction\n\n${body}`);
}

/** Snapshot every file's mtime + size under root, recursively, for a
 * before/after no-writes comparison. */
function snapshot(dir: string): Map<string, number> {
  const out_ = new Map<string, number>();
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      for (const [k, v] of snapshot(full)) out_.set(k, v);
    } else if (entry.isFile()) {
      out_.set(full, statSync(full).mtimeMs);
    }
  }
  return out_;
}

test("reflect emits a prompt naming friction, journal months, and the ROADMAP rule", () => {
  const res = runCli(["reflect"], { cwd: root });
  assert.equal(res.status, 0);
  assert.ok(res.stdout.includes("reflect task"));
  assert.ok(res.stdout.includes("meta/friction.md"));
  assert.ok(res.stdout.includes("ROADMAP.md is owner-edited") || res.stdout.includes("owner-edited"));
  assert.ok(
    res.stdout.includes("never to `ROADMAP.md` directly") ||
      res.stdout.includes("never applied automatically") ||
      res.stdout.includes("never disposes"),
  );
});

test("reflect includes captured friction entries in its digest", () => {
  writeFriction(
    root,
    `---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: meta/friction.md\n---\nThe doctor error message was confusing.\n`,
  );
  const res = runCli(["reflect"], { cwd: root });
  assert.ok(res.stdout.includes("FRI-001"));
  assert.ok(res.stdout.includes("doctor error message was confusing"));
});

test("reflect --json returns a versioned envelope with friction and journal coverage", () => {
  writeFriction(
    root,
    `---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: meta/friction.md\n---\nSomething was confusing.\n`,
  );
  const res = runCli(["reflect", "--json"], { cwd: root });
  const parsed = JSON.parse(res.stdout);
  assert.equal(parsed.agnosgram_reflect, 1);
  assert.deepEqual(parsed.friction_ids, ["FRI-001"]);
  assert.equal(parsed.friction_count, 1);
  assert.ok(Array.isArray(parsed.journal_months));
  assert.equal(typeof parsed.prompt, "string");
});

test("reflect --months limits how many recent journal months are listed", () => {
  const journalDir = join(root, ".agnosgram", "journal");
  for (const m of ["2026-01", "2026-02", "2026-03", "2026-04"]) {
    writeFileSync(join(journalDir, `${m}.md`), `# Journal - ${m}\n`);
  }
  // init already created the current month's journal file too, so with
  // --months 2 the two most recent (by name) win, whatever "current" is.
  const allMonths = readdirSync(journalDir)
    .filter((f) => /^\d{4}-\d{2}\.md$/.test(f))
    .map((f) => f.slice(0, 7))
    .sort();
  const expected = allMonths.slice(-2);

  const res = runCli(["reflect", "--months", "2", "--json"], { cwd: root });
  const parsed = JSON.parse(res.stdout);
  assert.deepEqual(parsed.journal_months.slice().sort(), expected);
});

test("reflect rejects a non-positive --months", () => {
  const res = runCli(["reflect", "--months", "0"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /--months must be a positive integer/);
});

test("reflect rejects a non-integer --months instead of silently truncating", () => {
  const res = runCli(["reflect", "--months", "2.5"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /--months must be a positive integer/);
});

test("reflect rejects a --months with trailing junk instead of silently parsing a prefix", () => {
  const res = runCli(["reflect", "--months", "3abc"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /--months must be a positive integer/);
});

test("FRI-001: reflect --months -1 gives the existing validation error, not a raw parseArgs crash", () => {
  const res = runCli(["reflect", "--months", "-1"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /--months must be a positive integer, got "-1"/);
  assert.ok(!/at Object|node:internal|ERR_PARSE_ARGS/.test(res.stderr));
});

test("reflect performs no writes to the repo (read-only)", () => {
  writeFriction(
    root,
    `---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: meta/friction.md\n---\nSome friction.\n`,
  );
  const before = snapshot(join(root, ".agnosgram"));
  runCli(["reflect"], { cwd: root });
  runCli(["reflect", "--json"], { cwd: root });
  const after = snapshot(join(root, ".agnosgram"));
  assert.deepEqual([...after.keys()].sort(), [...before.keys()].sort(), "reflect must not create or delete any file");
  for (const [file, mtime] of before) {
    assert.equal(after.get(file), mtime, `reflect must not modify ${file}`);
  }
});
