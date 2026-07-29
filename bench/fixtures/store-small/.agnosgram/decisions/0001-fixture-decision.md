---
id: DEC-0001
type: decision
scope: [core]
confidence: high
created: 2026-06-01
last_verified: 2026-06-01
source: journal/2026-06.md
---
## Context
We needed a consistent retry policy across services.

## Decision
Use exponential backoff with jitter, capped at 5 retries, for every outbound call.

## Consequences
Slightly higher latency on transient failures; far fewer thundering-herd incidents.
