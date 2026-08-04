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

## Releases

Releases are tag-driven. The `Release` workflow runs on any `v*.*.*` tag: it builds,
tests, runs the bench gate, and creates a GitHub release with auto-generated notes.

To cut a release:

1. On a branch, bump `version` in `package.json` and tick the milestone's items in
   `ROADMAP.md`. Open + merge the PR.
2. Tag `main` and push the tag:
   ```bash
   git tag v0.5.0
   git push origin v0.5.0
   ```
3. The workflow publishes the GitHub release. npm publish runs only if the repo
   variable `NPM_PUBLISH=true` and secret `NPM_TOKEN` are set (owner-controlled).

### Release checkpoints (see ROADMAP.md)

| Version | Gate |
|---|---|
| `0.1.0` | Milestone 1 (done) - first GitHub release |
| `0.2.0` | First npm publish |
| `0.5.0` (beta) | Milestone 2 done + format freeze - first release safe on a real project |
| `0.8.0` (RC) | Milestone 3 done (`advise`, `pack`/`show`) |
| `0.9.0` | Milestone 4 done (adapters, Claude Code hooks, install.sh + binaries, SDD coexistence) |
| `1.0.0` | `0.9.0` + real-world soak |

## Enabling npm publish (owner)

1. Create an npm automation token; add it as the `NPM_TOKEN` repo secret.
2. Set repo variable `NPM_PUBLISH=true`.
3. Push the next version tag; the `publish-npm` job runs with provenance.
