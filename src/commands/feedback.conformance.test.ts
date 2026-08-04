import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runCli } from "../conformance/harness.js";

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "agnos-feedback-"));
  assert.equal(runCli(["init", "--adapt", "none"], { cwd: root }).status, 0);
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

test("feedback creates meta/friction.md on first use, not before", () => {
  const file = join(root, ".agnosgram", "meta", "friction.md");
  assert.ok(!existsSync(file));
  const res = runCli(["feedback", "doctor's error message was confusing"], { cwd: root });
  assert.equal(res.status, 0);
  assert.ok(existsSync(file));
  const text = readFileSync(file, "utf8");
  assert.ok(text.includes("id: FRI-001"));
  assert.ok(text.includes("type: friction"));
  assert.ok(text.includes("doctor's error message was confusing"));
});

test("feedback allocates sequential FRI- ids", () => {
  runCli(["feedback", "one"], { cwd: root });
  runCli(["feedback", "two"], { cwd: root });
  const text = readFileSync(join(root, ".agnosgram", "meta", "friction.md"), "utf8");
  assert.ok(text.includes("id: FRI-001"));
  assert.ok(text.includes("id: FRI-002"));
});

test("feedback defaults scope to cli and confidence to medium", () => {
  runCli(["feedback", "default scope test"], { cwd: root });
  const text = readFileSync(join(root, ".agnosgram", "meta", "friction.md"), "utf8");
  assert.ok(text.includes("scope: [cli]"));
  assert.ok(text.includes("confidence: medium"));
});

test("feedback --scope and --confidence override the defaults", () => {
  runCli(["feedback", "custom", "--scope", "docs,ux", "--confidence", "high"], { cwd: root });
  const text = readFileSync(join(root, ".agnosgram", "meta", "friction.md"), "utf8");
  assert.ok(text.includes("scope: [docs, ux]"));
  assert.ok(text.includes("confidence: high"));
});

test("feedback rejects an unknown --confidence", () => {
  const res = runCli(["feedback", "x", "--confidence", "bogus"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /--confidence must be one of/);
});

test("feedback rejects a --scope tag that would break the YAML flow sequence (bracket)", () => {
  const res = runCli(["feedback", "x", "--scope", "cli]x"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /--scope tag "cli\]x" must contain only/);
});

test("feedback rejects a --scope tag containing a colon-space", () => {
  const res = runCli(["feedback", "x", "--scope", "a: b"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /--scope tag "a: b" must contain only/);
});

test("feedback with no text throws a usage error", () => {
  const res = runCli(["feedback"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /Usage: agnosgram feedback/);
});

test("feedback --stdin reads the entry text from piped input", () => {
  const res = runCli(["feedback", "--stdin"], {
    cwd: root,
    input: "captured over stdin, longer than a single positional arg\n",
  });
  assert.equal(res.status, 0);
  const text = readFileSync(join(root, ".agnosgram", "meta", "friction.md"), "utf8");
  assert.ok(text.includes("captured over stdin, longer than a single positional arg"));
});

test("feedback --json prints a structured envelope and no gh command by default", () => {
  const res = runCli(["feedback", "json output test", "--json"], { cwd: root });
  const parsed = JSON.parse(res.stdout);
  assert.equal(parsed.id, "FRI-001");
  assert.equal(parsed.text, "json output test");
  assert.equal(parsed.share, null);
});

test("feedback --share prints a ready-to-run gh issue create command targeting the real repo, but never runs it", () => {
  const res = runCli(["feedback", "share me", "--share"], { cwd: root });
  assert.ok(res.stdout.includes("gh issue create"));
  assert.ok(res.stdout.includes("--repo brolyssjl/agnosgram"), "must pin --repo, or it files on whatever repo the CLI runs in");
  assert.ok(res.stdout.includes("never executes it") || res.stdout.includes("run it yourself"));
});

test("feedback --share --json includes the command, targeting the real repo, as a string field", () => {
  const res = runCli(["feedback", "share me", "--share", "--json"], { cwd: root });
  const parsed = JSON.parse(res.stdout);
  assert.ok(typeof parsed.share === "string" && parsed.share.startsWith("gh issue create"));
  assert.ok(parsed.share.includes("--repo brolyssjl/agnosgram"));
});

test("feedback writes only under .agnosgram/meta/, touching no other file", () => {
  const before = readFileSync(join(root, ".agnosgram", "state", "status.md"), "utf8");
  const beforeRoadmapExists = existsSync(join(root, "ROADMAP.md"));
  runCli(["feedback", "isolation check"], { cwd: root });
  const after = readFileSync(join(root, ".agnosgram", "state", "status.md"), "utf8");
  assert.equal(before, after);
  assert.equal(existsSync(join(root, "ROADMAP.md")), beforeRoadmapExists);
});

test("FRI-001: feedback --scope -docs accepts a dash-leading but otherwise valid tag, not a crash", () => {
  const res = runCli(["feedback", "some text", "--scope", "-docs"], { cwd: root });
  assert.equal(res.status, 0);
  assert.ok(!/at Object|node:internal|ERR_PARSE_ARGS/.test(res.stderr));
  const text = readFileSync(join(root, ".agnosgram", "meta", "friction.md"), "utf8");
  assert.ok(text.includes("scope: [-docs]"));
});
