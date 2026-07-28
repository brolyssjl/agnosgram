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
