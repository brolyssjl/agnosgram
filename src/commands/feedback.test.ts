import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { fileURLToPath } from "node:url";
import { runFeedback } from "./feedback.js";
import { runInit } from "./init.js";

/** `--stdin` reads from real fd 0, so it can only be exercised by spawning
 * the built CLI with piped input - not by calling `runFeedback` in-process. */
const CLI_PATH = fileURLToPath(new URL("../../dist/cli.js", import.meta.url));

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

test("feedback rejects a --scope tag that would break the YAML flow sequence (bracket)", () => {
  assert.throws(() => runFeedback(["x", "--scope", "cli]x"]), /--scope tag "cli\]x" must contain only/);
});

test("feedback rejects a --scope tag containing a colon-space", () => {
  assert.throws(() => runFeedback(["x", "--scope", "a: b"]), /--scope tag "a: b" must contain only/);
});

test("feedback with no text throws a usage error", () => {
  assert.throws(() => runFeedback([]), /Usage: agnosgram feedback/);
});

test("feedback --stdin reads the entry text from piped input", () => {
  execFileSync(process.execPath, [CLI_PATH, "feedback", "--stdin"], {
    cwd: root,
    input: "captured over stdin, longer than a single positional arg\n",
  });
  const text = readFileSync(join(root, ".agnosgram", "meta", "friction.md"), "utf8");
  assert.ok(text.includes("captured over stdin, longer than a single positional arg"));
});

test("feedback --json prints a structured envelope and no gh command by default", () => {
  runFeedback(["json output test", "--json"]);
  const parsed = JSON.parse(out);
  assert.equal(parsed.id, "FRI-001");
  assert.equal(parsed.text, "json output test");
  assert.equal(parsed.share, null);
});

test("feedback --share prints a ready-to-run gh issue create command targeting the real repo, but never runs it", () => {
  runFeedback(["share me", "--share"]);
  assert.ok(out.includes("gh issue create"));
  assert.ok(out.includes("--repo brolyssjl/agnosgram"), "must pin --repo, or it files on whatever repo the CLI runs in");
  assert.ok(out.includes("never executes it") || out.includes("run it yourself"));
});

test("feedback --share --json includes the command, targeting the real repo, as a string field", () => {
  runFeedback(["share me", "--share", "--json"]);
  const parsed = JSON.parse(out);
  assert.ok(typeof parsed.share === "string" && parsed.share.startsWith("gh issue create"));
  assert.ok(parsed.share.includes("--repo brolyssjl/agnosgram"));
});

test("feedback writes only under .agnosgram/meta/, touching no other file", () => {
  const before = readFileSync(join(root, ".agnosgram", "state", "status.md"), "utf8");
  const beforeRoadmapExists = existsSync(join(root, "ROADMAP.md"));
  runFeedback(["isolation check"]);
  const after = readFileSync(join(root, ".agnosgram", "state", "status.md"), "utf8");
  assert.equal(before, after);
  assert.equal(existsSync(join(root, "ROADMAP.md")), beforeRoadmapExists);
});
