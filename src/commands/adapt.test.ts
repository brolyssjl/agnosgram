import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { ADAPTERS } from "../adapters/index.js";
import { defaultConfig } from "../core/config.js";
import { UserError } from "../core/output.js";
import { applyAdapter, resolveSddHints, runAdapt } from "./adapt.js";
import { runInit } from "./init.js";

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "agnos-adapt-"));
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

test("applyAdapter creates CLAUDE.md and is idempotent", () => {
  const first = applyAdapter(root, ADAPTERS.claude!, []);
  assert.equal(first.action, "created");
  const contentA = readFileSync(join(root, "CLAUDE.md"), "utf8");

  const second = applyAdapter(root, ADAPTERS.claude!, []);
  assert.equal(second.action, "unchanged");
  const contentB = readFileSync(join(root, "CLAUDE.md"), "utf8");
  assert.equal(contentA, contentB);
});

test("applyAdapter preserves user content outside the markers", () => {
  const userText = "# House rules\n\nBe excellent to each other.\n";
  writeFileSync(join(root, "CLAUDE.md"), userText);

  applyAdapter(root, ADAPTERS.claude!, []);
  const out = readFileSync(join(root, "CLAUDE.md"), "utf8");
  assert.ok(out.startsWith("# House rules\n\nBe excellent to each other."));
  assert.ok(out.includes(".agnosgram/MEMORY.md"));

  // Re-running after the user edits above the block must not disturb their text.
  const edited = "# House rules v2\n\nNew note.\n" + out.slice(userText.length);
  writeFileSync(join(root, "CLAUDE.md"), edited);
  applyAdapter(root, ADAPTERS.claude!, []);
  const out2 = readFileSync(join(root, "CLAUDE.md"), "utf8");
  assert.ok(out2.startsWith("# House rules v2\n\nNew note."));
});

test("cursor adapter writes a dedicated .mdc file with frontmatter", () => {
  const res = applyAdapter(root, ADAPTERS.cursor!, []);
  assert.equal(res.action, "created");
  const file = join(root, ".cursor/rules/agnosgram.mdc");
  assert.ok(existsSync(file));
  const content = readFileSync(file, "utf8");
  assert.ok(content.includes("alwaysApply: true"));
  assert.ok(content.includes(".agnosgram/MEMORY.md"));
});

test("SDD hints appear in the pointer body when passed", () => {
  applyAdapter(root, ADAPTERS.agents!, [{ key: "openspec", matchedPath: "openspec/" }]);
  const out = readFileSync(join(root, "AGENTS.md"), "utf8");
  assert.ok(out.includes("openspec/"));
});

test("a forced-on SDD hint with no detected directory never fabricates a path", () => {
  applyAdapter(root, ADAPTERS.agents!, [{ key: "openspec", matchedPath: null }]);
  const out = readFileSync(join(root, "AGENTS.md"), "utf8");
  assert.ok(out.includes("no directory detected on disk"));
  assert.ok(!out.includes("openspec/"));
});

test("resolveSddHints reports matchedPath: null when a framework is forced on but never detected", () => {
  const config = defaultConfig();
  config.sdd.openspec = "on";
  const hints = resolveSddHints(root, config);
  assert.deepEqual(
    hints.find((h) => h.key === "openspec"),
    { key: "openspec", matchedPath: null },
  );
});

for (const key of ["windsurf", "cline", "roo"] as const) {
  test(`${key} adapter creates a dedicated file and is idempotent`, () => {
    const first = applyAdapter(root, ADAPTERS[key]!, []);
    assert.equal(first.action, "created");
    const file = join(root, ADAPTERS[key]!.targetPath);
    assert.ok(existsSync(file));
    const contentA = readFileSync(file, "utf8");
    assert.ok(contentA.includes(".agnosgram/MEMORY.md"));

    const second = applyAdapter(root, ADAPTERS[key]!, []);
    assert.equal(second.action, "unchanged");
    assert.equal(readFileSync(file, "utf8"), contentA);
  });
}

test("windsurf adapter writes always_on trigger frontmatter", () => {
  applyAdapter(root, ADAPTERS.windsurf!, []);
  const content = readFileSync(join(root, ".windsurf/rules/agnosgram.md"), "utf8");
  assert.ok(content.startsWith("---\ntrigger: always_on\n---\n"));
});

test("cline: a legacy single-file .clinerules gets the managed block merged in, not crashed on", () => {
  const userText = "# my old cline rules\n\nBe terse.\n";
  writeFileSync(join(root, ".clinerules"), userText);

  const res = applyAdapter(root, ADAPTERS.cline!, []);
  assert.equal(res.action, "updated");
  assert.equal(res.path, ".clinerules");
  assert.ok(!existsSync(join(root, ".clinerules/agnosgram.md")));

  const out = readFileSync(join(root, ".clinerules"), "utf8");
  assert.ok(out.startsWith("# my old cline rules\n\nBe terse."));
  assert.ok(out.includes(".agnosgram/MEMORY.md"));

  // Re-running must stay idempotent against the legacy file, not the directory form.
  const second = applyAdapter(root, ADAPTERS.cline!, []);
  assert.equal(second.action, "unchanged");
  assert.equal(second.path, ".clinerules");
});

test("cline: init auto-adapting a legacy single-file .clinerules does not crash", () => {
  const cwd = process.cwd();
  process.chdir(root);
  try {
    writeFileSync(join(root, ".clinerules"), "# old rules\n");
    assert.doesNotThrow(() => runInit([]));
    const out = readFileSync(join(root, ".clinerules"), "utf8");
    assert.ok(out.includes(".agnosgram/MEMORY.md"));
    assert.ok(!existsSync(join(root, ".clinerules/agnosgram.md")));
  } finally {
    process.chdir(cwd);
  }
});

test("cline: no legacy file present still gets the .clinerules/ directory form", () => {
  const res = applyAdapter(root, ADAPTERS.cline!, []);
  assert.equal(res.action, "created");
  assert.equal(res.path, ".clinerules/agnosgram.md");
});

test("applyAdapter raises a UserError instead of crashing when a file occupies the adapter's directory", () => {
  // Not the cline-specific fix path - a plain file sitting where an unrelated
  // adapter needs a directory should still fail cleanly, not with a raw stack trace.
  writeFileSync(join(root, ".windsurf"), "not a directory");
  assert.throws(() => applyAdapter(root, ADAPTERS.windsurf!, []), (err) => {
    assert.ok(err instanceof UserError);
    return true;
  });
});

test("adapt --claude-hooks installs hooks and a skill without requiring an adapter target", () => {
  const cwd = process.cwd();
  process.chdir(root);
  try {
    runInit(["--adapt", "none"]);
    runAdapt(["--claude-hooks"]);
    assert.ok(existsSync(join(root, ".claude/hooks/agnosgram-session-start.mjs")));
    assert.ok(existsSync(join(root, ".claude/hooks/agnosgram-stop-reminder.mjs")));
    assert.ok(existsSync(join(root, ".claude/skills/agnosgram/SKILL.md")));
    const settings = JSON.parse(readFileSync(join(root, ".claude/settings.json"), "utf8"));
    assert.ok(settings.hooks.SessionStart[0].hooks[0].command.includes("agnosgram-session-start.mjs"));
  } finally {
    process.chdir(cwd);
  }
});
