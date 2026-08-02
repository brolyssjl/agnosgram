import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runCli } from "../conformance/harness.js";

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "agnos-init-"));
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

const SCAFFOLD = [
  "MEMORY.md",
  "config.yml",
  "state/status.md",
  "context/architecture.md",
  "context/stack.md",
  "context/domain.md",
  "decisions/README.md",
  "lessons/pitfalls.md",
  "lessons/conventions.md",
];

test("init scaffolds the full store", () => {
  const res = runCli(["init", "--json"], { cwd: root });
  assert.equal(res.status, 0);
  for (const rel of SCAFFOLD) {
    assert.ok(existsSync(join(root, ".agnosgram", rel)), `missing ${rel}`);
  }
  // a journal file for the current month
  const month = new Date().toISOString().slice(0, 7);
  assert.ok(existsSync(join(root, ".agnosgram", "journal", `${month}.md`)));
});

test("init refuses to overwrite without --force", () => {
  assert.equal(runCli(["init", "--adapt", "none"], { cwd: root }).status, 0);
  const second = runCli(["init", "--adapt", "none"], { cwd: root });
  assert.notEqual(second.status, 0);
  assert.match(second.stderr, /already exists/);
});

test("init auto-adapts detected agents", () => {
  writeFileSync(join(root, "CLAUDE.md"), "# existing\n");
  assert.equal(runCli(["init"], { cwd: root }).status, 0);
  const claude = readFileSync(join(root, "CLAUDE.md"), "utf8");
  assert.ok(claude.includes(".agnosgram/MEMORY.md"));
  const config = readFileSync(join(root, ".agnosgram", "config.yml"), "utf8");
  assert.match(config, /^\s*claude:\s*on\s*$/m);
});

test("init --adapt none writes no adapters", () => {
  writeFileSync(join(root, "AGENTS.md"), "# existing\n");
  assert.equal(runCli(["init", "--adapt", "none"], { cwd: root }).status, 0);
  const agents = readFileSync(join(root, "AGENTS.md"), "utf8");
  assert.ok(!agents.includes(".agnosgram/MEMORY.md"));
});

test("init does not scaffold meta/ - it is opt-in via `feedback` on first use", () => {
  assert.equal(runCli(["init", "--adapt", "none"], { cwd: root }).status, 0);
  assert.ok(!existsSync(join(root, ".agnosgram", "meta")));
});

test("init records SDD detection into config-driven hints", () => {
  mkdirSync(join(root, "openspec"), { recursive: true });
  writeFileSync(join(root, "AGENTS.md"), "");
  assert.equal(runCli(["init"], { cwd: root }).status, 0);
  const agents = readFileSync(join(root, "AGENTS.md"), "utf8");
  assert.ok(agents.includes("openspec/"));
});

test("FRI-001: init --adapt -none gives the existing validation error, not a raw parseArgs crash", () => {
  const res = runCli(["init", "--adapt", "-none"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /Unknown adapter\(s\) in --adapt: -none/);
  assert.ok(!/at Object|node:internal|ERR_PARSE_ARGS/.test(res.stderr));
});
