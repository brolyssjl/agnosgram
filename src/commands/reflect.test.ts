import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runInit } from "./init.js";
import { runReflect } from "./reflect.js";

let root: string;
let cwd: string;
let out: string;
let origWrite: typeof process.stdout.write;

beforeEach(() => {
  cwd = process.cwd();
  root = mkdtempSync(join(tmpdir(), "agnos-reflect-"));
  process.chdir(root);
  runInit(["--adapt", "none"]);
  out = "";
  origWrite = process.stdout.write.bind(process.stdout);
  process.stdout.write = ((chunk: string | Uint8Array) => {
    out += chunk.toString();
    return true;
  }) as typeof process.stdout.write;
});
afterEach(() => {
  process.stdout.write = origWrite;
  process.chdir(cwd);
  rmSync(root, { recursive: true, force: true });
  process.exitCode = 0;
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
  runReflect([]);
  assert.ok(out.includes("reflect task"));
  assert.ok(out.includes("meta/friction.md"));
  assert.ok(out.includes("ROADMAP.md is owner-edited") || out.includes("owner-edited"));
  assert.ok(out.includes("never to `ROADMAP.md` directly") || out.includes("never applied automatically") || out.includes("never disposes"));
});

test("reflect includes captured friction entries in its digest", () => {
  writeFriction(
    root,
    `---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: meta/friction.md\n---\nThe doctor error message was confusing.\n`,
  );
  runReflect([]);
  assert.ok(out.includes("FRI-001"));
  assert.ok(out.includes("doctor error message was confusing"));
});

test("reflect --json returns a versioned envelope with friction and journal coverage", () => {
  writeFriction(
    root,
    `---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: meta/friction.md\n---\nSomething was confusing.\n`,
  );
  runReflect(["--json"]);
  const parsed = JSON.parse(out);
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
  // runInit already created the current month's journal file too, so with
  // --months 2 the two most recent (by name) win, whatever "current" is.
  const allMonths = readdirSync(journalDir)
    .filter((f) => /^\d{4}-\d{2}\.md$/.test(f))
    .map((f) => f.slice(0, 7))
    .sort();
  const expected = allMonths.slice(-2);

  runReflect(["--months", "2", "--json"]);
  const parsed = JSON.parse(out);
  assert.deepEqual(parsed.journal_months.slice().sort(), expected);
});

test("reflect rejects a non-positive --months", () => {
  assert.throws(() => runReflect(["--months", "0"]), /--months must be a positive integer/);
});

test("reflect performs no writes to the repo (read-only)", () => {
  writeFriction(
    root,
    `---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: meta/friction.md\n---\nSome friction.\n`,
  );
  const before = snapshot(join(root, ".agnosgram"));
  runReflect([]);
  runReflect(["--json"]);
  const after = snapshot(join(root, ".agnosgram"));
  assert.deepEqual([...after.keys()].sort(), [...before.keys()].sort(), "reflect must not create or delete any file");
  for (const [file, mtime] of before) {
    assert.equal(after.get(file), mtime, `reflect must not modify ${file}`);
  }
});
