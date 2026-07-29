# Agnosgram roadmap

Living plan for evolving Agnosgram from a capture tool into a self-maintaining,
agent-agnostic memory system. Checkboxes track progress; each milestone ends at a
named release checkpoint.

**How to read this:** the on-disk `.agnosgram/` format is the stable contract - once
frozen (end of Milestone 2) it will not break under you. Commands are added milestone
by milestone and are non-breaking. The version numbers below signal *how much you can
lean on the tool*, not feature count.

## Release strategy

| Version | Gate | What it means for you |
|---|---|---|
| `0.1.0` | Milestone 1 (done) | Core capture works; installable from GitHub. Early. |
| `0.2.0` | Published to npm | `npx agnosgram` / `npm i -g`. Still the M1 surface. |
| `0.5.0` (beta) | Milestone 2 done + format freeze | First release safe to adopt on a real - even legacy - project. Self-maintaining: schema-validated, `doctor` catches rot, `distill` curates. The format is frozen. |
| **`0.8.0` (RC)** | **Milestone 3 done** | **Feature-complete core** incl. `advise` (the landmine-catcher) and `pack`/`show`. Release candidate. |
| `1.0.0` | Milestone 4 + real-world soak | Adapters, skill/plugin, docs, dogfooded for months. Stable CLI + format. |
| `1.x+` | Milestone 5 | Reflexive self-improvement loop and beyond. |

**The "more stable, not quite 1.0" point you asked about is `0.5.0`** - the end of
Milestone 2. From there the format is frozen and the tool maintains itself, which is
exactly what a long-lived legacy project needs. You can start accumulating trustworthy
memory at `0.5.0`. The `advise` feature (`0.8.0`) is what pays off *most* on legacy
code specifically, so you may want to reach it before relying on the tool to catch bad
plans - but capture and anti-rot are solid from `0.5.0`.

## Milestone 1 - Core capture · `0.1` ✅ done
- [x] `.agnosgram/` scaffold + `MEMORY.md` reading protocol + templates
- [x] `init` (scaffold, SDD + agent detection, write adapters)
- [x] `adapt` (Claude Code, Cursor, AGENTS.md) - idempotent managed block
- [x] `log` (4-slot journal entry, branch auto-detect, `--json`)
- [x] Dependency-free YAML subset, marker upsert, config model
- [x] Golden-file idempotence tests + YAML/scaffold/log tests (24 passing)
- [x] Tier-1 token benchmark + CI regression gate
- [x] Repo publish-ready: LICENSE, `prepare` script, GitHub Actions CI, README

### → Publish `0.2.0` to npm (needs npm auth; owner-triggered)
- [ ] Reserve `agnosgram` on npm + first `npm publish`
- [ ] Verify `npx agnosgram init` on a clean machine

## Milestone 2 - Anti-rot & curation · `0.5.0` (beta) ✅ done
_Turns "a Markdown folder" into a self-maintaining store. Ends with a format freeze -
the release to adopt on the legacy project._
- [x] Frontmatter schema: parser + validator (`id`, `type`, `scope`, `confidence`, `created`, `last_verified`, `source`, `supersedes?`)
- [x] `doctor`: stale entries (by `last_verified` age), budget overruns, broken links, orphan/duplicate ids, near-duplicate heuristic
- [x] `doctor` safety lints: secret-scan pattern check, suspicious-imperative flag (prompt-injection guard)
- [x] Budgets: enforce per-file token budgets from `config.yml` + freshness table
- [x] `distill`: emit compaction prompt (merge with `supersedes:`, never append near-dups) + mechanically validate result (schema, ids, budgets); archive distilled journal months
- [x] `bootstrap` (legacy-focused): emit a prompt that seeds `context/architecture.md` + `domain.md` from an existing codebase - fast onboarding for a repo with no memory yet
- [x] **Format freeze**: lock `.agnosgram/` layout + frontmatter schema; document the stable contract (see `.agnosgram/decisions/0002-format-freeze.md`)
- [x] Docs: schema reference + `doctor`/`distill`/`bootstrap` guides (`docs/`)

## Milestone 3 - Retrieval & the killer feature · `0.8.0` (RC) ✅ done
- [x] Pluggable output serializer (JSON default, TOON opt-in) behind one interface
- [x] `pack`: token-budgeted context bundle to stdout (status + lessons + matching scopes)
- [x] `show <topic>`: print entries matching a tag/scope (for agents with weak file navigation)
- [x] `advise <spec-or-plan-path>`: emit contradiction-review prompt (cross-check plan against `lessons/` + `decisions/` with provenance); machine-parseable `--json` report for Gate
- [x] Tier-2 end-to-end token evals (task suite × format matrix)
- [x] Faithful `@toon-format` encoder in bench for real small-object numbers (see LES-002)

## Milestone 4 - Reach & ergonomics · toward `1.0.0`
- [ ] Remaining adapters: Windsurf, Cline/Roo, OpenCode, Codex
- [ ] Claude Code skill/plugin + `SessionStart`/`Stop` hooks (auto-`pack` / auto-`log`)
- [ ] `install.sh` (detect Node → npm, else prebuilt binary) + `bun build --compile` / Node SEA binary in releases
- [ ] Deeper SDD coexistence (OpenSpec / Spec Kit / BMAD / Agent OS behaviors)
- [ ] Docs site + real-project case study (the legacy project)
- [ ] `1.0.0`: format + core CLI declared stable after real-world soak

## Milestone 5 - Reflexive loop · `1.x+`
_Agnosgram uses Agnosgram to evolve Agnosgram. Constraints held: the CLI never calls an
LLM, sends no telemetry, and the human decides._
- [ ] `feedback`: capture tool-friction to a separate `meta/` namespace (never host-project memory)
- [ ] `reflect`: emit a prompt turning friction + journal into improvement proposals + candidate milestones
- [ ] Opt-in community feedback inbox (Discussions / `feedback/` PRs); agent-filed issues via `gh` with consent
- [ ] Human-in-the-loop guarantee: the tool proposes roadmap items, never auto-edits its own roadmap

## Deferred (not scheduled)
Semantic search / embeddings · per-user private memory (`local/`) · monorepo nested
stores · SudoLang playbook variants · Go/Rust port (only if latency data demands it).
