# Agnosgram

**Agent-agnostic, per-project memory for AI coding agents.** Plain Markdown + YAML
frontmatter, committed to your repo, reviewable in PRs. No database, no server, no
API keys, no network after install. Any agent that can read a file can use it.

> Every session with a coding agent starts from zero: it re-discovers the
> architecture, repeats last month's mistakes, and re-litigates settled decisions.
> Existing fixes are agent-captive (`CLAUDE.md`, `.cursor/rules`, Memory Bank).
> Agnosgram stores the memory once, in the repo, and makes the **agents adapt to
> it** - never the reverse.

> **Status:** early and evolving (0.x). Milestone 1 (`init` / `adapt` / `log`) is done
> and dogfooded on this repo. The on-disk `.agnosgram/` layout is the stable contract;
> commands are still being added. Expect changes before 1.0.

## Install

```bash
npm i -g agnosgram      # daily use
npx agnosgram init      # zero-install trial
```

Requires Node ≥ 20. Zero runtime dependencies.

## Quick start

```bash
agnosgram init                 # scaffold .agnosgram/, detect agents + SDD, write adapters
agnosgram adapt claude agents  # inject the managed pointer block into CLAUDE.md / AGENTS.md
agnosgram log --did "..." --learned "..." --next "..."   # append a journal entry
```

## What it creates

```
.agnosgram/
├── MEMORY.md              # entry point: reading protocol + freshness table
├── config.yml             # budgets, adapters, SDD detection toggles
├── state/status.md        # current focus (small, volatile)
├── context/               # architecture.md, stack.md, domain.md
├── decisions/             # lightweight ADRs, one per file
├── lessons/               # pitfalls.md ("don't"), conventions.md ("always")
└── journal/               # append-only session log, one file per month
```

Adapters inject only a ~10-line managed pointer block (between
`<!-- agnosgram:start -->` / `<!-- agnosgram:end -->` markers) that tells the agent
to read `.agnosgram/MEMORY.md` first. The block is idempotent - re-running `adapt`
rewrites only that region and never touches your own content. `AGENTS.md` is the
universal fallback for any agent.

## Commands (Milestone 1)

| Command | What it does |
|---|---|
| `agnosgram init` | Scaffold `.agnosgram/`, detect SDD frameworks + agents, write adapters. `--adapt <list\|none>`, `--force`, `--no-journal-commit`, `--json`. |
| `agnosgram adapt [claude\|cursor\|agents ...]` | Insert/refresh the managed pointer block. `--all`, `--refresh`, `--json`. |
| `agnosgram log` | Append a journal entry from flags (`--did/--learned/--decided/--avoid/--next`) or `--stdin`. Auto-detects branch. `--json` for machine consumers. |

Coming next (Milestone 2): `doctor`, `distill`, `advise`, `pack`, `show`.

## Status & roadmap

- **Milestone 1 - done:** scaffold + `MEMORY.md` protocol + templates, `init`, `adapt` (Claude Code / Cursor / AGENTS.md), `log`.
- **Milestone 2 - next:** `doctor` (anti-rot lint: stale entries, budget overruns, broken links, near-dups), frontmatter schema validation, `distill` (emit compaction prompt + validate result), `pack` / `show`, pluggable JSON/TOON output.
- **Milestone 3 - later:** Claude Code skill/plugin, `install.sh` + prebuilt binary (no-Node fallback), remaining adapters (Windsurf / Cline / OpenCode / Codex), deeper SDD linking.

## How it compares

| | Storage | Works with | Reviewable in PRs | Anti-rot |
|---|---|---|---|---|
| **Agnosgram** | plain Markdown in the repo | any agent (adapters + AGENTS.md fallback) | yes | `last_verified` + `doctor` |
| engram | per-user SQLite + 19 MCP tools | MCP-capable agents only | no (opaque DB) | - |
| Cline Memory Bank | Markdown, single tool | Cline / Roo | in-repo but tool-bound | - |
| braingram | Markdown | Claude Code only | yes | - |
| AGENTS.md / CLAUDE.md | Markdown instructions | per-format | yes | no lifecycle or schema |

The difference: memory as **reviewable documentation that lives in your repo**,
portable across agents, with a capture→distill lifecycle and staleness tracking -
not a per-user database and not a single-agent file.

## Design principles

1. **Storage is plain Markdown + YAML, in the repo.** Human-readable, PR-reviewable. Never a database or opaque blob.
2. **The CLI never calls an LLM.** For judgment steps it *emits a precise prompt* for whatever agent is present, then validates the result mechanically. No API keys, no vendor lock-in.
3. **Agents adapt to the memory, never the reverse.** One source template; content lives only in `.agnosgram/`.
4. **Capture is cheap, distillation is deliberate.** Append-only journal (4 fixed slots) → curated lessons/decisions via explicit `distill`.
5. **Anti-rot is first-class.** Every entry carries `confidence` + `last_verified`; budgets per file; `doctor` flags staleness.
6. **Integrations are advisory, never load-bearing.** SDD frameworks (OpenSpec, Spec Kit, BMAD, Agent OS) only add hint lines; nothing depends on their presence.

## Development

```bash
npm run build     # tsc -> dist/
npm test          # compile + node --test
npm run bench     # Tier-1 token benchmark (add --check to gate in CI)
```

## License

MIT
