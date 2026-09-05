---
id: DEC-0005
type: decision
scope: [install, release]
confidence: high
created: 2026-09-05
last_verified: 2026-09-05
source: journal/2026-08.md
---
# npm is permanently out of the install path

Owner decision (2026-08-21/22, Milestone 6): the Rust binary is the
canonical distribution - GitHub releases with checksummed assets installed
via install.sh, build-from-source as the fallback. npm publishing is
permanently retired, not deferred: no npm/npx install mentions in docs, no
npm fallback in install.sh (it could only 404), and CONTRIBUTING documents
the dormant publish workflow as permanently disabled. Never reintroduce
"once published to npm" phrasing - the decision is settled, not pending.
