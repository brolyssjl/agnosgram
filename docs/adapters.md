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

**Every adapter here is instruction-driven, not automatic.** The pointer block
tells the agent to read `.agnosgram/MEMORY.md` and follow its reading protocol -
agnosgram never reads or writes agent context on its own behalf. The one exception
is Claude Code with the separate, opt-in `agnosgram adapt --claude-hooks`, which
installs `SessionStart`/`Stop` hooks that actually run `pack`/`log` for you. See
[docs/claude-hooks.md](claude-hooks.md).

## Supported tools

| Tool | `adapt` key | Target file | Detected by |
|---|---|---|---|
| Claude Code | `claude` | `CLAUDE.md` (shared) | `CLAUDE.md`, `.claude/` |
| Cursor | `cursor` | `.cursor/rules/agnosgram.mdc` (dedicated) | `.cursor/` |
| Windsurf | `windsurf` | `.windsurf/rules/agnosgram.md` (dedicated, `trigger: always_on`) | `.windsurf/`, `.windsurfrules` |
| Cline | `cline` | `.clinerules/agnosgram.md` (dedicated), or the legacy `.clinerules` file itself if it already exists | `.clinerules` |
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

## Cline: legacy single-file `.clinerules`

Cline's older convention was a single `.clinerules` file; the current one is a
`.clinerules/` directory of rule files (what `agnosgram adapt cline` writes by
default: `.clinerules/agnosgram.md`). If a project already has the legacy
single-file form, `init`/`adapt` write there instead of creating a colliding
`.clinerules/` directory - the managed block is merged into that file exactly
like `CLAUDE.md` or `AGENTS.md`. `AdaptResult.path` reflects whichever form was
actually used.

## `--json` output

Both `init --json` and `adapt --json` report detected/applied SDD frameworks as
`{ key, matchedPath }` objects (e.g. `{ "key": "openspec", "matchedPath": "openspec/" }`)
under `detectedSdd` (init) and `sdd` (adapt) respectively - the field name is
`matchedPath` in both, matching the internal `SddHint`/`SddDetection` types.
`matchedPath` is only ever a real, `statSync`-confirmed directory; a framework
forced `on` in `config.yml` without a detected directory reports `matchedPath:
null` rather than fabricating a path that may not exist.

**Note for anyone scripting against this:** prior to `0.9.0`, both fields were a
plain array of framework key strings (`["openspec"]`). `0.9.0` is pre-`1.0.0`, so
this shape change ships without a compatibility path - update any script reading
`detectedSdd`/`sdd` to read `.key` off each object instead of using the entries
directly.
