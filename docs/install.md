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

## Building the (single-file) binary yourself

```bash
npm run build           # tsc -> dist/
npm run build:binary    # -> dist-bin/agnosgram
```

`build:binary` (`scripts/buildBinary.mjs`) prefers `bun build --compile` when
`bun` is on `PATH` - it bundles our zero-runtime-deps ESM output directly, no
extra steps. Without `bun`, it falls back to Node's built-in Single Executable
Applications (SEA) support: it first bundles `dist/`'s multi-file ESM graph into
one CJS file with `esbuild` (a devDependency used only by this script - it is
never shipped or used at runtime, so it doesn't touch the zero-runtime-dependency
rule), then injects that bundle into a copy of the running `node` binary via
`postject` (installed ad hoc: `npm install --no-save postject` - kept out of
`package.json` on purpose). On macOS this also strips and re-applies an ad hoc
code signature, since `postject` can't modify a signed binary in place. The
script always logs which engine it picked; set `AGNOSGRAM_BINARY_ENGINE=bun` or
`=sea` to force one instead of PATH-sniffing for `bun` (useful in CI, or to
reproduce a report against a specific engine).

Verify a fresh build actually works end to end:

```bash
mkdir /tmp/agnosgram-smoke && cd /tmp/agnosgram-smoke && git init -q
/path/to/dist-bin/agnosgram init
/path/to/dist-bin/agnosgram pack
/path/to/dist-bin/agnosgram doctor
```

## Releases

Pushing a `v*.*.*` tag runs `.github/workflows/release.yml`: a `test` job runs
the build/test/bench gate (the one hard requirement), in parallel with a
`build-binaries` job that builds `agnosgram-linux-x64` and `agnosgram-darwin-arm64`
(via the same `build:binary` script, one per matrix OS). Binaries are
best-effort - a bun-setup outage or SEA failure on one platform attaches
whatever succeeded rather than blocking the release or `npm publish`. `npm
publish` stays a separate, owner-gated job (`NPM_PUBLISH=true` repo variable +
`NPM_TOKEN` secret) - the release workflow never publishes on its own.
