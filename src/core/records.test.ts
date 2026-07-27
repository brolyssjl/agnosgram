import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runInit } from "../commands/init.js";
import { allScopes, loadRecords } from "./records.js";

let root: string;
let cwd: string;

beforeEach(() => {
  cwd = process.cwd();
  root = mkdtempSync(join(tmpdir(), "agnos-records-"));
  process.chdir(root);
  runInit(["--adapt", "none"]);
});
afterEach(() => {
  process.chdir(cwd);
  rmSync(root, { recursive: true, force: true });
});

function writePitfalls(body: string): void {
  writeFileSync(join(root, ".agnosgram", "lessons", "pitfalls.md"), `# Pitfalls\n\n${body}`);
}

test("loadRecords returns valid records with their frontmatter and location", () => {
  writePitfalls(
    `---\nid: LES-001\ntype: pitfall\nscope: [core, tooling]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nDo not do X.\n`,
  );
  const records = loadRecords(root);
  const found = records.find((r) => r.frontmatter.id === "LES-001");
  assert.ok(found, "expected LES-001 to be indexed");
  assert.equal(found!.frontmatter.type, "pitfall");
  assert.deepEqual(found!.frontmatter.scope, ["core", "tooling"]);
  assert.equal(found!.storeRel, "lessons/pitfalls.md");
  assert.equal(found!.body, "Do not do X.");
});

test("loadRecords silently skips schema-invalid records", () => {
  writePitfalls(`---\nid: bad\ntype: rumor\n---\nbroken record\n`);
  const records = loadRecords(root);
  assert.equal(records.some((r) => r.body === "broken record"), false);
});

test("allScopes collects distinct scope tags across the store, sorted", () => {
  writePitfalls(
    `---\nid: LES-001\ntype: pitfall\nscope: [backend, tooling]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nA.\n\n` +
      `---\nid: LES-002\ntype: pitfall\nscope: [auth]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nB.\n`,
  );
  assert.deepEqual(allScopes(root), ["auth", "backend", "tooling"]);
});
