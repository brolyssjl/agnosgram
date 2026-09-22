# How it compares

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
5. **Anti-rot is first-class.** Every entry carries `confidence` + `last_verified`; budgets per file; `doctor` flags staleness. See [docs/doctor.md](doctor.md).
6. **Integrations are advisory, never load-bearing.** SDD frameworks (OpenSpec, Spec Kit, BMAD, Agent OS) only add hint lines; nothing depends on their presence.
