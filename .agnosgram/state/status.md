# Status

_Small and volatile. Overwrite freely; history lives in the journal._

- **Focus:** `1.6.0` cut 2026-09-23 - the 2026-09-22 security-audit
  remediation (PRs #50-#54: strict managed blocks, read/write containment,
  parser depth limits, lint bypasses, attested releases, SECURITY.md); gate
  ships the matching `1.7.0` (per-machine trust store, write containment,
  stored-state validation, playbook confinement, scanner hardening). Both
  repos public since 2026-09-15; zero open issues.
- **In flight:** nothing.
- **Next:** the upgrade story (version-stamped managed blocks, `doctor`
  stale-artifact warnings, `adapt --refresh` diff) is the next scheduled
  build item - gate's `doctor`/`update` pair is the template. Then recruit an
  outside pilot user and the docs site + case study (both unblocked by
  going public). Keep the reflect cadence running each soak/milestone.
- **Blocked on:** nothing.

_Last updated: 2026-09-23_
