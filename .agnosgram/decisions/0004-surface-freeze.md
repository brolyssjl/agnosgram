---
id: DEC-0004
type: decision
scope: [core, cli]
confidence: high
created: 2026-08-19
last_verified: 2026-08-19
source: journal/2026-08.md
---

# Freeze the CLI surface at `0.11.0` as the Rust port contract

## Context
Milestone 6 (`ROADMAP.md`) ships `1.0.0` as a Rust reimplementation, ported against
a frozen CLI surface with the existing test suite doubling as a cross-implementation
conformance suite. Round 1 did the work that makes freezing honest: a shared
`parseCliArgs` wrapper so every command fails the same way on bad input (FRI-001), a
symlink-aware `adapt` so `CLAUDE.md`<->`AGENTS.md` collapse to one honest write
(FRI-003), and conformance mode - `npm run conformance` spawns a binary via
`AGNOSGRAM_BIN` and exercises it as a real user would, never importing command
internals. With those three pieces merged (PR #8), the surface they exercise is
worth declaring stable so the Rust port has a fixed target instead of a moving one.

## Decision
The CLI surface exposed by the TypeScript implementation at `0.11.0` is **frozen as
the port contract** for the Milestone 6 Rust implementation. Frozen means: a
conforming implementation must reproduce every command's flags, stdout and stderr
text, exit codes, and `--json` output shapes exactly as exercised by the
`*.conformance.test.ts` suite. Changes to the frozen surface require a new decision
record explicitly superseding this one, not a quiet patch during the port.

### Frozen commands
All 11 commands `agnosgram` currently exposes (`src/cli.ts`):
`init`, `adapt`, `log`, `doctor`, `distill`, `bootstrap`, `show`, `pack`, `advise`,
`feedback`, `reflect` - plus the global `-h`/`--help`, `-v`/`--version`, and
`--format <fmt>` flags. Every flag on every command is long-form only; short
options are not part of the frozen surface (see `CONTRIBUTING.md`).

### Frozen behavior
- **Exit codes:** `0` on success, `1` on a reported command failure (a thrown
  `UserError`, `doctor` findings, `distill` validation errors, `advise`
  contradictions, `show` with no matches), `2` on an unknown command.
- **`--json` output shapes:** the structured payload each command prints via
  `printStructured` when `--json`/`--format json` is given is part of the surface,
  field names and all, not just the human-readable text.
- **On-disk effects:** which files a command writes or leaves untouched (for
  example, `adapt`'s single-write symlink collapse) is part of the surface.

### The conformance suite is the normative definition
`npm run conformance` (documented in `CONTRIBUTING.md`) is the executable
specification of this freeze, the same role `doctor` plays for the on-disk format
(`DEC-0002`). It spawns whatever binary `AGNOSGRAM_BIN` points at - by default this
checkout's `dist/cli.js`, but the same suite must pass unmodified against the
eventual Rust binary. A `*.conformance.test.ts` file may only import the harness
(`src/conformance/harness.ts`) and Node builtins; if it imports a command or
`core/` module it is testing implementation, not surface, and does not belong here.

### The format contract is unchanged
This freeze covers the CLI surface only. The on-disk `.agnosgram/` layout and
frontmatter schema frozen by `DEC-0002` is unaffected, unchanged, and remains
non-negotiable - the Rust port must read and write stores exactly as the TypeScript
implementation does today.

## Consequences
- Milestone 6 Round 2 (the Rust port) has a fixed target: match the conformance
  suite, not the TypeScript source. Any surface change discovered to be necessary
  during the port (a flag that can't be replicated, an output shape that leaked an
  implementation detail) needs a new decision superseding this one before landing.
- New commands or flags added after this freeze are non-breaking additions and do
  not themselves require a new decision, but must land with a corresponding
  `*.conformance.test.ts` update - the suite must always equal the frozen surface,
  never trail it.
- `1.0.0` retires or demotes the TypeScript implementation once the Rust binary is
  the canonical distribution (`ROADMAP.md` Milestone 6, Round 3); until then the
  TypeScript implementation stays the reference the conformance suite is written
  against.
