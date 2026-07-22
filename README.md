# Agnosgram

**Agent-agnostic, per-project memory for AI coding agents.** Plain Markdown + YAML
frontmatter, committed to your repo, reviewable in PRs. No database, no server, no
API keys, no network after install. Any agent that can read a file can use it.

> Every session with a coding agent starts from zero: it re-discovers the
> architecture, repeats last month's mistakes, and re-litigates settled decisions.
> Existing fixes are agent-captive (`CLAUDE.md`, `.cursor/rules`, Memory Bank).
> Agnosgram stores the memory once, in the repo, and makes the **agents adapt to
> it** - never the reverse.

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

Coming next (Milestone 2): `distill`, `doctor`, `advise`, `pack`, `show`.

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
