import assert from "node:assert/strict";
import { test } from "node:test";
import { estimateTokens } from "./tokens.js";

test("empty text is zero tokens", () => {
  assert.equal(estimateTokens(""), 0);
  assert.equal(estimateTokens("   \n  "), 0);
});

test("estimate grows monotonically with content", () => {
  const small = estimateTokens("hello world");
  const big = estimateTokens("hello world ".repeat(50));
  assert.ok(big > small);
});

test("estimate is in a sane range for a paragraph", () => {
  // ~40 words; a real tokenizer lands near 50-60 tokens. Heuristic must be close-ish.
  const text = "The quick brown fox jumps over the lazy dog. ".repeat(5);
  const t = estimateTokens(text);
  assert.ok(t > 40 && t < 120, `unexpected estimate ${t}`);
});
