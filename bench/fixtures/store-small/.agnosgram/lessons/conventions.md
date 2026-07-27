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

