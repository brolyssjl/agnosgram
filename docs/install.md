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
`darwin-arm64`) from `github.com/<repo>/releases/latest/download/<asset>` - a
redirect straight to the current release's asset, no separate API call to
resolve the tag first - and places it at `~/.local/bin/agnosgram` (override with
`AGNOSGRAM_INSTALL_DIR`). The download goes to a temp file first and is only
moved into place once it succeeds, so a failed transfer never leaves a truncated
binary behind. If your platform or architecture has no matching release asset
yet, it falls back to `npm install -g agnosgram` when Node is available -
otherwise it tells you what to install by hand. Nothing here needs network
access again after install: the CLI itself never calls out.

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
