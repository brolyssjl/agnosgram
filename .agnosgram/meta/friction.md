# Friction - tool-usage friction, not host-project memory

_Captured with `agnosgram feedback`. Never read by `pack`/`show`/`advise` -
this namespace is about Agnosgram itself, and feeds `agnosgram reflect`. See
docs/feedback.md._

---
id: FRI-001
type: friction
scope: [cli]
confidence: medium
created: 2026-07-30
last_verified: 2026-07-30
source: meta/friction.md
---
Commands with numeric options crash with a raw parseArgs stack trace when the value starts with a dash, e.g. `agnosgram pack --budget -1`. parseArgs interprets the leading dash as another option rather than a negative number. Affects every command with a numeric flag (pack --budget, reflect --months, advise timeouts if any are added later), not just the ones added in Milestone 5. Pre-existing on main; noted here for the first reflect run rather than fixed in this branch.
