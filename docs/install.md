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

While the repository is private, the unauthenticated one-liner 404s - both
the raw script URL and the asset download. Run the script from a clone
instead: when the plain download fails it falls back to
`gh release download`, which reuses your GitHub auth and sees the same
assets. The curl path starts working for everyone the moment the repository
goes public.

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

Zero external crates (std only, same zero-runtime-dependency policy as the
TypeScript implementation) - needs only a stable Rust toolchain, no `bun` or
Node SEA tooling. This is exactly what `install.sh` tells you to do when
there is no matching binary asset for your platform.

Contributors working on the TypeScript reference implementation still need
Node >= 20 (`docs/rust-port.md`, `.agnosgram/decisions/0004-surface-freeze.md`):

```bash
npm run build            # tsc -> dist/
node dist/cli.js --help  # run directly
```

Verify a fresh build actually works end to end:

```bash
mkdir /tmp/agnosgram-smoke && cd /tmp/agnosgram-smoke && git init -q
/path/to/agnosgram init
/path/to/agnosgram pack
/path/to/agnosgram doctor
```

## Releases

Pushing a `v*.*.*` tag runs `.github/workflows/release.yml`:

- A `test` job first checks that the tag, `package.json` version, and
  `rust/Cargo.toml` version all agree, then runs the build/test/bench gate.
- A `conformance` job builds the Rust crate's release binary and runs the
  full conformance suite (`npm run conformance`) against it - a binary that
  fails its own surface contract never reaches a release.
- A `build-binaries` job builds `agnosgram-linux-x64` and
  `agnosgram-darwin-arm64` with `cargo build --release`, one per matrix OS.
  This job is best-effort - a Rust toolchain setup outage on one runner
  attaches whatever succeeded rather than blocking the release, since `test`
  and `conformance` are the hard gates.

`npm publish` stays a separate, owner-gated job (`NPM_PUBLISH=true` repo
variable + `NPM_TOKEN` secret) - the release workflow never publishes on its
own.
