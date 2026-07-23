import assert from "node:assert/strict";
import { test } from "node:test";
import { extractRecords, validateRecord } from "./frontmatter.js";

const VALID = `# Pitfalls

intro text that is not a record

---
id: LES-001
type: pitfall
scope: [core, tooling]
confidence: high
created: 2026-07-21
last_verified: 2026-07-21
source: journal/2026-07.md
---
Do not use require() in this ESM package.

---
id: LES-002
type: convention
scope: [tooling]
confidence: medium
created: 2026-07-21
last_verified: 2026-07-21
source: journal/2026-07.md
supersedes: LES-000
---
Prefer node built-ins.
`;

test("extractRecords finds every record and skips intro text", () => {
  const recs = extractRecords(VALID);
  assert.equal(recs.length, 2);
  assert.equal(recs[0]!.data.id, "LES-001");
  assert.ok(recs[0]!.body.includes("require()"));
  assert.equal(recs[1]!.data.id, "LES-002");
});

test("extractRecords ignores frontmatter inside HTML comments", () => {
  const templated = `# Pitfalls

<!-- Example:

---
id: LES-001
type: pitfall
---
example body

-->
`;
  assert.equal(extractRecords(templated).length, 0);
});

test("extractRecords ignores frontmatter inside fenced code blocks", () => {
  const doc = "# Decisions\n\n```yaml\n---\nid: DEC-0001\n---\n```\n";
  assert.equal(extractRecords(doc).length, 0);
});

test("validateRecord accepts a well-formed record", () => {
  const [rec] = extractRecords(VALID);
  const { frontmatter, issues } = validateRecord(rec!);
  assert.equal(issues.filter((i) => i.level === "error").length, 0);
  assert.ok(frontmatter);
  assert.deepEqual(frontmatter!.scope, ["core", "tooling"]);
});

test("validateRecord normalizes a single supersedes id to a list", () => {
  const recs = extractRecords(VALID);
  const { frontmatter } = validateRecord(recs[1]!);
  assert.deepEqual(frontmatter!.supersedes, ["LES-000"]);
});

test("validateRecord flags missing required fields", () => {
  const [rec] = extractRecords(`---\ntype: pitfall\n---\nbody\n`);
  const { frontmatter, issues } = validateRecord(rec!);
  assert.equal(frontmatter, undefined);
  const codes = issues.map((i) => i.code);
  assert.ok(codes.includes("id.missing"));
  assert.ok(codes.includes("scope.missing"));
  assert.ok(codes.includes("confidence.missing"));
  assert.ok(codes.includes("created.missing"));
});

test("validateRecord rejects bad id, type, confidence, and date", () => {
  const [rec] = extractRecords(
    `---\nid: bad_id\ntype: rumor\nscope: [x]\nconfidence: maybe\ncreated: 2026-13-40\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nbody\n`,
  );
  const codes = validateRecord(rec!).issues.map((i) => i.code);
  assert.ok(codes.includes("id.format"));
  assert.ok(codes.includes("type.unknown"));
  assert.ok(codes.includes("confidence.unknown"));
  assert.ok(codes.includes("created.format"));
});

test("validateRecord warns when last_verified precedes created", () => {
  const [rec] = extractRecords(
    `---\nid: LES-009\ntype: pitfall\nscope: [x]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-01\nsource: journal/2026-07.md\n---\nbody\n`,
  );
  const codes = validateRecord(rec!).issues.map((i) => i.code);
  assert.ok(codes.includes("date.order"));
});
