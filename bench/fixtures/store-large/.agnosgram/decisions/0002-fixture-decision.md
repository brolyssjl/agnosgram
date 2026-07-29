---
id: DEC-0002
type: decision
scope: [backend]
confidence: medium
created: 2026-06-01
last_verified: 2026-06-02
source: journal/2026-06.md
---
## Context
Multiple services needed a shared identity format.

## Decision
Adopt ULIDs for all new primary keys instead of auto-increment integers.

## Consequences
Sortable by creation time; slightly larger index footprint.
