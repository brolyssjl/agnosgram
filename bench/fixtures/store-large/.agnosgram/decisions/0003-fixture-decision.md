---
id: DEC-0003
type: decision
scope: [frontend]
confidence: high
created: 2026-06-01
last_verified: 2026-06-03
source: journal/2026-06.md
---
## Context
Teams kept reinventing pagination shapes.

## Decision
Standardize on cursor-based pagination with an opaque, signed cursor for every list endpoint.

## Consequences
No more "page N of M" drift under concurrent writes; clients cannot jump to an arbitrary page.
