---
id: DEC-0004
type: decision
scope: [tooling]
confidence: low
created: 2026-06-01
last_verified: 2026-06-04
source: journal/2026-06.md
---
## Context
Incident response needed a single source of truth for what changed and when.

## Decision
Every deploy writes a structured release event to the audit log, including the diff summary.

## Consequences
Faster incident timelines; a small amount of extra deploy latency.
