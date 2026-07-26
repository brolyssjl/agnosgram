import assert from "node:assert/strict";
import { test } from "node:test";
import { parseFreshnessTable } from "./freshness.js";

test("parses rows from a MEMORY.md freshness table", () => {
  const md = `## Freshness
| File | Last verified | Budget |
|------|---------------|--------|
| state/status.md | 2026-07-21 | 400 tokens |
| context/architecture.md | 2026-07-20 | 1500 tokens |
`;
  const rows = parseFreshnessTable(md);
  assert.equal(rows.length, 2);
  assert.deepEqual(rows[0], { file: "state/status.md", lastVerified: "2026-07-21", budget: 400 });
  assert.equal(rows[1]!.budget, 1500);
});

test("ignores the header and separator rows", () => {
  const md = "| File | Last verified | Budget |\n|---|---|---|\n";
  assert.deepEqual(parseFreshnessTable(md), []);
});
