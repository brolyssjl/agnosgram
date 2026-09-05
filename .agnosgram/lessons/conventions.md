# Conventions — "always do Y"

_Patterns that worked, worth repeating. Same frontmatter schema as pitfalls._

---
id: CON-001
type: convention
scope: [core, tooling]
confidence: high
created: 2026-07-21
last_verified: 2026-07-21
source: journal/2026-07.md
---
Reach for Node built-ins before any dependency: `node:util` `parseArgs` for CLI
args, `node:test` + `node:assert/strict` for tests. This kept Milestone 1 at zero
runtime deps and only three dev deps. See DEC-0001.

---
id: CON-002
type: convention
scope: [adapters]
confidence: high
created: 2026-07-21
last_verified: 2026-07-21
source: journal/2026-07.md
---
Any write into a user-shared file (CLAUDE.md, AGENTS.md) must go through
`upsertManagedBlock` so it stays idempotent and never clobbers user content.
Add a golden test asserting run-twice-is-identical for every new adapter target.

---
id: CON-003
type: convention
scope: [core, tooling]
confidence: high
created: 2026-07-23
last_verified: 2026-07-23
source: journal/2026-07.md
---
`doctor` is the executable specification of the frozen format (DEC-0002): if it
passes, a store conforms. Any change to the schema, budgets, or store layout must
land together with a `doctor` change and its tests - never one without the other.
Run `agnosgram doctor --strict` before committing changes to `.agnosgram/`.

---
id: CON-004
type: convention
scope: [install, release]
confidence: high
created: 2026-09-05
last_verified: 2026-09-05
source: journal/2026-08.md
---
install.sh is curl-first against public release-redirect URLs with a `gh`
fallback for the private-repo case - so it needs zero changes when the
repo goes public. Keep that shape for any new download path, and keep the
platform list in sync with release.yml (header comment contract).

---
id: CON-005
type: convention
scope: [store, friction]
confidence: high
created: 2026-09-05
last_verified: 2026-09-05
source: journal/2026-08.md
---
Friction-resolution convention: when a meta/friction.md record is fixed,
keep the original paragraph verbatim, prepend a "RESOLVED (version): ..."
note above it inside the same record body, and bump only last_verified -
id/created/scope/confidence stay untouched.

---
id: CON-006
type: convention
scope: [release]
confidence: high
created: 2026-09-05
last_verified: 2026-09-05
source: journal/2026-08.md
---
Before tagging a release, re-run the full local matrix (cargo fmt/clippy/
test plus the conformance suite against the rebuilt binary) - the
release workflow's tag/Cargo.toml version guard only catches metadata
drift, not behavior. One manual version bump site per file family
(main.rs compiles VERSION from CARGO_PKG_VERSION).
