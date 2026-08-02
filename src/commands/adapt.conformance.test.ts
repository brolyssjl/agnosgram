import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, symlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runCli } from "../conformance/harness.js";

let cwd: string;

beforeEach(() => {
  cwd = mkdtempSync(join(tmpdir(), "agnos-adapt-conf-"));
  assert.equal(runCli(["init", "--adapt", "none"], { cwd }).status, 0);
});
afterEach(() => {
  rmSync(cwd, { recursive: true, force: true });
});

test("FRI-001: a dash-leading positional gives a clean UserError, not a raw parseArgs crash", () => {
  const res = runCli(["adapt", "-claude"], { cwd });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /Unknown option/);
  assert.ok(!/at Object|node:internal|ERR_PARSE_ARGS/.test(res.stderr));
});

test("FRI-003: CLAUDE.md symlinked to AGENTS.md reports one honest write, not two", () => {
  symlinkSync("AGENTS.md", join(cwd, "CLAUDE.md"));

  const first = runCli(["adapt", "claude", "agents"], { cwd });
  assert.equal(first.status, 0);
  assert.match(first.stdout, /CLAUDE\.md -> AGENTS\.md \(symlink\), managed block written once/);
  // Never report the two adapters as if they were independent files.
  assert.ok(!/created\s+CLAUDE\.md\s+\(Claude Code\)/.test(first.stdout));
  assert.ok(!/created\s+AGENTS\.md\s+\(AGENTS\.md\)/.test(first.stdout));

  const agentsContent = readFileSync(join(cwd, "AGENTS.md"), "utf8");
  assert.equal((agentsContent.match(/agnosgram:start/g) ?? []).length, 1);

  // CON-002: running twice is identical.
  const second = runCli(["adapt", "claude", "agents"], { cwd });
  assert.equal(second.status, 0);
  assert.match(second.stdout, /unchanged\s+CLAUDE\.md -> AGENTS\.md \(symlink\)/);
  assert.equal(readFileSync(join(cwd, "AGENTS.md"), "utf8"), agentsContent);
});

test("FRI-003: the reverse symlink direction (AGENTS.md -> CLAUDE.md) is also honest", () => {
  symlinkSync("CLAUDE.md", join(cwd, "AGENTS.md"));

  const res = runCli(["adapt", "claude", "agents"], { cwd });
  assert.equal(res.status, 0);
  assert.match(res.stdout, /AGENTS\.md -> CLAUDE\.md \(symlink\), managed block written once/);
});

test("FRI-003: --json reports the write once, with the symlinked adapter as an alias", () => {
  symlinkSync("AGENTS.md", join(cwd, "CLAUDE.md"));

  const res = runCli(["adapt", "claude", "agents", "--json"], { cwd });
  assert.equal(res.status, 0);
  const parsed = JSON.parse(res.stdout);
  assert.equal(parsed.adapters.length, 1);
  assert.equal(parsed.adapters[0].path, "AGENTS.md");
  assert.deepEqual(parsed.adapters[0].symlinkAliases, [{ adapter: "claude", path: "CLAUDE.md" }]);
});

test("non-symlinked CLAUDE.md and AGENTS.md are still reported independently", () => {
  const res = runCli(["adapt", "claude", "agents"], { cwd });
  assert.equal(res.status, 0);
  assert.match(res.stdout, /created\s+CLAUDE\.md\s+\(Claude Code\)/);
  assert.match(res.stdout, /created\s+AGENTS\.md\s+\(AGENTS\.md\)/);
});
