# Pitfalls — "do not do X"

_Distilled failures. Each entry carries frontmatter (see below) so `doctor` can
track staleness. Add via `distill`; edit by hand when you learn something now._

---
id: LES-001
type: pitfall
scope: [core, tooling]
confidence: high
created: 2026-07-21
last_verified: 2026-07-21
source: journal/2026-07.md
---
Do not use CommonJS `require()` in this package - it is ESM (`"type": "module"`)
with `verbatimModuleSyntax`. `require` fails at runtime. Use `import`. (Hit this in
a test helper during Milestone 1.)

---
id: LES-002
type: pitfall
scope: [tooling]
confidence: high
created: 2026-07-21
last_verified: 2026-07-27
source: journal/2026-07.md
---
RESOLVED (Milestone 3): `bench/bench.mjs` now measures the faithful
`@toon-format/toon` encoder (exact-pinned dev dep) alongside our hand-rolled
runtime encoder (`src/core/toon.ts`), so small-object numbers are honest -
see the `record` row, a realistic small object with a punctuation-heavy body.
`--check` gates "no net TOON win" on that `record` payload specifically (the
direction, not the loss's exact magnitude), plus a drift guard between the
two encoders on the uniform `lessons` array. That gate is payload-specific,
not a blanket claim: `status`, a different small object, actually wins
slightly. Kept for history: earlier in the project, `bench/bench.mjs` used a
toy serializer that could only honestly guard the uniform-array (tabular)
win, not small-object overhead.

---
id: LES-003
type: pitfall
scope: [store, process]
confidence: high
created: 2026-09-05
last_verified: 2026-09-05
source: journal/2026-08.md
---
status.md staleness compounds silently - it once survived a full milestone
and a full round unmodified because updating it was nobody's merge step.
Update status.md (and its MEMORY.md Freshness row) in the same PR that
merges the work it describes, never in a separate closeout pass. The
`status.stale`/`freshness.mismatch` doctor checks (1.3.0) now catch the
drift, but they fire after the fact; the convention prevents it.

---
id: LES-004
type: pitfall
scope: [install, release]
confidence: high
created: 2026-09-05
last_verified: 2026-09-05
source: journal/2026-08.md
---
Never declare an install path fixed from repo-side evidence alone - `gh
release view` showing assets proves nothing about the anonymous download
path (private repos 404 unauthenticated curl even for existing assets).
Verify install.sh end-to-end the way an outsider would run it before
closing install friction.

---
id: LES-005
type: pitfall
scope: [conformance, testing]
confidence: high
created: 2026-09-05
last_verified: 2026-09-05
source: journal/2026-08.md
---
A green conformance run is not proof of byte-equality: the suite mostly
asserts includes(), so byte-level claims (output parity, template
indentation, truncation behavior) need actual byte-diffs of produced
files/output, not test passes. Twin-dir diffs are what caught the real
divergences during the Rust port.
