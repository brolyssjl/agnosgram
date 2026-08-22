# Claude Code hooks + skill

`agnosgram adapt --claude-hooks` is an **opt-in** flag that wires Agnosgram into
Claude Code's session lifecycle, so memory is loaded and refreshed without
remembering to run commands by hand.

```bash
agnosgram adapt --claude-hooks             # just the hooks + skill
agnosgram adapt claude --claude-hooks       # also (re)write CLAUDE.md's pointer block
agnosgram adapt --refresh                  # re-apply, including hooks IF already installed
```

Nothing here ever happens silently: `--claude-hooks` (or a prior run having already
installed the hooks, combined with `--refresh`) is required, and the command
prints exactly what it wrote or updated, e.g.:

```
Claude Code hooks + skill (SessionStart runs `agnosgram pack`, Stop reminds `agnosgram log`):
  created   .claude/hooks/agnosgram-session-start.mjs
  created   .claude/hooks/agnosgram-stop-reminder.mjs
  created   .claude/settings.json
  created   .claude/skills/agnosgram/SKILL.md
```

## What it installs

| File | Role |
|---|---|
| `.claude/hooks/agnosgram-session-start.mjs` | Runs `agnosgram pack` and returns its output as `additionalContext` on `SessionStart` (matches `startup`, `resume`, `clear`, `compact`, `fork`). |
| `.claude/hooks/agnosgram-stop-reminder.mjs` | On `Stop`, returns a plain reminder to run `agnosgram log` as `additionalContext` - **never blocks**, and backs off immediately if `stop_hook_active` is set, so it cannot loop. |
| `.claude/settings.json` | Gets a merged-in `hooks.SessionStart` / `hooks.Stop` entry pointing at the two scripts above. |
| `.claude/skills/agnosgram/SKILL.md` | Describes when and how to use each agnosgram command, for Claude Code's skill system. |

The Stop hook is a **reminder only** - by design, agnosgram never auto-writes
journal entries. Whether anything happened worth logging, and what to say, stays a
human/agent decision (design principle 4: capture is cheap, but still deliberate).

## Idempotent, conservative merging

Every file is either fully agnosgram-owned (the hook scripts, the skill file:
overwritten whole, safe to regenerate after an upgrade - re-running the command is
how you pick up a newer version) or a narrow merge into `.claude/settings.json`
that:

- identifies "our" hook entry by the hook script's own path inside `command`, and
  only ever replaces or refreshes that one entry;
- **never removes or reorders** any other hook, matcher, or top-level key already
  in the file;
- appends a brand-new entry (never overwrites someone else's) the first time it
  runs, then updates that same entry in place on every re-run - so re-running
  after an agnosgram upgrade never produces duplicates;
- **bails with a clear error** instead of guessing if `.claude/settings.json` isn't
  valid JSON, or if `hooks.SessionStart` / `hooks.Stop` already exist in a shape it
  doesn't recognize (e.g. not an array) - you fix it by hand once, and the merge
  is safe to retry after that.

Run it again any time (after an agnosgram upgrade, or if you deleted the hook
scripts) - it reports `unchanged` for anything already in place. After the first
`--claude-hooks` run, `agnosgram adapt --refresh` alone picks the hooks back up
too (it detects they're already installed), so upgrading doesn't require
remembering the flag again.

## Requirements

The hook scripts shell out to the `agnosgram` binary on `PATH` (see the
[install script](install.md)). If it isn't installed, the
`SessionStart` hook still returns valid JSON with a short explanatory
`additionalContext` instead of failing the session.
