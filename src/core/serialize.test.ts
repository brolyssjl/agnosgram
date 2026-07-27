import assert from "node:assert/strict";
import { test } from "node:test";
import { isFormat, resolveFormat, serialize } from "./serialize.js";

test("isFormat accepts only json and toon", () => {
  assert.equal(isFormat("json"), true);
  assert.equal(isFormat("toon"), true);
  assert.equal(isFormat("yaml"), false);
});

test("serialize defaults to pretty JSON", () => {
  assert.equal(serialize({ a: 1 }, "json"), JSON.stringify({ a: 1 }, null, 2));
});

test("serialize routes toon through the toon encoder", () => {
  assert.equal(serialize(["a", "b"], "toon"), "[2]: a,b");
});

test("resolveFormat: no flags means human", () => {
  assert.equal(resolveFormat({}), "human");
});

test("resolveFormat: --json means json", () => {
  assert.equal(resolveFormat({ json: true }), "json");
});

test("resolveFormat: --format takes precedence over --json", () => {
  assert.equal(resolveFormat({ json: true, format: "toon" }), "toon");
});

test("resolveFormat: unknown --format throws a UserError", () => {
  assert.throws(() => resolveFormat({ format: "yaml" }), /unknown --format/);
});
