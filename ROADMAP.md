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
| ~~`0.2.0`~~ | ~~Published to npm~~ | ~~`npx agnosgram` / `npm i -g`. Still the M1 surface.~~ superseded by Milestone 6 (binaries, no npm) |
| `0.5.0` (beta) | Milestone 2 done + format freeze | First release safe to adopt on a real - even legacy - project. Self-maintaining: schema-validated, `doctor` catches rot, `distill` curates. The format is frozen. |
| `0.8.0` (RC) | Milestone 3 done | Feature-complete core incl. `advise` (the landmine-catcher) and `pack`/`show`. Release candidate. |
| **`0.9.0`** | **Milestone 4 done** | **Reach**: every mainstream adapter, opt-in Claude Code hooks, `install.sh` + binaries, deeper SDD coexistence. Still pre-1.0 - the docs site and a real-project case study wait for the soak below. |
| `1.0.0` | Milestone 6 Rust port, shipped 2026-08-21 on Round 2 conformance | Rust implementation is canonical. CLI + format frozen (at 0.5.0). Distributed as prebuilt binaries. |
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

### → ~~Publish `0.2.0` to npm (needs npm auth; owner-triggered)~~ superseded by Milestone 6 (binaries, no npm)
- [ ] ~~Reserve `agnosgram` on npm + first `npm publish`~~
- [ ] ~~Verify `npx agnosgram init` on a clean machine~~

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

## Milestone 4 - Reach & ergonomics · `0.9.0` ✅ done
- [x] Remaining adapters: Windsurf, Cline/Roo, OpenCode, Codex
- [x] Claude Code skill + `SessionStart`/`Stop` hooks (auto-`pack` / auto-`log`), opt-in via `agnosgram adapt --claude-hooks`
- [x] `install.sh` (detect Node → npm, else prebuilt binary) + `bun build --compile` / Node SEA binary in releases
- [x] Deeper SDD coexistence: adapter hints point at the actual detected spec directory; `init` acknowledges an existing SDD setup

### Deferred to the `1.0.0` soak period
Moved out of Milestone 4 rather than blocking `0.9.0` on them: both need real-world
usage on a project that hasn't happened yet.
- [ ] Docs site + real-project case study (the legacy project)
- [ ] `1.0.0`: format + core CLI declared stable after real-world soak (dogfood `0.10.0` on the legacy project first)

## Milestone 5 - Reflexive loop · `0.10.0` ✅ done
_Agnosgram uses Agnosgram to evolve Agnosgram. Constraints held: the CLI never calls an
LLM, sends no telemetry, and the human decides._
- [x] `feedback`: capture tool-friction to a separate `meta/` namespace (never host-project memory)
- [x] `reflect`: emit a prompt turning friction + journal into improvement proposals + candidate milestones
- [x] Opt-in community feedback inbox (Discussions / `feedback/` PRs); agent-filed issues via `gh` with consent
- [x] Human-in-the-loop guarantee: the tool proposes roadmap items, never auto-edits its own roadmap

## Milestone 6 - Rust port & `1.0.0`
_Owner decision (2026-08-02): `1.0.0` ships as a Rust implementation, ported
against a frozen CLI surface with the existing test suite as a
cross-implementation conformance suite. Primary distribution becomes prebuilt
static binaries on GitHub Releases - npm leaves the user-facing install path.
The on-disk format contract (DEC-0002) is unchanged and non-negotiable._

### Round 1 - surface freeze (TS) ✅ done
- [x] Shared CLI argument-parsing wrapper: option values starting with `-`
      produce clean UserErrors, never parseArgs stack traces (resolves FRI-001)
- [x] Symlink-aware adapters: CLAUDE.md<->AGENTS.md symlinks reported as one
      honest write (resolves FRI-003)
- [x] Conformance mode: CLI-surface tests run against `$AGNOSGRAM_BIN`
      (`npm run conformance`); contract documented in CONTRIBUTING.md
- [x] Surface freeze declared: CLI commands/flags/outputs documented as the
      port contract (decision record)

### Round 2 - the port ✅ done
- [x] Rust crate in-repo (one crate per tool; duplication with gate accepted):
      identical CLI surface, zero behavior drift, conformance suite green
      against the Rust binary on linux-x64 + darwin-arm64
- [x] Release pipeline builds Rust binaries on tag; `install.sh` unchanged
      (already downloads binaries); `1.0.0-rc` tags from here

### Round 3 - post-`1.0.0` hardening

_Owner decision (2026-08-21): `1.0.0` ships directly from the Round 2 port -
no rc cycle. The conformance suite (104/104 on linux-x64 + darwin-arm64) and
the release pipeline's hard conformance gate stand in for the rc soak as the
`1.0.0` evidence; the Rust binary is the canonical distribution from `1.0.0`.
The remaining items below are still wanted, but as post-`1.0.0` hardening
rather than release gates._

- [x] `1.0.0`: Rust binary is the canonical distribution; TS implementation
      retired 2026-08-22 (owner decision - implementation, unit/conformance
      tests, and the npm toolchain removed; conformance suite ported to
      `rust/tests/`)
- [ ] Upgrade story: version-stamped managed blocks, `doctor` warns on
      stale artifacts, `adapt --refresh` shows a diff before updating.
      **Next scheduled build item** - gate shipped its equivalent as
      `gate doctor`/`gate update` (gate 1.3.0) and it decisively fixed
      gate's invisibility in host repos; that implementation is the
      template (diagnose adapter/artifact drift, heal idempotently, never
      clobber user edits without --force)
- [x] Soak: the real host projects run the Rust binaries daily, including
      one gate → agnosgram RETRO-sync exercise run with both Rust
      binaries together (audit RM-06). Satisfied 2026-08-29: the CF-123
      run on constructflow-api walked gate's full loop on the Rust
      binaries with `gate retro` -> `.agnosgram/journal` sync verified
      end to end (constructflow-api PR #88), and both host repos have
      since completed the full capture -> distill -> archive cycle with
      `doctor` reporting a fully healthy store in each
- [ ] Decide whether to go public / recruit at least one outside pilot
      user (owner decision) (audit RM-03)
- [ ] Docs site + case study (the 2026-07/08 constructflow soak writeup)

## Fixes & improvements
_Open items from the 2026-08-22 audit + remediation (PRs gate#11/#12,
agnosgram#14, all merged) that aren't tied to a milestone above._
- [x] Cut `1.0.1`: `1.0.0` predates SHA256SUMS, so checksum-verified
      installs only become real once the first post-#14 tag publishes it.
      Small, do soon. (audit SEC-02 tail)
- [x] Wire the injection lint into the `pack` path: run the existing
      `core/lint.rs` injection patterns over assembled `pack` output
      before the `SessionStart` hook injects it (owner decision:
      warn-and-mark, not refuse - stderr warning + a reduced-trust banner
      prefixing stdout, never redacted). `doctor`'s own lint is unchanged.
      (audit SEC-07)
- [x] Scope the "automatic memory" claim per adapter in the README: state
      explicitly that automatic context injection is Claude Code-only
      (opt-in hooks) - every other adapter is instruction-driven.
      (audit PUR-02)
- [ ] Give the reflect cadence a tracked heartbeat: run `feedback`/
      `reflect` at the end of every milestone/soak session and log
      accept/reject per proposal, instead of relying on one historical
      proof-of-concept (FRI-001/002/003). (audit RM-04) - first real cadence
      run 2026-08-24 in both host projects (constructflow-web,
      constructflow-api): each filed friction via `feedback`, ran `reflect`,
      and synthesized accept/reject proposals (see the soak reports). Still
      open - this is a recurring practice to keep running, not a one-shot.
      Second cadence run 2026-09-02: friction from two framework-blind
      distill runs in the host repos was filed as issues #26/#27/#28,
      fixed, and released as `1.3.0` the same day (all three accepted;
      see the round block below).
- [x] Refresh `.agnosgram/context/architecture.md`: still describes the
      Milestone-1 TS module map and never tracked Milestones 2-6 or the
      Rust port. (found during PR #14 work)
- [x] Pin GitHub Actions to commit SHAs (owner decision: pin and add
      update automation, not accept tag-pinning risk - see
      `.github/dependabot.yml`). (audit SEC-08)
- [x] Recall-freshness warnings: `doctor` gained `status.stale` (journal
      outpacing status.md's recorded freshness) and `distill.lag` (real
      journal content never distilled), and `pack` appends a matching
      compact note - both soak agents independently ranked this the
      highest-impact finding of the 2026-08-24 soak (soak FRI-004,
      FRI-007/FRI-008). Also scoped the `reflect` prompt template to be
      host-generic, removing dangling `ROADMAP.md`/`CON-003` references
      that only make sense in this repo's own store (soak FRI-005).

_Items from the 2026-09-02 dogfooding round: two framework-blind agents ran
the full distill workflow in the host repos (constructflow-web/-api) with
friction reporting as a primary deliverable; both independently hit the
same trap (#27), which is what earned it the top spot. All fixed and
released as `1.3.0` the same day._

- [x] Per-subcommand `-h`/`--help` (issue #26, PR #29): every subcommand
      used to error with "Unknown option"; now each prints its own scoped
      usage block, exit 0 (mirrors gate's FRI-004 fix)
- [x] Freshness source of truth (issue #27, PR #30): the distill prompt
      now names MEMORY.md's Freshness table as a required output, the
      `status.stale` warning says where the recorded date actually lives,
      and a new `freshness.mismatch` doctor check catches the table and
      status.md's own "Last updated" line disagreeing - previously an
      agent following the emitted prompt literally produced a store its
      own doctor rejected, with no hint why
- [x] `distill --validate` batch mode + token-vs-budget report (`~N/M
      tokens` per file) and documented `doctor --strict` exit semantics
      (issue #28, PR #31)
- [x] `install.sh` removes obsolete pre-Rust `brainstorm-tools/
      agnosgram-v*` install dirs after a checksum-verified install
      (opt-out via `AGNOSGRAM_KEEP_OLD_INSTALLS=1`; PR #25) - this
      installer never even flagged them before
- [ ] `doctor`: warn when `.agnosgram/` contains files untracked by git
      (e.g. `meta/friction.md` sitting untracked in both host repos) -
      friction captured in one checkout is invisible to worktrees and to
      `reflect` runs elsewhere, and every agent doing a `.agnosgram/`-
      scoped task trips over the ambiguity (2026-08-29 soak item I2,
      re-confirmed by both 2026-09-02 distill agents)

## Deferred (not scheduled)
Semantic search / embeddings · per-user private memory (`local/`) · monorepo nested
stores · SudoLang playbook variants.
