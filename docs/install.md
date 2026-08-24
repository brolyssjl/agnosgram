# Install

## install.sh (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/brolyssjl/agnosgram/main/install.sh | bash
```

This fetches the single-file binary for your platform (`linux-x64` or
`darwin-arm64`, built since `0.9.0`) from
`github.com/<repo>/releases/latest/download/<asset>` - a redirect straight to
the current release's asset, no separate API call to resolve the tag first -
and places it at `~/.local/bin/agnosgram` (override with
`AGNOSGRAM_INSTALL_DIR`). The download goes to a temp file first and is only
moved into place once it succeeds, so a failed transfer never leaves a truncated
binary behind. If your platform or architecture has no matching release asset,
it prints build-from-source instructions instead - see below. `agnosgram` is
not on npm and has no npm install path, ever (Milestone 6 owner decision).
Nothing here needs network access again after install: the CLI itself never
calls out.

If the plain download ever fails (rate limiting, a flaky network), run the
script from a clone instead: when the plain download fails it falls back to
`gh release download`, which reuses your GitHub auth and sees the same
assets.

Read the script before piping it into `bash` if you'd rather - it's a plain,
short, `set -euo pipefail` shell script with no hidden steps.

## Build from source

`agnosgram` is not on npm and never will be - the Milestone 6 owner decision
(see `.agnosgram/decisions/` and `ROADMAP.md`) makes prebuilt binaries the
permanent user-facing install path. If there is no matching binary asset for
your platform, build the Rust crate yourself:

```bash
git clone https://github.com/brolyssjl/agnosgram.git
cd agnosgram
cargo build --release --manifest-path rust/Cargo.toml
# -> rust/target/release/agnosgram
```

Zero external crates (std only) - needs only a stable Rust toolchain, no
other tooling. This is exactly what `install.sh` tells you to do when there
is no matching binary asset for your platform.

Verify a fresh build actually works end to end:

```bash
mkdir /tmp/agnosgram-smoke && cd /tmp/agnosgram-smoke && git init -q
/path/to/agnosgram init
/path/to/agnosgram pack
/path/to/agnosgram doctor
```

## Releases

Pushing a `v*.*.*` tag runs `.github/workflows/release.yml`:

- A `version-guard` job checks that the tag and `rust/Cargo.toml` version agree.
- A `build-and-verify` job builds `agnosgram-linux-x64` and
  `agnosgram-darwin-arm64` with `cargo build --release`, one per matrix OS,
  then runs the full test suite (unit + conformance + token benchmark gates)
  against that exact release binary on that platform - a binary that fails
  its own surface contract never reaches a release. Neither the build nor
  the test run is best-effort: either platform failing fails the release.
- A `release` job fails outright if zero assets were produced, otherwise
  publishes the GitHub release.

There is no npm publish path - prebuilt binaries are the permanent,
only user-facing install story (Milestone 6 owner decision).
