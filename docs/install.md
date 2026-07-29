# Install

## npm (recommended if you already have Node)

```bash
npm i -g agnosgram      # daily use
npx agnosgram init      # zero-install trial
```

Requires Node >= 20. Zero runtime dependencies either way.

## install.sh (no local npm install)

```bash
curl -fsSL https://raw.githubusercontent.com/brolyssjl/agnosgram/main/install.sh | bash
```

This fetches the single-file binary for your platform (`linux-x64` or
`darwin-arm64`) from the latest GitHub release and places it at
`~/.local/bin/agnosgram` (override with `AGNOSGRAM_INSTALL_DIR`). If your platform
has no matching release asset yet, or curl can't reach GitHub's API, it falls back
to `npm install -g agnosgram` when Node is available - otherwise it tells you what
to install by hand. Nothing here needs network access again after install: the CLI
itself never calls out.

Read the script before piping it into `bash` if you'd rather - it's a plain,
short, `set -euo pipefail` shell script with no hidden steps.

## Building the binary yourself

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
code signature, since `postject` can't modify a signed binary in place.

Verify a fresh build actually works end to end:

```bash
mkdir /tmp/agnosgram-smoke && cd /tmp/agnosgram-smoke && git init -q
/path/to/dist-bin/agnosgram init
/path/to/dist-bin/agnosgram pack
/path/to/dist-bin/agnosgram doctor
```

## Releases

Pushing a `v*.*.*` tag runs `.github/workflows/release.yml`: it builds
`agnosgram-linux-x64` and `agnosgram-darwin-arm64` binaries (via the same
`build:binary` script, one per matrix OS), runs the full test/bench gate, and
attaches both binaries to a GitHub release. `npm publish` stays a separate,
owner-gated job (`NPM_PUBLISH=true` repo variable + `NPM_TOKEN` secret) - the
release workflow never publishes on its own.
