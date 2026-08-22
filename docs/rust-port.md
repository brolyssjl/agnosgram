# Rust port plan (Milestone 6 Round 2)

Working plan for porting the frozen CLI surface (DEC-0004) to Rust. This is a
contributor document; the user-facing contract is unchanged. Delete or archive
once `1.0.0` ships and the TypeScript implementation is retired.

**Status:** All waves landed 2026-08-20, and 1.0.0 shipped 2026-08-21 with the
Rust binary as canonical. This document remains as the port's design record.

## Goal and definition of done

One Rust crate, in-repo, producing a single static `agnosgram` binary with a
CLI surface identical to the TypeScript implementation at `0.11.0`. Done means:

```bash
cargo build --release --manifest-path rust/Cargo.toml
AGNOSGRAM_BIN="$PWD/rust/target/release/agnosgram" npm run conformance
```

passes unmodified, on darwin-arm64 and linux-x64. The conformance suite is the
normative surface definition (DEC-0004); this plan and the TS source are the
spec for everything the suite exercises.

## Layout

```
rust/
├── Cargo.toml          # package + bin "agnosgram", edition 2021
└── src/
    ├── main.rs         # argv dispatch, help/version, exit codes (mirrors src/cli.ts)
    ├── core/           # one module per src/core/*.ts file, same names
    │   ├── args.rs     # parseCliArgs port, incl. Node parseArgs emulation
    │   ├── yaml.rs     # the hand-rolled YAML subset
    │   ├── frontmatter.rs
    │   ├── json.rs     # hand-rolled JSON emit (JSON.stringify(v, null, 2)) + parse
    │   ├── toon.rs
    │   ├── serialize.rs
    │   ├── output.rs   # info/warn/UserError, print_structured
    │   ├── dates.rs    # todayIso (UTC) + local timestamp (libc localtime_r FFI)
    │   ├── templates.rs
    │   ├── config.rs
    │   ├── store.rs
    │   ├── records.rs
    │   ├── detect.rs
    │   └── lint.rs
    ├── adapters.rs     # src/adapters/index.ts
    ├── claude_hooks.rs # src/core/claudeHooks.ts
    └── commands/       # one module per src/commands/*.ts, same names
```

The 1:1 file mapping is deliberate: it makes porting, review, and later
divergence-hunting mechanical. Port each file against its TS counterpart.

## Dependency policy: std only

Zero external crates, mirroring the TS implementation's zero runtime
dependencies (a deliberate reviewability property, see DEC-0001 for the same
choice on YAML). Consequences:

- **JSON**: hand-rolled in `core/json.rs`. Emit must byte-match
  `JSON.stringify(value, null, 2)`: 2-space indent, `": "` after keys,
  insertion-ordered object keys (use a Vec-backed ordered map, never a
  HashMap), minimal escaping exactly as JS does it (`"` `\\` `\b` `\f` `\n`
  `\r` `\t`, other control chars as `\u00XX`, non-ASCII passed through
  verbatim, lone surrogates as `\uXXXX`), integers without a decimal point.
  Parse is needed by `advise --validate` (and anywhere else the TS parses
  JSON); accept what `JSON.parse` accepts for the shapes we read.
- **Local time**: `core/dates.rs` uses a direct `extern "C"` binding to
  `localtime_r` (identical leading `struct tm` layout on macOS and glibc; we
  only target those two). No chrono. `todayIso`/`journalMonth` use UTC math
  from `SystemTime` (days-from-epoch conversion, no tz involved).
- Everything else (fs, process, env) is std.

## Exactness contract (what "identical surface" means here)

1. **Exit codes**: 0 success, 1 `UserError`/reported failure, 2 unknown
   command. `main.rs` mirrors `src/cli.ts` exactly, including bare
   `agnosgram`, `-h`/`--help`/`help`, `-v`/`--version`/`version` handling and
   the help text byte-for-byte.
2. **Arg parsing**: port `src/core/args.ts` semantics: the dash-leading-value
   rewrite (with its `--` terminator and known-flag guards, long options join
   with `=`, short options glue directly), then a strict parseArgs-equivalent.
   Error strings must match the FIRST LINE of Node `parseArgs` errors
   verbatim, because `parseCliArgs` surfaces exactly that line as the
   `UserError` message. Verified empirically against Node 20 (the multi-line
   hints Node appends are dropped by `firstLine`; only these survive):
   - `Unknown option '--x'` (same shape for short: `Unknown option '-z'`)
   - `Option '--x <value>' argument missing`
   - `Option '--x' does not take an argument`
   - `Unexpected argument 'x'. This command does not take positional arguments`
   Re-verify any case you are unsure of with a `node -e` experiment; the
   conformance tests win over both this list and intuition.
3. **Output**: stdout/stderr strings ported verbatim from the TS source
   (every `info(...)`/`warn(...)`/`UserError(...)` literal). `--json` /
   `--format` output via the JSON emitter above; TOON via a faithful
   `core/toon.rs` port.
4. **Filesystem effects**: same files written with the same contents,
   including trailing newlines, template bodies (`core/templates.ts`
   literals), and the managed marker upsert semantics (`adapt`).
5. **Journal timestamps**: `log` uses LOCAL time `YYYY-MM-DD HH:MM` exactly
   as `src/commands/log.ts` does.

When TS behavior and this document disagree, the TS source and the
conformance suite win, in that order of convenience and that reversed order
of authority.

## Build/test loop

```bash
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"   # this machine
cargo fmt --check && cargo clippy -- -D warnings   # style gate
cargo test --manifest-path rust/Cargo.toml         # Rust unit tests
npm run build                                      # TS still builds (untouched)
AGNOSGRAM_BIN="$PWD/rust/target/release/agnosgram" npm run conformance
```

Rust unit tests port the TS unit tests for the hand-rolled parsers (args,
yaml, frontmatter, toon, json at minimum) so parser fidelity does not rest on
the conformance suite alone.

## Waves

1. **Scaffold + core**: crate, `main.rs` dispatch with all 11 commands
   stubbed (stub = `UserError("not yet ported")`, exit 1), help/version
   working, all `core/` modules ported with unit tests. Conformance not
   expected to pass yet.
2. **Commands**, three disjoint groups (no shared files beyond what wave 1
   froze): (a) `init`, `adapt` + `adapters.rs` + `claude_hooks.rs`;
   (b) `log`, `show`, `pack`, `doctor`; (c) `distill`, `bootstrap`, `advise`,
   `feedback`, `reflect`.
3. **Conformance closure**: run the full suite against the Rust binary, fix
   every diff, port remaining unit parity, `cargo fmt`/`clippy` clean.
4. **Pipeline**: `release.yml` builds Rust binaries on tag for both targets
   (replacing the bun/SEA artifacts once conformance is the gate), `ci.yml`
   gains a Rust job (fmt, clippy, test, conformance). `install.sh` unchanged.

Waves 2a/2b/2c only touch their own `commands/*.rs` files; wave 1
pre-registers every module so no later wave edits shared files.
