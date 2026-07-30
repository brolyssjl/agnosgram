import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runFeedback } from "./feedback.js";
import { runInit } from "./init.js";

let root: string;
let cwd: string;
let out: string;
let origWrite: typeof process.stdout.write;

beforeEach(() => {
  cwd = process.cwd();
  root = mkdtempSync(join(tmpdir(), "agnos-feedback-"));
  process.chdir(root);
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
  process.exitCode = 0;
});

test("feedback creates meta/friction.md on first use, not before", () => {
  const file = join(root, ".agnosgram", "meta", "friction.md");
  assert.ok(!existsSync(file));
  runFeedback(["doctor's error message was confusing"]);
  assert.ok(existsSync(file));
  const text = readFileSync(file, "utf8");
  assert.ok(text.includes("id: FRI-001"));
  assert.ok(text.includes("type: friction"));
  assert.ok(text.includes("doctor's error message was confusing"));
});

test("feedback allocates sequential FRI- ids", () => {
  runFeedback(["one"]);
  runFeedback(["two"]);
  const text = readFileSync(join(root, ".agnosgram", "meta", "friction.md"), "utf8");
  assert.ok(text.includes("id: FRI-001"));
  assert.ok(text.includes("id: FRI-002"));
});

test("feedback defaults scope to cli and confidence to medium", () => {
  runFeedback(["default scope test"]);
  const text = readFileSync(join(root, ".agnosgram", "meta", "friction.md"), "utf8");
  assert.ok(text.includes("scope: [cli]"));
  assert.ok(text.includes("confidence: medium"));
});

test("feedback --scope and --confidence override the defaults", () => {
  runFeedback(["custom", "--scope", "docs,ux", "--confidence", "high"]);
  const text = readFileSync(join(root, ".agnosgram", "meta", "friction.md"), "utf8");
  assert.ok(text.includes("scope: [docs, ux]"));
  assert.ok(text.includes("confidence: high"));
});

test("feedback rejects an unknown --confidence", () => {
  assert.throws(() => runFeedback(["x", "--confidence", "bogus"]), /--confidence must be one of/);
});

test("feedback with no text throws a usage error", () => {
  assert.throws(() => runFeedback([]), /Usage: agnosgram feedback/);
});

test("feedback --json prints a structured envelope and no gh command by default", () => {
  runFeedback(["json output test", "--json"]);
  const parsed = JSON.parse(out);
  assert.equal(parsed.id, "FRI-001");
  assert.equal(parsed.text, "json output test");
  assert.equal(parsed.share, null);
});

test("feedback --share prints a ready-to-run gh issue create command but never runs it", () => {
  runFeedback(["share me", "--share"]);
  assert.ok(out.includes("gh issue create"));
  assert.ok(out.includes("never executes it") || out.includes("run it yourself"));
});

test("feedback --share --json includes the command as a string field", () => {
  runFeedback(["share me", "--share", "--json"]);
  const parsed = JSON.parse(out);
  assert.ok(typeof parsed.share === "string" && parsed.share.startsWith("gh issue create"));
});

test("feedback writes only under .agnosgram/meta/, touching no other file", () => {
  const before = readFileSync(join(root, ".agnosgram", "state", "status.md"), "utf8");
  const beforeRoadmapExists = existsSync(join(root, "ROADMAP.md"));
  runFeedback(["isolation check"]);
  const after = readFileSync(join(root, ".agnosgram", "state", "status.md"), "utf8");
  assert.equal(before, after);
  assert.equal(existsSync(join(root, "ROADMAP.md")), beforeRoadmapExists);
});
