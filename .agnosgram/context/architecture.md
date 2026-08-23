# Architecture

_System shape, module map, key invariants. Keep it to what an agent must know
before touching the code - not an exhaustive tour._

## Shape

A single Rust crate (`rust/`, package `agnosgram`) with `lib` + `bin`
targets. The `bin` (`src/main.rs`) is a thin CLI dispatcher; `lib.rs`
re-exports `core::*` so `rust/tests/*_conformance.rs` and `bench/` can
exercise the exact code the binary ships, instead of reimplementing it.
Zero runtime dependencies (decisions/0001) - everything below is hand-rolled
against `std` only. Rust is the only implementation; the TypeScript
reference it was ported from was retired 2026-08-22.

## Module map

- `src/main.rs` -> entry point: argv dispatch to `commands::*`, `--help`/
  `--version`, converts `UserError` to exit code 1, unknown command to 2.
- `src/adapters.rs` -> the adapter registry (`claude`, `cursor`, `windsurf`,
  `cline`, `roo`, `agents`) + the one shared pointer-body template and SDD
  hint lines every adapter injects.
- `src/claude_hooks.rs` -> opt-in Claude Code integration
  (`adapt --claude-hooks`): writes `SessionStart`/`Stop` hook scripts + a
  skill file. The only adapter that injects context automatically - every
  other adapter is instruction-driven (see README's adapters section).
- `src/commands/` -> one file per CLI command, each a `run(argv)`: `init`,
  `adapt`, `log`, `doctor`, `distill`, `bootstrap`, `show`, `pack`, `advise`,
  `feedback`, `reflect`.
- `src/core/`:
  - `args.rs` -> shared CLI arg parsing (`parse_cli_args`); every command
    routes through it so bad input fails the same way everywhere (FRI-001).
  - `paths.rs` -> project-root discovery (`.agnosgram/` -> `.git/` -> cwd).
  - `config.rs` -> `config.yml` load/save/defaults.
  - `frontmatter.rs` -> record frontmatter extraction + schema validation
    (read-only; `distill` is where records get authored, by prompt).
  - `records.rs` -> shared, schema-valid record index for `show`/`pack`/
    `advise`, built on `store.rs` + `frontmatter.rs`.
  - `store.rs` -> raw file walk of `.agnosgram/`, record-bearing detection.
  - `lint.rs` -> safety lints: secret patterns (error) and prompt-injection
    patterns (warn). `doctor` runs both over the whole store; `pack` runs
    the injection set over just the content it assembles for output
    (warn-and-mark, see `docs/pack.md`).
  - `markers.rs` -> `upsert_managed_block`, the idempotent injection every
    adapter write goes through.
  - `write_file.rs` -> shared write-if-changed helper.
  - `detect.rs` -> advisory-only SDD + agent detection (path presence).
  - `freshness.rs` -> `MEMORY.md`'s freshness table (file-level staleness,
    the counterpart to per-record `last_verified`).
  - `dates.rs` -> dependency-free ISO date helpers (UTC-normalized).
  - `templates.rs` -> scaffold content for a fresh store.
  - `tokens.rs` -> dependency-free token-count estimate for budget checks.
  - `serialize.rs` -> pluggable structured output: `json.rs` (default),
    `toon.rs` (opt-in via `--format toon`).
  - `json.rs` / `yaml.rs` / `toon.rs` -> hand-rolled value types and
    encoders, no crates.
  - `meta.rs` -> `.agnosgram/meta/` (tool friction), strictly separate from
    host-project memory; never read by `pack`/`show`/`advise`.
  - `output.rs` -> `UserError`, `info`/`warn`/`print_structured`.

## Tests

- Unit tests live inline (`#[cfg(test)] mod tests`) next to the code they
  cover.
- `rust/tests/*_conformance.rs` -> the CLI-surface conformance suite
  (decisions/0004): spawns the real binary via
  `env!("CARGO_BIN_EXE_agnosgram")`, or `AGNOSGRAM_BIN` as a runtime
  override, and asserts exact stdout/stderr/exit-code/on-disk behavior -
  never imports command internals. `rust/tests/common/mod.rs` is the shared
  harness. `rust/tests/CONFORMANCE_MAP.md` traces every ported case back to
  the retired TS conformance test it came from.
- `rust/tests/bench_tokens.rs` -> the Tier-1 token-budget regression gate
  (lessons/pitfalls.md LES-002); `bench/` holds the fixtures it measures.

## Invariants

- **The CLI never calls an LLM and touches no network.** Judgment steps
  (`distill`, `bootstrap`, `advise`, `reflect`) emit prompts; they never
  call an API.
- **Zero runtime dependencies**, same constraint as the retired TS
  implementation (decisions/0001) - no `[dependencies]`, no
  `[dev-dependencies]`; JSON, YAML, TOON, frontmatter, and CLI parsing are
  all hand-rolled.
- **`adapt` is idempotent** and only ever rewrites the managed block; user
  content outside the markers is preserved byte-for-byte.
- **Detection is advisory, never load-bearing.** A wrong/missing detection
  can only change hint lines, never corrupt memory.
- **The on-disk format (decisions/0002) and CLI surface (decisions/0004)
  are both frozen.** A behavior change to either needs a superseding
  decision record, not a quiet patch.
