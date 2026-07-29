# Adapters

An adapter injects the same ~10-line pointer body into one agent's config file,
telling it to read `.agnosgram/MEMORY.md` first (design principle 3: one source
template; content lives only in `.agnosgram/`). Shared files (`CLAUDE.md`,
`AGENTS.md`) get the block merged into whatever else is already there; a dedicated
file (Cursor's `.mdc`, Windsurf's `.md`, Cline's, Roo's) is fully agnosgram's.

```bash
agnosgram adapt claude cursor   # write named adapters, and enable them for `--refresh`
agnosgram adapt --all           # write every known adapter
agnosgram adapt                 # refresh whatever's already enabled (config.yml)
```

Every write goes through `upsertManagedBlock`: idempotent, and it never touches
content outside the `<!-- agnosgram:start -->` / `<!-- agnosgram:end -->` markers.
Golden-file tests (`src/commands/adapt.test.ts`) prove run-twice-is-identical for
every adapter.

## Supported tools

| Tool | `adapt` key | Target file | Detected by |
|---|---|---|---|
| Claude Code | `claude` | `CLAUDE.md` (shared) | `CLAUDE.md`, `.claude/` |
| Cursor | `cursor` | `.cursor/rules/agnosgram.mdc` (dedicated) | `.cursor/` |
| Windsurf | `windsurf` | `.windsurf/rules/agnosgram.md` (dedicated, `trigger: always_on`) | `.windsurf/`, `.windsurfrules` |
| Cline | `cline` | `.clinerules/agnosgram.md` (dedicated) | `.clinerules/` |
| Roo Code | `roo` | `.roo/rules/agnosgram.md` (dedicated) | `.roo/`, `.roorules` |
| Codex, OpenCode, or any other `AGENTS.md` reader | `agents` | `AGENTS.md` (shared) | `AGENTS.md`, `.codex/`, `.opencode/`, `opencode.json` |

## Codex and OpenCode read `AGENTS.md` natively

Rather than duplicate the pointer body into a second file, `init`/`adapt` detect
Codex (`.codex/`) and OpenCode (`.opencode/`, `opencode.json`) and route them to the
`agents` adapter - the same `AGENTS.md` that Claude Code, Cursor, and any other
`AGENTS.md`-aware tool can read. This keeps the "one source template" principle:
there is exactly one file to keep in sync for every tool that already reads
`AGENTS.md`.

## Detection is advisory, never load-bearing

`init` and `adapt --all`/default use presence detection (`src/core/detect.ts`) only
to decide which adapters to *offer* or *auto-refresh*. A missing or wrong detection
can only change which files get a hint block; it can never corrupt the
`.agnosgram/` store itself. Force an adapter on regardless of detection with
`agnosgram adapt <key>` (which flips `config.yml`'s `adapters.<key>` to `on`) or
`--all`.

## SDD coexistence hints

If a spec-driven-development framework (OpenSpec, Spec Kit, BMAD, Agent OS) is
present, the pointer body gets an extra "Coexisting tools detected" section that
names the actual directory found (e.g. `openspec/`, `.bmad-core/`) so the agent
knows where specs live without agnosgram ever reading or editing them. See
`config.yml`'s `sdd` toggles to force a hint on/off regardless of detection.
