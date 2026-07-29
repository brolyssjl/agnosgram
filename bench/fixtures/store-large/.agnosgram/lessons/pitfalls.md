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

---
id: LES-004
type: pitfall
scope: [tooling]
confidence: low
created: 2026-06-01
last_verified: 2026-06-04
source: journal/2026-06.md
---
Do not trust client-supplied pagination limits without a server-side cap; an unbounded request once exhausted memory.

---
id: LES-005
type: pitfall
scope: [auth]
confidence: medium
created: 2026-06-01
last_verified: 2026-06-05
source: journal/2026-06.md
---
Do not log full request bodies in production - a customer's auth token ended up in the log aggregator once.

---
id: LES-006
type: pitfall
scope: [infra]
confidence: high
created: 2026-06-01
last_verified: 2026-06-06
source: journal/2026-06.md
---
Do not retry a non-idempotent write without a dedupe key; a network blip caused a double-charge in staging once.

---
id: LES-007
type: pitfall
scope: [core]
confidence: low
created: 2026-06-01
last_verified: 2026-06-07
source: journal/2026-06.md
---
Do not assume the cache and the database agree after a partial deploy; a stale read served the wrong price for an hour.

---
id: LES-008
type: pitfall
scope: [backend]
confidence: medium
created: 2026-06-01
last_verified: 2026-06-08
source: journal/2026-06.md
---
Do not run destructive cleanup jobs on a cron without a dry-run flag; one job once deleted a week of uploads.

---
id: LES-009
type: pitfall
scope: [frontend]
confidence: high
created: 2026-06-01
last_verified: 2026-06-09
source: journal/2026-06.md
---
Do not swallow errors in a fire-and-forget background task; a silent failure hid a broken export for a month.

---
id: LES-010
type: pitfall
scope: [tooling]
confidence: low
created: 2026-06-01
last_verified: 2026-06-10
source: journal/2026-06.md
---
Do not hardcode a region or timezone in a scheduler; a daylight-saving shift once fired a job an hour early.

---
id: LES-011
type: pitfall
scope: [auth]
confidence: medium
created: 2026-06-01
last_verified: 2026-06-11
source: journal/2026-06.md
---
Do not share a single database connection across worker threads; a leaked transaction once wedged the pool.

---
id: LES-012
type: pitfall
scope: [infra]
confidence: high
created: 2026-06-01
last_verified: 2026-06-12
source: journal/2026-06.md
---
Do not deploy a schema change and a code change that depends on it in the same release without a feature flag.

---
id: LES-013
type: pitfall
scope: [core]
confidence: low
created: 2026-06-01
last_verified: 2026-06-13
source: journal/2026-06.md
---
Do not trust wall-clock time for ordering distributed events; use a monotonic sequence or vector clock instead.

---
id: LES-014
type: pitfall
scope: [backend]
confidence: medium
created: 2026-06-01
last_verified: 2026-06-14
source: journal/2026-06.md
---
Do not let a feature flag default to "on" in a new environment; an unfinished feature once shipped to staging by accident.

---
id: LES-015
type: pitfall
scope: [frontend]
confidence: high
created: 2026-06-01
last_verified: 2026-06-15
source: journal/2026-06.md
---
Do not assume an idempotency key is unique forever; a key reused after a long TTL once caused a duplicate refund.

---
id: LES-016
type: pitfall
scope: [tooling]
confidence: low
created: 2026-06-01
last_verified: 2026-06-16
source: journal/2026-06.md
---
Do not batch writes without a size cap; an unbounded batch once exceeded the database's max packet size.

---
id: LES-017
type: pitfall
scope: [auth]
confidence: medium
created: 2026-06-01
last_verified: 2026-06-17
source: journal/2026-06.md
---
Do not parse a third-party webhook payload without validating its signature; an unsigned test payload once triggered a real charge.

---
id: LES-018
type: pitfall
scope: [infra]
confidence: high
created: 2026-06-01
last_verified: 2026-06-18
source: journal/2026-06.md
---
Do not assume a queue delivers messages exactly once; a redelivery once processed the same order twice.

---
id: LES-019
type: pitfall
scope: [core]
confidence: low
created: 2026-06-01
last_verified: 2026-06-19
source: journal/2026-06.md
---
Do not rely on client clocks for rate limiting; a skewed client clock once bypassed the limiter entirely.

---
id: LES-020
type: pitfall
scope: [backend]
confidence: medium
created: 2026-06-01
last_verified: 2026-06-20
source: journal/2026-06.md
---
Do not remove a deprecated API field before every known consumer has migrated; one internal script broke silently for weeks.

