import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { UserError } from "./output.js";
import { writeIfChanged, type WriteResult } from "./writeFile.js";

/**
 * Opt-in Claude Code integration (`agnosgram adapt --claude-hooks`): a SessionStart
 * hook that runs `agnosgram pack` into context, a Stop hook that reminds (never
 * forces) the agent to run `agnosgram log`, and a skill file describing the CLI.
 * Every write is either a fully-owned file (overwritten whole, safe because nothing
 * else should be editing it) or a narrow merge into `.claude/settings.json` that
 * only ever touches the one hook entry this tool itself previously wrote.
 */

const HOOKS_REL_DIR = ".claude/hooks";
const SETTINGS_REL_PATH = ".claude/settings.json";
const SKILL_REL_PATH = ".claude/skills/agnosgram/SKILL.md";

const SESSION_START_SCRIPT = "agnosgram-session-start.mjs";
const STOP_SCRIPT = "agnosgram-stop-reminder.mjs";

// A shell one-liner (or a raw `agnosgram pack` command) would spawn one fewer
// Node process per session, but would lose the structured hookSpecificOutput
// envelope and the graceful, explanatory fallback when agnosgram isn't on PATH
// (a bare `|| true` just goes silent) - not worth it for an event that fires at
// session start/resume/compact/fork, not per turn. Kept as plain Node scripts.
const SESSION_START_SCRIPT_CONTENT = `#!/usr/bin/env node
// Managed by agnosgram (\`adapt --claude-hooks\`). Safe to regenerate; re-run that
// command to refresh after an upgrade.
import { execSync } from "node:child_process";

let context;
try {
  context = execSync("agnosgram pack", { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
} catch (err) {
  const detail = err && err.stderr ? err.stderr.toString().trim() : err && err.message ? err.message : String(err);
  context = \`(agnosgram pack failed - is it installed and is this an agnosgram project? \${detail})\`;
}

process.stdout.write(
  JSON.stringify({
    hookSpecificOutput: {
      hookEventName: "SessionStart",
      additionalContext: context,
    },
  }),
);
`;

const STOP_SCRIPT_CONTENT = `#!/usr/bin/env node
// Managed by agnosgram (\`adapt --claude-hooks\`). Safe to regenerate; re-run that
// command to refresh after an upgrade.
import { readFileSync } from "node:fs";

let input = {};
try {
  input = JSON.parse(readFileSync(0, "utf8"));
} catch {
  input = {};
}

// A Stop hook can re-fire itself into a loop; stop_hook_active is Claude Code's
// signal that this hook already ran once this turn, so back off instead of nagging.
if (input.stop_hook_active) {
  process.exit(0);
}

process.stdout.write(
  JSON.stringify({
    hookSpecificOutput: {
      hookEventName: "Stop",
      additionalContext:
        "Reminder: if anything worth remembering happened this session, run " +
        "\`agnosgram log --did .. --learned .. --decided .. --avoid .. --next ..\` " +
        "before you finish. This is a reminder only - deciding whether to log, and what to say, is yours.",
    },
  }),
);
`;

const SKILL_CONTENT = `---
name: agnosgram
description: Read and update this project's Agnosgram memory (.agnosgram/) - status, lessons, decisions, and the session journal. Use when you need project context beyond what's in the code, or to record what happened this session.
---

# Agnosgram

This project keeps durable, agent-agnostic memory in \`.agnosgram/\` (plain Markdown,
reviewed in PRs). The CLI never calls an LLM and sends no telemetry.

## When to use each command
- **Session start:** \`agnosgram pack\` prints a token-budgeted bundle (status +
  lessons, + decisions if scoped). A SessionStart hook may have already loaded this
  into context - check before re-running.
- **Looking for something specific:** \`agnosgram show <topic>\` prints records
  matching an id, scope tag, or type.
- **Before trusting a plan against past failures:** \`agnosgram advise <plan-path>\`
  emits a contradiction-review prompt cross-checking the plan against
  \`lessons/\` and \`decisions/\`.
- **Session end:** \`agnosgram log --did ".." --learned ".." --decided ".." --avoid ".." --next ".."\`
  (at least one flag) appends a journal entry. A Stop hook may remind you to do this.
- **Something looks stale or off:** \`agnosgram doctor\` lints the store (schema,
  staleness, budgets, broken links, safety lints).

## Rules
- Never hand-edit \`lessons/\`, \`decisions/\`, or \`journal/\` directly - use
  \`agnosgram log\` to capture, \`agnosgram distill\` to curate.
- \`state/status.md\` is small and volatile - safe to overwrite freely.
- Read \`.agnosgram/MEMORY.md\` for the full reading protocol.
`;

interface HookGroup {
  matcher?: string;
  hooks: Array<Record<string, unknown>>;
}

function isOwnHook(hook: unknown, scriptName: string): boolean {
  if (!hook || typeof hook !== "object") return false;
  const command = (hook as Record<string, unknown>).command;
  return typeof command === "string" && command.includes(scriptName);
}

/**
 * Insert or refresh this tool's own entry within one hook event's array, touching
 * nothing else. Bails loudly if the existing shape isn't something we can safely
 * merge into, rather than guessing.
 */
function upsertHookGroup(
  raw: unknown,
  eventName: string,
  matcher: string | undefined,
  hookObj: Record<string, unknown>,
  scriptName: string,
): HookGroup[] {
  if (raw !== undefined && !Array.isArray(raw)) {
    throw new UserError(
      `${SETTINGS_REL_PATH}: hooks.${eventName} is not an array; merge --claude-hooks by hand.`,
    );
  }
  const groups = (raw as HookGroup[] | undefined) ?? [];
  for (const g of groups) {
    if (!g || typeof g !== "object" || !Array.isArray(g.hooks)) {
      throw new UserError(
        `${SETTINGS_REL_PATH}: a hooks.${eventName} entry is malformed; merge --claude-hooks by hand.`,
      );
    }
  }

  const groupIdx = groups.findIndex((g) => g.hooks.some((h) => isOwnHook(h, scriptName)));
  if (groupIdx >= 0) {
    // Refresh only our own hook entry - leave the group's matcher alone, since it
    // may have been hand-edited (or shared with a co-located user hook) since we
    // last wrote it.
    const group = groups[groupIdx]!;
    const hookIdx = group.hooks.findIndex((h) => isOwnHook(h, scriptName));
    group.hooks[hookIdx] = hookObj;
    return groups;
  }

  const newGroup: HookGroup = matcher !== undefined ? { matcher, hooks: [hookObj] } : { hooks: [hookObj] };
  return [...groups, newGroup];
}

export interface ClaudeHooksResult {
  written: WriteResult[];
}

/** True when a previous `--claude-hooks` run already installed the SessionStart hook. */
export function hooksInstalled(root: string): boolean {
  return existsSync(join(root, HOOKS_REL_DIR, SESSION_START_SCRIPT));
}

export function installClaudeHooks(root: string): ClaudeHooksResult {
  const written: WriteResult[] = [];

  written.push(writeIfChanged(root, `${HOOKS_REL_DIR}/${SESSION_START_SCRIPT}`, SESSION_START_SCRIPT_CONTENT, { executable: true }));
  written.push(writeIfChanged(root, `${HOOKS_REL_DIR}/${STOP_SCRIPT}`, STOP_SCRIPT_CONTENT, { executable: true }));

  const settingsPath = join(root, SETTINGS_REL_PATH);
  let settings: Record<string, unknown> = {};
  if (existsSync(settingsPath)) {
    const raw = readFileSync(settingsPath, "utf8");
    try {
      settings = JSON.parse(raw);
    } catch {
      throw new UserError(`${SETTINGS_REL_PATH} is not valid JSON; fix it by hand before using --claude-hooks.`);
    }
    if (settings === null || typeof settings !== "object" || Array.isArray(settings)) {
      throw new UserError(`${SETTINGS_REL_PATH} does not contain a JSON object; cannot merge hooks automatically.`);
    }
  }

  const hooksRaw = settings.hooks;
  if (hooksRaw !== undefined && (typeof hooksRaw !== "object" || hooksRaw === null || Array.isArray(hooksRaw))) {
    throw new UserError(`${SETTINGS_REL_PATH}: "hooks" is not an object; cannot merge automatically.`);
  }
  const hooks: Record<string, unknown> = { ...(hooksRaw as Record<string, unknown> | undefined) };

  const sessionStartHook = {
    type: "command",
    command: `node "\${CLAUDE_PROJECT_DIR}/${HOOKS_REL_DIR}/${SESSION_START_SCRIPT}"`,
    timeout: 15,
  };
  const stopHook = {
    type: "command",
    command: `node "\${CLAUDE_PROJECT_DIR}/${HOOKS_REL_DIR}/${STOP_SCRIPT}"`,
    timeout: 5,
  };

  hooks.SessionStart = upsertHookGroup(
    hooks.SessionStart,
    "SessionStart",
    "startup|resume|clear|compact|fork",
    sessionStartHook,
    SESSION_START_SCRIPT,
  );
  hooks.Stop = upsertHookGroup(hooks.Stop, "Stop", undefined, stopHook, STOP_SCRIPT);
  settings.hooks = hooks;

  const next = JSON.stringify(settings, null, 2) + "\n";
  written.push(writeIfChanged(root, SETTINGS_REL_PATH, next));

  written.push(writeIfChanged(root, SKILL_REL_PATH, SKILL_CONTENT));

  return { written };
}
