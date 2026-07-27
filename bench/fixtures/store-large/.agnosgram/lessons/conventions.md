# Conventions

---
id: CON-001
type: convention
scope: [core]
confidence: high
created: 2026-06-01
last_verified: 2026-06-01
source: journal/2026-06.md
---
Always validate input at the boundary (API handler), never deep in a service - keeps error messages actionable.

---
id: CON-002
type: convention
scope: [backend]
confidence: medium
created: 2026-06-01
last_verified: 2026-06-02
source: journal/2026-06.md
---
Always use structured logging (key=value) instead of string interpolation, so the log aggregator can index fields.

---
id: CON-003
type: convention
scope: [frontend]
confidence: high
created: 2026-06-01
last_verified: 2026-06-03
source: journal/2026-06.md
---
Always write a rollback plan before a schema migration, even a 'safe' additive one.

---
id: CON-004
type: convention
scope: [tooling]
confidence: low
created: 2026-06-01
last_verified: 2026-06-04
source: journal/2026-06.md
---
Always prefer composition over inheritance for shared behavior across handlers.

---
id: CON-005
type: convention
scope: [auth]
confidence: medium
created: 2026-06-01
last_verified: 2026-06-05
source: journal/2026-06.md
---
Always name background jobs after the outcome they produce, not the trigger that started them.

---
id: CON-006
type: convention
scope: [infra]
confidence: high
created: 2026-06-01
last_verified: 2026-06-06
source: journal/2026-06.md
---
Always attach a request id to every log line in a request's lifecycle, generated once at the edge.

---
id: CON-007
type: convention
scope: [core]
confidence: low
created: 2026-06-01
last_verified: 2026-06-07
source: journal/2026-06.md
---
Always default new feature flags to off and flip them on per-environment explicitly.

---
id: CON-008
type: convention
scope: [backend]
confidence: medium
created: 2026-06-01
last_verified: 2026-06-08
source: journal/2026-06.md
---
Always write the failure-path test before the happy-path test for anything touching money.

---
id: CON-009
type: convention
scope: [frontend]
confidence: high
created: 2026-06-01
last_verified: 2026-06-09
source: journal/2026-06.md
---
Always version an external API contract explicitly rather than relying on additive-only conventions.

---
id: CON-010
type: convention
scope: [tooling]
confidence: low
created: 2026-06-01
last_verified: 2026-06-10
source: journal/2026-06.md
---
Always put a circuit breaker in front of a dependency that has ever caused a cascading outage.

---
id: CON-011
type: convention
scope: [auth]
confidence: medium
created: 2026-06-01
last_verified: 2026-06-11
source: journal/2026-06.md
---
Always emit a metric for a background job's success/failure counts, not just its duration.

---
id: CON-012
type: convention
scope: [infra]
confidence: high
created: 2026-06-01
last_verified: 2026-06-12
source: journal/2026-06.md
---
Always document a non-obvious performance trade-off next to the code that makes it, not only in a design doc.

---
id: CON-013
type: convention
scope: [core]
confidence: low
created: 2026-06-01
last_verified: 2026-06-13
source: journal/2026-06.md
---
Always run the linter and the type checker in CI before the test suite, so cheap checks fail fast.

---
id: CON-014
type: convention
scope: [backend]
confidence: medium
created: 2026-06-01
last_verified: 2026-06-14
source: journal/2026-06.md
---
Always prefer an explicit allowlist over a denylist for anything security-sensitive.

