import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { installClaudeHooks } from "./claudeHooks.js";
import { UserError } from "./output.js";

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "agnos-hooks-"));
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

test("creates hook scripts, settings.json, and the skill file", () => {
  const result = installClaudeHooks(root);
  const paths = result.written.map((w) => w.path).sort();
  assert.deepEqual(paths, [
    ".claude/hooks/agnosgram-session-start.mjs",
    ".claude/hooks/agnosgram-stop-reminder.mjs",
    ".claude/settings.json",
    ".claude/skills/agnosgram/SKILL.md",
  ]);
  assert.ok(result.written.every((w) => w.action === "created"));

  const settings = JSON.parse(readFileSync(join(root, ".claude/settings.json"), "utf8"));
  assert.equal(settings.hooks.SessionStart[0].hooks[0].command.includes("agnosgram-session-start.mjs"), true);
  assert.equal(settings.hooks.Stop[0].hooks[0].command.includes("agnosgram-stop-reminder.mjs"), true);
  assert.equal(settings.hooks.Stop[0].matcher, undefined);

  const skill = readFileSync(join(root, ".claude/skills/agnosgram/SKILL.md"), "utf8");
  assert.ok(skill.startsWith("---\nname: agnosgram"));
});

test("is idempotent: running twice reports unchanged and yields identical files", () => {
  installClaudeHooks(root);
  const before = {
    settings: readFileSync(join(root, ".claude/settings.json"), "utf8"),
    sessionStart: readFileSync(join(root, ".claude/hooks/agnosgram-session-start.mjs"), "utf8"),
  };
  const second = installClaudeHooks(root);
  assert.ok(second.written.every((w) => w.action === "unchanged"));
  assert.equal(readFileSync(join(root, ".claude/settings.json"), "utf8"), before.settings);
  assert.equal(readFileSync(join(root, ".claude/hooks/agnosgram-session-start.mjs"), "utf8"), before.sessionStart);
});

test("preserves unrelated hooks and settings already in .claude/settings.json", () => {
  mkdirSync(join(root, ".claude"), { recursive: true });
  const userSettings = {
    permissions: { allow: ["Bash(git *)"] },
    hooks: {
      SessionStart: [{ matcher: "startup", hooks: [{ type: "command", command: "echo user-hook" }] }],
      PreToolUse: [{ matcher: "Bash", hooks: [{ type: "command", command: "echo audit" }] }],
    },
  };
  writeFileSync(join(root, ".claude/settings.json"), JSON.stringify(userSettings, null, 2) + "\n");

  installClaudeHooks(root);

  const settings = JSON.parse(readFileSync(join(root, ".claude/settings.json"), "utf8"));
  assert.deepEqual(settings.permissions, { allow: ["Bash(git *)"] });
  assert.deepEqual(settings.hooks.PreToolUse, userSettings.hooks.PreToolUse);
  assert.equal(settings.hooks.SessionStart.length, 2);
  assert.equal(settings.hooks.SessionStart[0].hooks[0].command, "echo user-hook");
  assert.ok(settings.hooks.SessionStart[1].hooks[0].command.includes("agnosgram-session-start.mjs"));
});

test("refreshing an existing agnosgram hook entry updates in place instead of duplicating", () => {
  installClaudeHooks(root);
  installClaudeHooks(root);
  const settings = JSON.parse(readFileSync(join(root, ".claude/settings.json"), "utf8"));
  assert.equal(settings.hooks.SessionStart.length, 1);
  assert.equal(settings.hooks.Stop.length, 1);
});

test("throws a clear UserError when settings.json is not valid JSON", () => {
  mkdirSync(join(root, ".claude"), { recursive: true });
  writeFileSync(join(root, ".claude/settings.json"), "{ not json");
  assert.throws(() => installClaudeHooks(root), UserError);
});

test("throws a clear UserError when hooks.SessionStart is not an array", () => {
  mkdirSync(join(root, ".claude"), { recursive: true });
  writeFileSync(
    join(root, ".claude/settings.json"),
    JSON.stringify({ hooks: { SessionStart: "not-an-array" } }),
  );
  assert.throws(() => installClaudeHooks(root), UserError);
});

test("hook commands quote ${CLAUDE_PROJECT_DIR} so a path with spaces still works", () => {
  installClaudeHooks(root);
  const settings = JSON.parse(readFileSync(join(root, ".claude/settings.json"), "utf8"));
  const sessionCommand = settings.hooks.SessionStart[0].hooks[0].command;
  const stopCommand = settings.hooks.Stop[0].hooks[0].command;
  assert.equal(sessionCommand, 'node "${CLAUDE_PROJECT_DIR}/.claude/hooks/agnosgram-session-start.mjs"');
  assert.equal(stopCommand, 'node "${CLAUDE_PROJECT_DIR}/.claude/hooks/agnosgram-stop-reminder.mjs"');
});

test("SessionStart matcher includes fork, so forked sessions get the pack context too", () => {
  installClaudeHooks(root);
  const settings = JSON.parse(readFileSync(join(root, ".claude/settings.json"), "utf8"));
  const sources = settings.hooks.SessionStart[0].matcher.split("|");
  assert.deepEqual(sources.sort(), ["clear", "compact", "fork", "resume", "startup"]);
});

test("refreshing our own SessionStart entry never touches an existing matcher we did not write", () => {
  mkdirSync(join(root, ".claude"), { recursive: true });
  writeFileSync(
    join(root, ".claude/settings.json"),
    JSON.stringify({
      hooks: {
        SessionStart: [
          {
            matcher: "startup",
            hooks: [
              { type: "command", command: "echo co-located-user-hook" },
              { type: "command", command: 'node "${CLAUDE_PROJECT_DIR}/.claude/hooks/agnosgram-session-start.mjs"' },
            ],
          },
        ],
      },
    }),
  );

  installClaudeHooks(root);

  const settings = JSON.parse(readFileSync(join(root, ".claude/settings.json"), "utf8"));
  assert.equal(settings.hooks.SessionStart.length, 1);
  assert.equal(settings.hooks.SessionStart[0].matcher, "startup");
  assert.equal(settings.hooks.SessionStart[0].hooks[0].command, "echo co-located-user-hook");
});

test("hook scripts are written executable", () => {
  installClaudeHooks(root);
  const sessionStartPath = join(root, ".claude/hooks/agnosgram-session-start.mjs");
  assert.ok(existsSync(sessionStartPath));
  const mode = statSync(sessionStartPath).mode;
  assert.ok((mode & 0o100) !== 0, "owner-executable bit should be set");
});
