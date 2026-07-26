import assert from "node:assert/strict";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runDistill } from "./distill.js";
import { runInit } from "./init.js";

let root: string;
let cwd: string;
let out: string;
let origWrite: typeof process.stdout.write;

beforeEach(() => {
  cwd = process.cwd();
  root = mkdtempSync(join(tmpdir(), "agnos-distill-"));
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

test("distill emits a compaction prompt naming the schema and rules", () => {
  runDistill([]);
  assert.ok(out.includes("distillation task"));
  assert.ok(out.includes("supersedes"));
  assert.ok(out.includes("Merge, do not append"));
  assert.ok(out.includes("journal/"));
});

test("distill --validate passes a well-formed file", () => {
  const file = join(root, ".agnosgram", "lessons", "pitfalls.md");
  writeFileSync(
    file,
    `# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nA valid lesson.\n`,
  );
  runDistill(["--validate", "lessons/pitfalls.md"]);
  assert.ok(out.includes("valid"));
  assert.notEqual(process.exitCode, 1);
});

test("distill --validate fails a schema-broken file with a non-zero exit", () => {
  const file = join(root, ".agnosgram", "lessons", "pitfalls.md");
  writeFileSync(file, `# Pitfalls\n\n---\nid: bad\ntype: pitfall\n---\nbroken\n`);
  runDistill(["--validate", "lessons/pitfalls.md"]);
  assert.equal(process.exitCode, 1);
  assert.ok(out.includes("error"));
});

test("distill --archive moves a journal month into archive/", () => {
  const month = new Date().toISOString().slice(0, 7);
  runDistill(["--archive", month]);
  assert.ok(!existsSync(join(root, ".agnosgram", "journal", `${month}.md`)));
  assert.ok(existsSync(join(root, ".agnosgram", "journal", "archive", `${month}.md`)));
});

test("distill --archive rejects a bad month argument", () => {
  assert.throws(() => runDistill(["--archive", "nope"]), /YYYY-MM/);
});

test("distill --validate fails frontmatter holding a block scalar", () => {
  const file = join(root, ".agnosgram", "lessons", "pitfalls.md");
  writeFileSync(
    file,
    `# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: |\n  journal/2026-07.md\n  plus trailing junk\n---\nBody.\n`,
  );
  runDistill(["--validate", "lessons/pitfalls.md"]);
  assert.equal(process.exitCode, 1);
  assert.ok(out.includes("frontmatter.parse") || out.includes("block scalars"), out);
});
