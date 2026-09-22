<div align="center">
  <img src="docs/assets/agnosgram-mascot.svg" width="150" alt="Agnosgram's mascot: a cyberpunk elephant with a glowing stylus in its trunk">

# Agnosgram

**Agent-agnostic, per-project memory for AI coding agents.**

[![Release](https://img.shields.io/github/v/release/brolyssjl/agnosgram)](https://github.com/brolyssjl/agnosgram/releases)
[![CI](https://github.com/brolyssjl/agnosgram/actions/workflows/ci.yml/badge.svg)](https://github.com/brolyssjl/agnosgram/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/github/license/brolyssjl/agnosgram)](LICENSE)

</div>

## What it is

Every session with a coding agent starts from zero. It re-discovers your
architecture, repeats mistakes your team already fixed, and re-litigates
decisions you already made. The usual fixes for this are agent-captive:
`CLAUDE.md`, `.cursor/rules`, Memory Bank, whatever file format your current
tool wants. Agnosgram stores that memory once, in your repo, and makes
agents adapt to it, never the other way around.

The store is plain Markdown + YAML frontmatter, committed to your repo and
reviewable in PRs. There is no database, no server, no API keys, and no
network after install. Any agent that can read a file can use it.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/brolyssjl/agnosgram/main/install.sh | bash
```

Pin a version with `AGNOSGRAM_VERSION=1.5.0` before the command. No prebuilt
binary for your platform? Build from source, zero external crates required:

```bash
cargo build --release --manifest-path rust/Cargo.toml
# -> rust/target/release/agnosgram
```

Full details, the `gh`-fallback for flaky networks, and troubleshooting:
[docs/install.md](docs/install.md).

## Quick start

```bash
agnosgram init                 # scaffold .agnosgram/, detect agents + SDD, write adapters
agnosgram adapt claude agents  # inject the managed pointer block into CLAUDE.md / AGENTS.md
```

`init` creates `.agnosgram/` in your repo; `adapt` adds a short block to your
agent's config file telling it to read `.agnosgram/MEMORY.md` first. Commit
both. From there, the day-to-day loop is:

```bash
agnosgram log --did "..." --learned "..." --next "..."   # append a journal entry
agnosgram doctor                                          # check the store for rot
agnosgram distill                                         # curate the journal into lessons
agnosgram advise <plan-path>                              # check a plan against memory first
```

Your agent reads the store itself because the pointer block tells it to.
Claude Code can also do this automatically: `agnosgram adapt --claude-hooks`.

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

`meta/` is opt-in tool-friction storage: `init` never scaffolds it, only
`agnosgram feedback` does. See [docs/feedback.md](docs/feedback.md).

## Commands

| Command | What it does | Docs |
|---|---|---|
| `init` | Scaffold `.agnosgram/`, detect agents and SDD frameworks, write adapters. | |
| `adapt` | Insert or refresh an agent's managed pointer block. | [guide](docs/adapters.md) |
| `log` | Append a journal entry (agents call this at session end). | |
| `doctor` | Lint the store: schema, staleness, budgets, links, safety. | [guide](docs/doctor.md) |
| `distill` | Curate the journal into lessons and decisions. | [guide](docs/distill.md) |
| `bootstrap` | Seed `context/` from an existing codebase. | [guide](docs/bootstrap.md) |
| `show` | Print records matching an id, scope tag, or type. | [guide](docs/show.md) |
| `pack` | Emit a token-budgeted context bundle. | [guide](docs/pack.md) |
| `advise` | Check a plan against memory before you commit to it. | [guide](docs/advise.md) |
| `feedback` | Capture friction with the tool itself, separate from project memory. | [guide](docs/feedback.md) |
| `reflect` | Turn friction and recent journal entries into improvement proposals. | [guide](docs/reflect.md) |

Run `agnosgram <command> --help` for flags. Judgment steps (`distill`,
`bootstrap`, `advise`, `reflect`) emit a prompt for your agent and validate
the result mechanically; the CLI itself never calls an LLM.

## Supported agents

| Tool | Target file |
|---|---|
| Claude Code | `CLAUDE.md` (shared) |
| Cursor | `.cursor/rules/agnosgram.mdc` |
| Windsurf | `.windsurf/rules/agnosgram.md` |
| Cline | `.clinerules/agnosgram.md` |
| Roo Code | `.roo/rules/agnosgram.md` |
| Codex, OpenCode, or anything else that reads `AGENTS.md` | `AGENTS.md` (shared) |

Every adapter is instruction-driven: it writes a pointer block, and the agent
reads it and follows the protocol itself. The one exception is Claude Code
with `agnosgram adapt --claude-hooks`, which runs `pack`/`log` automatically
through session hooks. Detection rules and per-tool notes:
[docs/adapters.md](docs/adapters.md).

## Learn more

- [docs/install.md](docs/install.md) - the full install story and troubleshooting
- [docs/adapters.md](docs/adapters.md) and [docs/claude-hooks.md](docs/claude-hooks.md) - agent integration details
- [docs/doctor.md](docs/doctor.md) - anti-rot checks and the safety lints
- [docs/distill.md](docs/distill.md), [docs/bootstrap.md](docs/bootstrap.md), [docs/show.md](docs/show.md), [docs/pack.md](docs/pack.md), [docs/advise.md](docs/advise.md) - one guide per command
- [docs/feedback.md](docs/feedback.md) and [docs/reflect.md](docs/reflect.md) - the opt-in friction loop
- [docs/schema-reference.md](docs/schema-reference.md) - the frozen on-disk format, field by field
- [docs/trust-posture.md](docs/trust-posture.md) - how Agnosgram treats store content as untrusted data
- [docs/how-it-compares.md](docs/how-it-compares.md) - comparison table and design principles
- [ROADMAP.md](ROADMAP.md) - release history and what's next

## Development

```bash
cargo build --release --manifest-path rust/Cargo.toml
cargo test --manifest-path rust/Cargo.toml   # unit + conformance + token benchmark gates
```

Full dev loop (fmt, clippy, and what each test tier covers):
[Rust implementation section in CONTRIBUTING.md](CONTRIBUTING.md#rust-implementation).

## License

MIT
