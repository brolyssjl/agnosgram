import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { loadConfig } from "../core/config.js";
import { runInit } from "./init.js";

let root: string;
let cwd: string;

beforeEach(() => {
  cwd = process.cwd();
  root = mkdtempSync(join(tmpdir(), "agnos-init-"));
  process.chdir(root);
});
afterEach(() => {
  process.chdir(cwd);
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
  runInit(["--json"]);
  for (const rel of SCAFFOLD) {
    assert.ok(existsSync(join(root, ".agnosgram", rel)), `missing ${rel}`);
  }
  // a journal file for the current month
  const month = new Date().toISOString().slice(0, 7);
  assert.ok(existsSync(join(root, ".agnosgram", "journal", `${month}.md`)));
});

test("init refuses to overwrite without --force", () => {
  runInit(["--adapt", "none"]);
  assert.throws(() => runInit(["--adapt", "none"]), /already exists/);
});

test("init auto-adapts detected agents", () => {
  writeFileSync(join(root, "CLAUDE.md"), "# existing\n");
  runInit([]);
  const claude = readFileSync(join(root, "CLAUDE.md"), "utf8");
  assert.ok(claude.includes(".agnosgram/MEMORY.md"));
  const config = loadConfig(root);
  assert.equal(config.adapters.claude, "on");
});

test("init --adapt none writes no adapters", () => {
  writeFileSync(join(root, "AGENTS.md"), "# existing\n");
  runInit(["--adapt", "none"]);
  const agents = readFileSync(join(root, "AGENTS.md"), "utf8");
  assert.ok(!agents.includes(".agnosgram/MEMORY.md"));
});

test("init records SDD detection into config-driven hints", () => {
  mkdirSync(join(root, "openspec"), { recursive: true });
  writeFileSync(join(root, "AGENTS.md"), "");
  runInit([]);
  const agents = readFileSync(join(root, "AGENTS.md"), "utf8");
  assert.ok(agents.includes("openspec/"));
});
