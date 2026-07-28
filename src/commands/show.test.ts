import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runInit } from "./init.js";
import { runShow } from "./show.js";

let root: string;
let cwd: string;
let out: string;
let errOut: string;
let origOut: typeof process.stdout.write;
let origErr: typeof process.stderr.write;

beforeEach(() => {
  cwd = process.cwd();
  root = mkdtempSync(join(tmpdir(), "agnos-show-"));
  process.chdir(root);
  runInit(["--adapt", "none"]);

  writeFileSync(
    join(root, ".agnosgram", "lessons", "pitfalls.md"),
    `# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [core, tooling]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nDo not use require().\n`,
  );
  writeFileSync(
    join(root, ".agnosgram", "lessons", "conventions.md"),
    `# Conventions\n\n---\nid: CON-001\ntype: convention\nscope: [tooling]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nAlways use node built-ins.\n`,
  );

  out = "";
  errOut = "";
  origOut = process.stdout.write.bind(process.stdout);
  origErr = process.stderr.write.bind(process.stderr);
  process.stdout.write = ((chunk: string | Uint8Array) => {
    out += chunk.toString();
    return true;
  }) as typeof process.stdout.write;
  process.stderr.write = ((chunk: string | Uint8Array) => {
    errOut += chunk.toString();
    return true;
  }) as typeof process.stderr.write;
});
afterEach(() => {
  process.stdout.write = origOut;
  process.stderr.write = origErr;
  process.chdir(cwd);
  rmSync(root, { recursive: true, force: true });
  process.exitCode = 0;
});

test("show matches an exact record id first", () => {
  runShow(["LES-001"]);
  assert.ok(out.includes("id: LES-001"));
  assert.ok(out.includes("Do not use require()."));
  assert.ok(!out.includes("CON-001"));
});

test("show matches a case-insensitive scope tag when no id matches", () => {
  runShow(["Tooling"]);
  assert.ok(out.includes("LES-001"));
  assert.ok(out.includes("CON-001"));
});

test("show matches a type name as a last resort", () => {
  runShow(["convention"]);
  assert.ok(out.includes("CON-001"));
  assert.ok(!out.includes("LES-001"));
});

test("--type filters the pool before matching", () => {
  runShow(["tooling", "--type", "convention"]);
  assert.ok(out.includes("CON-001"));
  assert.ok(!out.includes("LES-001"));
});

test("no match exits 1 and hints known scopes on stderr", () => {
  runShow(["nonexistent-topic"]);
  assert.equal(process.exitCode, 1);
  assert.ok(errOut.includes("No records match"));
  assert.ok(errOut.includes("core"));
});

test("--format json prints a uniform flat array", () => {
  runShow(["LES-001", "--format", "json"]);
  const parsed = JSON.parse(out);
  assert.equal(Array.isArray(parsed), true);
  assert.equal(parsed[0].id, "LES-001");
  assert.equal(parsed[0].scope, "core,tooling");
});

test("--format toon renders a tabular header for multiple matches", () => {
  runShow(["tooling", "--format", "toon"]);
  assert.ok(out.startsWith("[2]{"));
});

test("missing topic argument throws a usage error", () => {
  assert.throws(() => runShow([]), /Usage: agnosgram show/);
});

test("invalid --type throws a usage error", () => {
  assert.throws(() => runShow(["tooling", "--type", "bogus"]), /--type must be one of/);
});

test("--format json on no match prints an empty JSON array to stdout and still exits 1", () => {
  runShow(["nonexistent-topic", "--format", "json"]);
  assert.equal(process.exitCode, 1);
  const parsed = JSON.parse(out);
  assert.deepEqual(parsed, []);
  // Structured mode never writes the human hints to stderr.
  assert.equal(errOut, "");
});

test("--format toon on no match prints an empty TOON array to stdout and still exits 1", () => {
  runShow(["nonexistent-topic", "--format", "toon"]);
  assert.equal(process.exitCode, 1);
  assert.ok(out.trim().length > 0, "expected a non-empty structured payload on stdout");
  assert.equal(errOut, "");
});
