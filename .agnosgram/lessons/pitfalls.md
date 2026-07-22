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
last_verified: 2026-07-21
source: journal/2026-07.md
---
Do not assert the design-session TOON "+5% on small objects" finding against
`bench/bench.mjs` - its serializer is a toy, not real TOON, and cannot reproduce
that overhead. The bench only honestly guards the uniform-array (tabular) win.
Faithful small-object numbers need the real `@toon-format` encoder (Milestone 2).
