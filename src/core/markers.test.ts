import assert from "node:assert/strict";
import { test } from "node:test";
import { END_MARKER, hasManagedBlock, START_MARKER, upsertManagedBlock } from "./markers.js";

test("appends a managed block to user content, preserving it", () => {
  const existing = "# My rules\n\nAlways be nice.\n";
  const out = upsertManagedBlock(existing, "BODY");
  assert.ok(out.startsWith("# My rules\n\nAlways be nice."));
  assert.ok(out.includes(START_MARKER));
  assert.ok(out.includes(END_MARKER));
  assert.ok(out.includes("BODY"));
});

test("is idempotent — running twice yields identical output", () => {
  const existing = "# My rules\n\nkeep me\n";
  const once = upsertManagedBlock(existing, "BODY v1");
  const twice = upsertManagedBlock(once, "BODY v1");
  assert.equal(once, twice);
});

test("replaces the block body without touching surrounding user content", () => {
  const existing = "top\n";
  const v1 = upsertManagedBlock(existing, "OLD");
  const v2 = upsertManagedBlock(v1 + "\nuser added this later\n", "NEW");
  assert.ok(v2.includes("NEW"));
  assert.ok(!v2.includes("OLD"));
  assert.ok(v2.startsWith("top"));
  assert.ok(v2.includes("user added this later"));
});

test("collapses accidental duplicate blocks into one", () => {
  const block = `${START_MARKER}\nx\n${END_MARKER}`;
  const existing = `a\n\n${block}\n\nb\n\n${block}\n`;
  const out = upsertManagedBlock(existing, "ONE");
  const count = out.split(START_MARKER).length - 1;
  assert.equal(count, 1);
  assert.ok(out.includes("a"));
  assert.ok(out.includes("b"));
});

test("handles empty input", () => {
  const out = upsertManagedBlock("", "BODY");
  assert.ok(hasManagedBlock(out));
});
