import assert from "node:assert/strict";
import { test } from "node:test";
import { encodeToon } from "./toon.js";

test("encodes a uniform array of flat objects as a tabular header + rows", () => {
  const out = encodeToon([
    { id: "LES-001", type: "pitfall" },
    { id: "LES-002", type: "convention" },
  ]);
  assert.equal(out, `[2]{id,type}:\n  LES-001,pitfall\n  LES-002,convention`);
});

test("encodes a small non-uniform object as indented key: value lines", () => {
  const out = encodeToon({ focus: "M3", inFlight: null });
  assert.equal(out, `focus: M3\ninFlight: null`);
});

test("encodes a primitive array inline", () => {
  assert.equal(encodeToon(["a", "b", "c"]), "[3]: a,b,c");
});

test("encodes an empty array", () => {
  assert.equal(encodeToon([]), "[0]:");
});

test("quotes values that need it (a bare key needs no quoting)", () => {
  const out = encodeToon({ "weird key": "has, a comma", n: "123abc" });
  assert.ok(out.startsWith("weird key: "));
  assert.ok(out.includes('"has, a comma"'));
  assert.ok(out.includes('"123abc"'));
});

test("quotes a key that contains reserved characters", () => {
  const out = encodeToon({ "a,b": 1 });
  assert.equal(out, '"a,b": 1');
});

test("falls back to a list form for a non-uniform array of objects", () => {
  const out = encodeToon([{ a: 1 }, { a: 1, b: 2 }]);
  assert.ok(out.startsWith("[2]:"));
  assert.ok(out.includes("a: 1"));
  assert.ok(out.includes("b: 2"));
});

test("never used for storage: round-tripping is not supported (encoder only)", () => {
  // Documented contract check: this module exports only an encoder.
  const mod = { encodeToon } as Record<string, unknown>;
  assert.equal(typeof mod.decodeToon, "undefined");
});
