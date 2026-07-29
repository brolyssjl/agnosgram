---
id: DEC-0005
type: decision
scope: [auth]
confidence: medium
created: 2026-06-01
last_verified: 2026-06-05
source: journal/2026-06.md
---
## Context
Secrets were scattered across environment files and a few hardcoded defaults.

## Decision
All secrets load from a central secrets manager at boot; the process refuses to start without them.

## Consequences
No more accidental commits of a real secret; local dev needs a documented bootstrap step.
