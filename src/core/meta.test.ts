import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { extractRecords } from "./frontmatter.js";
import { FRICTION_FILE, KNOWN_META_TYPES, loadFrictionRecords, validateFrictionRecord } from "./meta.js";

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "agnos-meta-"));
  mkdirSync(join(root, ".agnosgram", "meta"), { recursive: true });
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

test("KNOWN_META_TYPES only allows friction today", () => {
  assert.deepEqual(KNOWN_META_TYPES, ["friction"]);
});

test("validateFrictionRecord accepts a well-formed friction entry", () => {
  const text = `---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: meta/friction.md\n---\nThe doctor error message was confusing.\n`;
  const [raw] = extractRecords(text);
  const { frontmatter, issues } = validateFrictionRecord(raw!);
  assert.equal(issues.length, 0);
  assert.equal(frontmatter?.id, "FRI-001");
  assert.equal(frontmatter?.type, "friction");
});

test("validateFrictionRecord rejects a lessons/decisions type", () => {
  const text = `---\nid: FRI-001\ntype: pitfall\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: meta/friction.md\n---\nWrong type for this namespace.\n`;
  const [raw] = extractRecords(text);
  const { frontmatter, issues } = validateFrictionRecord(raw!);
  assert.equal(frontmatter, undefined);
  assert.ok(issues.some((i) => i.code === "type.unknown"));
});

test("loadFrictionRecords reads only meta/friction.md and skips invalid entries", () => {
  writeFileSync(
    join(root, ".agnosgram", "meta", "friction.md"),
    `# Friction\n\n---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: ${FRICTION_FILE}\n---\nConfusing error message.\n\n---\nid: bad\ntype: friction\n---\nbroken\n`,
  );
  const records = loadFrictionRecords(root);
  assert.equal(records.length, 1);
  assert.equal(records[0]!.frontmatter.id, "FRI-001");
});
