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
