---
id: DEC-0006
type: decision
scope: [infra]
confidence: high
created: 2026-06-01
last_verified: 2026-06-06
source: journal/2026-06.md
---
## Context
Different teams used different error response shapes.

## Decision
Every API error responds with the same envelope: a stable code, a human message, and a request id.

## Consequences
Clients can branch on `code` reliably; existing bespoke error shapes need a migration window.
