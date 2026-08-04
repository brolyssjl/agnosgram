import assert from "node:assert/strict";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runCli } from "../conformance/harness.js";

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "agnos-distill-"));
  assert.equal(runCli(["init", "--adapt", "none"], { cwd: root }).status, 0);
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

test("distill emits a compaction prompt naming the schema and rules", () => {
  const res = runCli(["distill"], { cwd: root });
  assert.equal(res.status, 0);
  assert.ok(res.stdout.includes("distillation task"));
  assert.ok(res.stdout.includes("supersedes"));
  assert.ok(res.stdout.includes("Merge, do not append"));
  assert.ok(res.stdout.includes("journal/"));
});

test("distill --validate passes a well-formed file", () => {
  const file = join(root, ".agnosgram", "lessons", "pitfalls.md");
  writeFileSync(
    file,
    `# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nA valid lesson.\n`,
  );
  const res = runCli(["distill", "--validate", "lessons/pitfalls.md"], { cwd: root });
  assert.ok(res.stdout.includes("valid"));
  assert.notEqual(res.status, 1);
});

test("distill --validate fails a schema-broken file with a non-zero exit", () => {
  const file = join(root, ".agnosgram", "lessons", "pitfalls.md");
  writeFileSync(file, `# Pitfalls\n\n---\nid: bad\ntype: pitfall\n---\nbroken\n`);
  const res = runCli(["distill", "--validate", "lessons/pitfalls.md"], { cwd: root });
  assert.equal(res.status, 1);
  assert.ok(res.stdout.includes("error"));
});

test("distill --archive moves a journal month into archive/", () => {
  const month = new Date().toISOString().slice(0, 7);
  const res = runCli(["distill", "--archive", month], { cwd: root });
  assert.equal(res.status, 0);
  assert.ok(!existsSync(join(root, ".agnosgram", "journal", `${month}.md`)));
  assert.ok(existsSync(join(root, ".agnosgram", "journal", "archive", `${month}.md`)));
});

test("distill --archive rejects a bad month argument", () => {
  const res = runCli(["distill", "--archive", "nope"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /YYYY-MM/);
});

test("FRI-001: distill --archive -2026-08 gives the existing validation error, not a raw parseArgs crash", () => {
  const res = runCli(["distill", "--archive", "-2026-08"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /--archive expects a YYYY-MM month, got "-2026-08"/);
  assert.ok(!/at Object|node:internal|ERR_PARSE_ARGS/.test(res.stderr));
});

test("distill --validate fails frontmatter holding a block scalar", () => {
  const file = join(root, ".agnosgram", "lessons", "pitfalls.md");
  writeFileSync(
    file,
    `# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: |\n  journal/2026-07.md\n  plus trailing junk\n---\nBody.\n`,
  );
  const res = runCli(["distill", "--validate", "lessons/pitfalls.md"], { cwd: root });
  assert.equal(res.status, 1);
  assert.ok(res.stdout.includes("frontmatter.parse") || res.stdout.includes("block scalars"), res.stdout);
});
