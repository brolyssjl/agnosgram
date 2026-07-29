# Agnosgram

**Agent-agnostic, per-project memory for AI coding agents.** Plain Markdown + YAML
frontmatter, committed to your repo, reviewable in PRs. No database, no server, no
API keys, no network after install. Any agent that can read a file can use it.

> Every session with a coding agent starts from zero: it re-discovers the
> architecture, repeats last month's mistakes, and re-litigates settled decisions.
> Existing fixes are agent-captive (`CLAUDE.md`, `.cursor/rules`, Memory Bank).
> Agnosgram stores the memory once, in the repo, and makes the **agents adapt to
> it** - never the reverse.

> **Status:** release candidate (`0.8.0`). Milestones 1-3 are done and dogfooded on
> this repo: capture (`init` / `adapt` / `log`), the self-maintaining half (`doctor` /
> `distill` / `bootstrap`), and retrieval (`pack` / `show` / `advise`).
> **The on-disk `.agnosgram/` format is frozen** at format version 1 - safe to adopt
> on a real, even legacy, project. See the [schema reference](docs/schema-reference.md).

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

## Supported adapters

| Tool | `adapt` key | Target file |
|---|---|---|
| Claude Code | `claude` | `CLAUDE.md` (shared, managed block) |
| Cursor | `cursor` | `.cursor/rules/agnosgram.mdc` (dedicated) |
| Windsurf | `windsurf` | `.windsurf/rules/agnosgram.md` (dedicated) |
| Cline | `cline` | `.clinerules/agnosgram.md` (dedicated) |
| Roo Code | `roo` | `.roo/rules/agnosgram.md` (dedicated) |
| Codex, OpenCode, or any other `AGENTS.md` reader | `agents` | `AGENTS.md` (shared, managed block) |

Codex and OpenCode both read `AGENTS.md` natively, so `init`/`adapt` detect them
(`.codex/`, `.opencode/`, `opencode.json`) and route to the `agents` adapter instead
of writing a duplicate file. See [docs/adapters.md](docs/adapters.md) for detection
signals and per-tool notes.

## Commands

| Command | What it does |
|---|---|
| `agnosgram init` | Scaffold `.agnosgram/`, detect SDD frameworks + agents, write adapters. `--adapt <list\|none>`, `--force`, `--no-journal-commit`, `--json`. |
| `agnosgram adapt [claude\|cursor\|windsurf\|cline\|roo\|agents ...]` | Insert/refresh the managed pointer block. `--all`, `--refresh`, `--claude-hooks` (opt-in Claude Code `SessionStart`/`Stop` hooks + skill, see [guide](docs/claude-hooks.md)), `--json`. |
| `agnosgram log` | Append a journal entry from flags (`--did/--learned/--decided/--avoid/--next`) or `--stdin`. Auto-detects branch. `--json` for machine consumers. |
| `agnosgram doctor` | Lint the store: schema, staleness, budgets, broken links, duplicate/near-duplicate ids, and safety lints (secret scan + prompt-injection guard). `--strict`, `--json`. See [guide](docs/doctor.md). |
| `agnosgram distill` | Emit a compaction prompt (merge via `supersedes:`, never append near-dups); `--validate <file>` checks a distilled result; `--archive <YYYY-MM>` retires an absorbed journal month. See [guide](docs/distill.md). |
| `agnosgram bootstrap` | Emit a prompt that seeds `context/architecture.md` + `domain.md` from an existing codebase - fast onboarding for a legacy repo. See [guide](docs/bootstrap.md). |
| `agnosgram show <topic>` | Print records matching an id, scope tag, or type - for agents with weak file navigation. `--type`, `--format json\|toon`. See [guide](docs/show.md). |
| `agnosgram pack` | Token-budgeted context bundle: status + lessons (+ decisions when `--scope`d). Human output is the Markdown bundle itself. `--scope`, `--budget`, `--format json\|toon`. See [guide](docs/pack.md). |
| `agnosgram advise <plan-path>` | Emit a plan-vs-memory contradiction review prompt; `--validate <report>` mechanically checks the agent's JSON report. `--strict`, `--format json\|toon`. See [guide](docs/advise.md). |

Judgment steps (`distill`, `bootstrap`, `advise`) **emit a prompt** for your agent
and then validate the result mechanically - the CLI itself never calls an LLM.

Milestone 4 added the remaining adapters (Windsurf, Cline/Roo, OpenCode, Codex), a
Claude Code skill with session hooks (see [docs/claude-hooks.md](docs/claude-hooks.md)),
and `install.sh` + prebuilt binaries (see [docs/install.md](docs/install.md)).

## Anti-rot

Every curated entry carries `confidence` + `last_verified`; `config.yml` sets a
`staleness_days` window and per-file token budgets. `agnosgram doctor` turns that
into an executable check you can run in CI - it is the specification of the frozen
format. Full field-by-field contract: **[docs/schema-reference.md](docs/schema-reference.md)**.

## Status & roadmap

Milestones 1-3 are done and dogfooded: capture (`init` / `adapt` / `log`), the
self-maintaining half (`doctor` / `distill` / `bootstrap`), and retrieval
(`pack` / `show` / `advise`), with the on-disk format frozen at version 1.
Full plan with progress checkboxes and release checkpoints:
**[ROADMAP.md](ROADMAP.md)**.

**`0.5.0`** was the first release safe to adopt on a real project: frontmatter
schema validation, `doctor`, `distill`, and a frozen on-disk format.
**`0.8.0`** (this release, RC) adds the killer feature: `advise`, the
contradiction-catcher, plus `pack` and `show`.

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
npm run tier2     # Tier-2 end-to-end token eval (add --check to gate in CI)
```

## License

MIT
