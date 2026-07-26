import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runBootstrap } from "./bootstrap.js";
import { runInit } from "./init.js";

let root: string;
let cwd: string;
let out: string;
let origWrite: typeof process.stdout.write;

beforeEach(() => {
  cwd = process.cwd();
  root = mkdtempSync(join(tmpdir(), "agnos-bootstrap-"));
  process.chdir(root);
  writeFileSync(join(root, "package.json"), `{ "name": "demo" }\n`);
  runInit(["--adapt", "none"]);
  out = "";
  origWrite = process.stdout.write.bind(process.stdout);
  process.stdout.write = ((chunk: string | Uint8Array) => {
    out += chunk.toString();
    return true;
  }) as typeof process.stdout.write;
});
afterEach(() => {
  process.stdout.write = origWrite;
  process.chdir(cwd);
  rmSync(root, { recursive: true, force: true });
});

test("bootstrap emits a prompt targeting context files and detected stack", () => {
  runBootstrap([]);
  assert.ok(out.includes("bootstrap task"));
  assert.ok(out.includes("context/architecture.md"));
  assert.ok(out.includes("context/domain.md"));
  assert.ok(out.includes("package.json")); // top-level entry surfaced
  assert.ok(out.includes("Node")); // stack signal surfaced
});
