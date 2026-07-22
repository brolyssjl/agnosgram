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
| `1.0.0` | Milestone 4 + real-world soak |

## Enabling npm publish (owner)

1. Create an npm automation token; add it as the `NPM_TOKEN` repo secret.
2. Set repo variable `NPM_PUBLISH=true`.
3. Push the next version tag; the `publish-npm` job runs with provenance.
