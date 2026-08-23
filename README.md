# Agnosgram

**Agent-agnostic, per-project memory for AI coding agents.** Plain Markdown + YAML
frontmatter, committed to your repo, reviewable in PRs. No database, no server, no
API keys, no network after install. Any agent that can read a file can use it.

> Every session with a coding agent starts from zero: it re-discovers the
> architecture, repeats last month's mistakes, and re-litigates settled decisions.
> Existing fixes are agent-captive (`CLAUDE.md`, `.cursor/rules`, Memory Bank).
> Agnosgram stores the memory once, in the repo, and makes the **agents adapt to
> it** - never the reverse.

> **Status:** `1.0.0`. Milestones 1-5 plus the Milestone 6 Rust port are done
> and dogfooded on this repo: capture
> (`init` / `adapt` / `log`), the self-maintaining half (`doctor` / `distill` /
> `bootstrap`), retrieval (`pack` / `show` / `advise`), reach (every mainstream
> adapter, opt-in Claude Code hooks, `install.sh` + binaries, SDD coexistence), the
> reflexive loop (`feedback` / `reflect`, a strictly separate `meta/` namespace),
> and the Rust port. **The on-disk `.agnosgram/` format is frozen** at format
> version 1 - safe to adopt on a real, even legacy, project - and the CLI surface
> is frozen as the conformance-checked contract (DEC-0004), pinned by
> `rust/tests/`. **Rust is the only implementation** - the TypeScript reference
> it was ported from was retired 2026-08-22. See [ROADMAP.md](ROADMAP.md) and the
> [schema reference](docs/schema-reference.md).

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/brolyssjl/agnosgram/main/install.sh | bash
```

While this repository is private, the unauthenticated one-liner 404s (both
the raw script and the release assets). Run the script from a clone instead -
it falls back to `gh release download`, which reuses your GitHub auth:

```bash
git clone https://github.com/brolyssjl/agnosgram.git && ./agnosgram/install.sh
```

Fetches the prebuilt binary for your platform from the latest GitHub release
and puts it on `PATH` - no local Node, no local Rust toolchain required.
Binaries have shipped since `0.9.0` (`linux-x64`, `darwin-arm64`) and are
cargo-built from `1.0.0` onward. No matching binary yet? The script falls
back to honest build-from-source steps instead of guessing.

`agnosgram` is not on npm and never will be - the Milestone 6 owner decision
(see [ROADMAP.md](ROADMAP.md)) makes prebuilt binaries the permanent
user-facing install path.

Build from source instead - zero external crates, only a stable Rust
toolchain required:

```bash
cargo build --release --manifest-path rust/Cargo.toml
# -> rust/target/release/agnosgram
```

Full details: **[docs/install.md](docs/install.md)**.

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

`meta/` (tool friction, not host-project memory) is opt-in: `init` never
scaffolds it, `agnosgram feedback` creates it on first use. See
[docs/feedback.md](docs/feedback.md).

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
| `agnosgram pack` | Token-budgeted context bundle: status + lessons (+ decisions when `--scope`d). Human output is the Markdown bundle itself. Runs the prompt-injection lint over assembled content - warns on stderr and marks stdout on a hit, never redacts. `--scope`, `--budget`, `--format json\|toon`. See [guide](docs/pack.md). |
| `agnosgram advise <plan-path>` | Emit a plan-vs-memory contradiction review prompt; `--validate <report>` mechanically checks the agent's JSON report. `--strict`, `--format json\|toon`. See [guide](docs/advise.md). |
| `agnosgram feedback "<text>"` | Capture tool friction into `.agnosgram/meta/` - never host-project memory, never read by `pack`/`show`/`advise`. `--scope`, `--confidence`, `--stdin`, `--share` (prints a ready-to-run `gh issue create` command; never runs it). See [guide](docs/feedback.md). |
| `agnosgram reflect` | Emit a prompt turning tool friction + recent journal months into improvement proposals and candidate roadmap milestones. Read-only; `ROADMAP.md` stays owner-edited. `--months`, `--format json\|toon`. See [guide](docs/reflect.md). |

Judgment steps (`distill`, `bootstrap`, `advise`, `reflect`) **emit a prompt** for your
agent and then validate the result mechanically (where there is a result to check) -
the CLI itself never calls an LLM.

Milestone 4 added the remaining adapters (Windsurf, Cline/Roo, OpenCode, Codex), a
Claude Code skill with session hooks (see [docs/claude-hooks.md](docs/claude-hooks.md)),
and `install.sh` + prebuilt binaries (see [docs/install.md](docs/install.md)). Milestone 5
added the reflexive loop: `feedback` captures tool friction into `.agnosgram/meta/`
(see [docs/feedback.md](docs/feedback.md)), and `reflect` turns it into proposals
(see [docs/reflect.md](docs/reflect.md)) - the tool proposes, a human decides.

## Anti-rot

Every curated entry carries `confidence` + `last_verified`; `config.yml` sets a
`staleness_days` window and per-file token budgets. `agnosgram doctor` turns that
into an executable check you can run in CI - it is the specification of the frozen
format. Full field-by-field contract: **[docs/schema-reference.md](docs/schema-reference.md)**.

## Status & roadmap

Milestones 1-5 plus the Milestone 6 Rust port are done and dogfooded: capture
(`init` / `adapt` / `log`), the
self-maintaining half (`doctor` / `distill` / `bootstrap`), retrieval (`pack` /
`show` / `advise`), reach (adapters, Claude Code hooks, install.sh + binaries,
SDD coexistence), the reflexive loop (`feedback` / `reflect`), and the Rust
port, with the on-disk format frozen at version 1. Round 3 (post-`1.0.0`
hardening: upgrade story, daily-driver soak, docs site) is next. Full plan
with progress checkboxes and release checkpoints: **[ROADMAP.md](ROADMAP.md)**.

**`0.5.0`** was the first release safe to adopt on a real project: frontmatter
schema validation, `doctor`, `distill`, and a frozen on-disk format.
**`0.8.0`** added the killer feature: `advise`, the contradiction-catcher, plus
`pack` and `show`. **`0.9.0`** was Milestone 4: every mainstream adapter,
opt-in Claude Code hooks, `install.sh` + prebuilt binaries, and deeper SDD
coexistence. **`0.10.0`** was Milestone 5, the reflexive loop:
`feedback` captures tool friction into a separate, additive `meta/` namespace;
`reflect` turns it into proposals - the tool proposes, a human decides.
**`0.11.0`** opened Milestone 6: the CLI surface frozen as the Rust port
contract (DEC-0004), enforced by a conformance suite run against
`$AGNOSGRAM_BIN`. **`1.0.0`** ships the Rust port itself, straight from the
merged Round 2 conformance evidence (104/104 on `linux-x64` and
`darwin-arm64`) with no rc cycle - the Rust binary became the canonical
distribution, with the TypeScript implementation staying in-repo as
reference. That reference was retired 2026-08-22, once its conformance
suite had been ported to `rust/tests/`: Rust is now the only implementation.

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
cargo build --release --manifest-path rust/Cargo.toml
cargo test --manifest-path rust/Cargo.toml   # unit + conformance + token benchmark gates
```

Full dev loop (fmt, clippy, and what each test tier covers): [Rust implementation section in CONTRIBUTING.md](CONTRIBUTING.md#rust-implementation).

## License

MIT
