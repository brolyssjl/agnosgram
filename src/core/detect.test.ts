import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { detectSdd } from "./detect.js";

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "agnos-detect-"));
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

test("detectSdd reports a trailing slash when the marker is a real directory", () => {
  mkdirSync(join(root, "openspec"), { recursive: true });
  const [hit] = detectSdd(root);
  assert.equal(hit?.key, "openspec");
  assert.equal(hit?.matchedPath, "openspec/");
});

test("detectSdd never claims a directory when the marker is a plain file", () => {
  writeFileSync(join(root, "openspec"), "not actually a directory");
  const [hit] = detectSdd(root);
  assert.equal(hit?.key, "openspec");
  assert.equal(hit?.matchedPath, "openspec");
  assert.ok(!hit?.matchedPath.endsWith("/"));
});

test("no markers present means no detections", () => {
  assert.deepEqual(detectSdd(root), []);
});
