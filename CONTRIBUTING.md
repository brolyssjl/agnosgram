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
3. **Before opening the PR:** `npm test` and `node bench/bench.mjs --check` must pass.
4. **Open a PR.** In the description, tick or reference the [ROADMAP.md](ROADMAP.md)
   item it advances. CI (build + test + bench gate, Node 20 & 22) must be green.
5. **Merge** into `main` (squash preferred). Then update the roadmap checkbox.

## Conformance mode

Starting at `0.11.0`, the CLI surface is frozen ahead of a planned Rust port
(Milestone 6): the TypeScript implementation is the reference, and the test
suite that exercises the CLI surface is written to be **binary-agnostic**, so
the same tests can later validate a from-scratch reimplementation.

Two kinds of test live side by side under `src/`:

- **`*.test.ts`** - unit tests of internals (pure functions, the `doctor`
  findings engine, YAML/TOON encoders, etc). These import command modules
  directly and run in-process. They are not part of conformance, since they
  test implementation, not surface.
- **`*.conformance.test.ts`** - tests of the CLI surface itself: argv
  parsing, stdout/stderr, exit codes, and on-disk effects. These never import
  command internals - they spawn a binary via the shared harness in
  `src/conformance/harness.ts` (`runCli(args, { cwd, input?, env? })`) and
  assert only on what a real invocation produces.

`npm test` runs everything. `npm run conformance` runs only the
`*.conformance.test.ts` subset, against whichever binary `AGNOSGRAM_BIN`
points at:

```bash
# Default: build this checkout, run its dist/cli.js
npm run conformance

# Validate an installed release, or a from-scratch reimplementation
AGNOSGRAM_BIN=$(command -v agnosgram) npm run conformance
AGNOSGRAM_BIN="/path/to/agnosgram-rs" npm run conformance
```

`AGNOSGRAM_BIN` is either a directly-executable binary or a `command arg...`
string (split on whitespace); it defaults to `node dist/cli.js` from this
checkout. When adding a new command or flag: if the behavior is reachable
only through the CLI (argv parsing, formatted output, exit codes, file
side-effects), it belongs in a `*.conformance.test.ts`; internals that are
also unit-testable in isolation (and aren't already covered by a conformance
test) can additionally get a plain `*.test.ts`. A `*.conformance.test.ts`
file must only ever import the harness and Node builtins - never a command
or `core/` module - or it stops being binary-agnostic.

The frozen surface has **no short options** (no `-b` for `--budget` or
similar) on any command - every flag is long-form only. `parseCliArgs`'s
short-option glue path (`src/core/args.ts`) is dead code against the real
surface; it's kept, with its own unit test, only because `node:util`'s
`parseArgs` supports short aliases generically and a future flag could add
one. The Rust port's argv parser does not need to implement short-option
handling to match this surface.

## Rust implementation

Starting with Milestone 6, the frozen CLI surface also has a from-scratch
Rust port living in `rust/` (crate `agnosgram`, `rust/Cargo.toml`, edition
2021, bin target `agnosgram`). See `docs/rust-port.md` for the full plan and
the module layout; the short version:

- **Zero dependencies, std only.** No external crates, mirroring the
  TypeScript implementation's zero-runtime-dependency policy. JSON, YAML,
  TOON, and frontmatter are all hand-rolled ports of the `core/*.ts`
  equivalents; local time uses a direct `extern "C"` binding to `localtime_r`
  instead of a crate. Do not add a `[dependencies]` entry without discussing it first
  - it breaks a deliberate reviewability property.
- **The CLI surface is frozen** (Milestone 6 owner decision, DEC-0004): the
  TypeScript implementation is the reference and the conformance suite
  (below) is the normative definition of correct behavior. The Rust port
  must match it byte-for-byte - stdout/stderr, exit codes, on-disk effects -
  not just "behave similarly."

Dev loop, from repo root:

```bash
cargo fmt --check --manifest-path rust/Cargo.toml
cargo clippy --manifest-path rust/Cargo.toml -- -D warnings
cargo test --manifest-path rust/Cargo.toml
cargo build --release --manifest-path rust/Cargo.toml
AGNOSGRAM_BIN="$PWD/rust/target/release/agnosgram" npm run conformance
```

All five must pass before opening a PR that touches `rust/`. Any change to
the CLI surface (a new command, flag, output string, or on-disk effect) must
land with a corresponding update to the conformance suite
(`*.conformance.test.ts`, see "Conformance mode" above) in the **same**
change, whichever implementation you touched first - the suite is what keeps
the two implementations from silently drifting apart.

## Releases

Releases are tag-driven. The `Release` workflow runs on any `v*.*.*` tag: it builds,
tests, runs the bench gate, and creates a GitHub release with auto-generated notes.

To cut a release:

1. On a branch, bump `version` in `package.json` and `rust/Cargo.toml` in lockstep,
   and tick the milestone's items in `ROADMAP.md`. Open + merge the PR.
2. Tag `main` and push the tag:
   ```bash
   git tag v1.0.1
   git push origin v1.0.1
   ```
3. The workflow publishes the GitHub release. A dormant `publish-npm` job exists
   behind `NPM_PUBLISH=true` + `NPM_TOKEN`, but per the Milestone 6 owner
   decision npm is permanently off the user-facing install path - that job is
   not enabled and is not part of the release story going forward.

### Release checkpoints (see ROADMAP.md)

| Version | Gate |
|---|---|
| `0.1.0` | Milestone 1 (done) - first GitHub release |
| `0.2.0` | First npm publish |
| `0.5.0` (beta) | Milestone 2 done + format freeze - first release safe on a real project |
| `0.8.0` (RC) | Milestone 3 done (`advise`, `pack`/`show`) |
| `0.9.0` | Milestone 4 done (adapters, Claude Code hooks, install.sh + binaries, SDD coexistence) |
| `1.0.0` | Milestone 6 Rust port, shipped on its conformance evidence (owner decision 2026-08-21; the rc soak became post-1.0.0 hardening) |

## npm publish (disabled)

The workflow still carries a `publish-npm` job (gated on `NPM_PUBLISH=true` +
`NPM_TOKEN`), but it is not enabled: the Milestone 6 owner decision makes
prebuilt binaries the permanent user-facing distribution and takes npm off
that path for good. Do not enable this job as a way to ship a user-facing
install method.
