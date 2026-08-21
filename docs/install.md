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
it prints build-from-source instructions instead of attempting an npm install
that can't work yet - see below. Nothing here needs network access again after
install: the CLI itself never calls out.

While the repository is private, the unauthenticated one-liner 404s - both
the raw script URL and the asset download. Run the script from a clone
instead: when the plain download fails it falls back to
`gh release download`, which reuses your GitHub auth and sees the same
assets. The curl path starts working for everyone the moment the repository
goes public.

Read the script before piping it into `bash` if you'd rather - it's a plain,
short, `set -euo pipefail` shell script with no hidden steps.

## npm

```bash
npm i -g agnosgram      # once published
npx agnosgram init      # zero-install trial, once published
```

Requires Node >= 20, zero runtime dependencies either way. **Not published
yet**: per the Milestone 6 owner decision (see `.agnosgram/decisions/` and
`ROADMAP.md`), prebuilt binaries are the canonical distribution and npm stays
off the user-facing install path, so `npm i -g agnosgram` 404s until that
changes. Use `install.sh` above, or build from source below, in the meantime.

### Read-only global npm prefix (nix, managed machines)

On a nix-managed machine, or any host where the npm global prefix is
read-only, `npm install -g` fails against that prefix and npm's error tells
you to run as root or fix permissions - misleading, since the actual problem
is the read-only prefix, not something `sudo` fixes. Two options that
actually work:

- `install.sh` above - it never touches the npm global prefix.
- Point npm at a writable prefix you own:
  `NPM_CONFIG_PREFIX=$HOME/.npm-global npm i -g agnosgram` (once published),
  then add `export PATH="$HOME/.npm-global/bin:$PATH"` to your shell profile.

## Build from source

```bash
git clone https://github.com/brolyssjl/agnosgram.git
cd agnosgram
npm ci
npm run build
node dist/cli.js --help   # run directly, or:
npm link                  # put `agnosgram` on PATH instead
```

Requires Node >= 20. This is exactly what `install.sh` tells you to do when
there is no matching binary asset for your platform.

## Building a binary yourself

The canonical way to build a single-file binary is now the Rust crate in
`rust/` (see `docs/rust-port.md`):

```bash
cargo build --release --manifest-path rust/Cargo.toml
# -> rust/target/release/agnosgram
```

Zero external crates (std only, same zero-runtime-dependency policy as the
TypeScript implementation), no `bun` or Node SEA tooling required. The
TypeScript implementation still works too, and stays the reference until the
Rust port replaces it (`docs/rust-port.md`, `.agnosgram/decisions/0004-surface-freeze.md`):

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
