# Contributing

## Workflow: feature branches + PRs (no direct pushes to `main`)

`main` is protected by convention (and, once enabled, by GitHub branch protection).
All changes land through pull requests.

1. **Branch** off `main`. Name it by type + short slug:
   - `feat/…` new command or capability
   - `fix/…` bug fix
   - `chore/…` tooling, CI, deps
   - `docs/…` docs / README / roadmap
2. **Make the change**, keep it scoped to one roadmap item where possible.
3. **Before opening the PR:** `cargo test --manifest-path rust/Cargo.toml` must pass
   (unit tests, the conformance suite, and the token benchmark gates all run under it).
4. **Open a PR.** In the description, tick or reference the [ROADMAP.md](ROADMAP.md)
   item it advances. CI (fmt + clippy + build + test, ubuntu & macos) must be green.
5. **Merge** into `main` (squash preferred). Then update the roadmap checkbox.

## Conformance mode

The CLI surface is frozen (Milestone 6 owner decision, DEC-0004): stdout,
stderr, exit codes, and on-disk effects are all part of the contract, not
just "current behavior." `rust/tests/*_conformance.rs` is the **behavioral
contract of the frozen CLI surface** - it is the normative definition of
correct behavior, not just a regression suite.

Two kinds of test live side by side in the Rust crate:

- **Unit tests** (`#[cfg(test)] mod tests` inside `rust/src/**/*.rs`) - test
  internals (pure functions, the `doctor` findings engine, YAML/TOON
  encoders, etc) in-process. They are not part of conformance, since they
  test implementation, not surface.
- **`rust/tests/*_conformance.rs`** - integration tests of the CLI surface
  itself: argv parsing, stdout/stderr, exit codes, and on-disk effects.
  These never call command internals - they spawn a binary via the shared
  harness in `rust/tests/common/mod.rs` (`run_cli(args, cwd)`) and assert
  only on what a real invocation produces. See `rust/tests/CONFORMANCE_MAP.md`
  for how each case traces back to the frozen surface it pins.

`cargo test --manifest-path rust/Cargo.toml` runs everything (unit +
conformance + the token benchmark gates). The conformance tests default to
the binary this checkout just built (via Cargo's `CARGO_BIN_EXE_agnosgram`),
but honor `AGNOSGRAM_BIN` as a runtime override to validate an installed
release or a different build instead:

```bash
# Default: builds and tests this checkout's own binary
cargo test --manifest-path rust/Cargo.toml

# Validate an installed release, or a differently-built binary
AGNOSGRAM_BIN=$(command -v agnosgram) cargo test --manifest-path rust/Cargo.toml
```

`AGNOSGRAM_BIN` is either a directly-executable binary or a `command arg...`
string (split on whitespace). When adding a new command or flag: if the
behavior is reachable only through the CLI (argv parsing, formatted output,
exit codes, file side-effects), it belongs in a `*_conformance.rs` test;
internals that are also unit-testable in isolation (and aren't already
covered by a conformance test) can additionally get a unit test next to the
code. A `*_conformance.rs` file must only ever import the harness and std -
never a `core`/`commands` module - or it stops being a black-box surface
check.

The frozen surface has **no short options** (no `-b` for `--budget` or
similar) on any command - every flag is long-form only.

## Rust implementation

Agnosgram is a single Rust crate living in `rust/` (crate `agnosgram`,
`rust/Cargo.toml`, edition 2021, bin target `agnosgram`). See
`docs/archive/rust-port.md` for the original port's design record
(historical - the TypeScript implementation it was ported from was retired
2026-08-22); the current-state short version:

- **Zero dependencies, std only.** No external crates - a deliberate
  reviewability property. JSON, YAML, TOON, and frontmatter are all
  hand-rolled; local time uses a direct `extern "C"` binding to
  `localtime_r` instead of a crate. `rust/tests/` integration tests hold to
  the same policy: std only, no dev-dependencies. Do not add a
  `[dependencies]` or `[dev-dependencies]` entry without discussing it first.
- **The CLI surface is frozen** (Milestone 6 owner decision, DEC-0004):
  `rust/tests/*_conformance.rs` is the normative definition of correct
  behavior. Any change to stdout/stderr, exit codes, or on-disk effects is a
  surface change and needs a deliberate decision, not an incidental one.

Dev loop, from repo root:

```bash
cargo fmt --check --manifest-path rust/Cargo.toml
cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path rust/Cargo.toml
cargo build --release --manifest-path rust/Cargo.toml
```

All four must pass before opening a PR. Any change to the CLI surface (a new
command, flag, output string, or on-disk effect) must land with a
corresponding update to `rust/tests/*_conformance.rs` in the **same**
change - the suite is what keeps the shipped binary from silently drifting
off its own documented contract.

## Releases

Releases are tag-driven. The `Release` workflow runs on any `v*.*.*` tag: it
verifies the tag matches `rust/Cargo.toml`, builds the release binary for
each platform and runs the full test suite (unit + conformance + token
benchmark gates) against that exact binary, then creates a GitHub release
with auto-generated notes. There is no npm publish path - prebuilt binaries
are the permanent, only user-facing install story (Milestone 6 owner
decision); a `publish-npm` job existed behind a disabled flag but was
deleted 2026-08-22 during TS retirement rather than kept dormant.

To cut a release:

1. On a branch, bump `version` in `rust/Cargo.toml` and tick the milestone's
   items in `ROADMAP.md`. Open + merge the PR.
2. Tag `main` and push the tag:
   ```bash
   git tag v1.0.1
   git push origin v1.0.1
   ```
3. The workflow builds, verifies, and publishes the GitHub release.

### Release checkpoints (see ROADMAP.md)

| Version | Gate |
|---|---|
| `0.1.0` | Milestone 1 (done) - first GitHub release |
| `0.5.0` (beta) | Milestone 2 done + format freeze - first release safe on a real project |
| `0.8.0` (RC) | Milestone 3 done (`advise`, `pack`/`show`) |
| `0.9.0` | Milestone 4 done (adapters, Claude Code hooks, install.sh + binaries, SDD coexistence) |
| `1.0.0` | Milestone 6 Rust port, shipped on its conformance evidence (owner decision 2026-08-21; the rc soak became post-1.0.0 hardening) |
