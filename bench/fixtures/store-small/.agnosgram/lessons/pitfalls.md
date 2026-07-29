# Pitfalls

---
id: LES-001
type: pitfall
scope: [core]
confidence: high
created: 2026-06-01
last_verified: 2026-06-01
source: journal/2026-06.md
---
Do not call the external API without a timeout - a hung request once blocked the whole worker pool for minutes.

---
id: LES-002
type: pitfall
scope: [backend]
confidence: medium
created: 2026-06-01
last_verified: 2026-06-02
source: journal/2026-06.md
---
Do not mutate the shared config object at runtime; downstream callers cache a reference and silently pick up the change.

---
id: LES-003
type: pitfall
scope: [frontend]
confidence: high
created: 2026-06-01
last_verified: 2026-06-03
source: journal/2026-06.md
---
Do not skip the migration dry-run on a large table - an unindexed column scan took the primary down for a few minutes.

