import assert from "node:assert/strict";
import { test } from "node:test";
import { SDD_FRAMEWORKS } from "../core/detect.js";
import { buildPointerBody } from "./index.js";

test("every SDD_FRAMEWORKS entry produces a hint line with a real matched path", () => {
  for (const framework of SDD_FRAMEWORKS) {
    const body = buildPointerBody([{ key: framework.key, matchedPath: `${framework.key}/` }]);
    assert.ok(
      body.includes(`${framework.key}/`),
      `expected a hint line for "${framework.key}" pointing at its matched path`,
    );
    assert.ok(body.includes(framework.name), `expected a hint line naming "${framework.name}"`);
  }
});

test("every SDD_FRAMEWORKS entry produces a hint line with no matched path (forced on, undetected)", () => {
  for (const framework of SDD_FRAMEWORKS) {
    const body = buildPointerBody([{ key: framework.key, matchedPath: null }]);
    assert.ok(body.includes(framework.name), `expected a hint line naming "${framework.name}"`);
    assert.ok(body.includes("no directory detected on disk"));
    assert.ok(!body.includes(`${framework.key}/`), "must not fabricate a path when none was detected");
  }
});

test("an unknown SDD key produces no hint line instead of crashing", () => {
  const body = buildPointerBody([{ key: "not-a-real-framework", matchedPath: "whatever/" }]);
  assert.ok(!body.includes("Coexisting tools detected"));
});
