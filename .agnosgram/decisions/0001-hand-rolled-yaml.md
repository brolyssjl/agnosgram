---
id: DEC-0001
type: decision
scope: [core, tooling]
confidence: high
created: 2026-07-21
last_verified: 2026-07-21
source: journal/2026-07.md
---

# Hand-rolled YAML subset instead of a dependency

## Context
`config.yml` and record frontmatter need YAML read/write. Every YAML library is a
runtime dependency, and the product thesis includes **zero runtime dependencies**
(security posture, supply-chain surface, "any agent that can read a file").

## Decision
Ship a small, well-tested YAML reader/writer (`src/core/yaml.ts`) covering only the
subset Agnosgram authors: nested maps (2-space indent), scalars, and block
sequences of scalars. Anything outside the subset throws rather than guessing.

## Consequences
- No runtime deps for config/frontmatter. Parser is ~150 LOC, unit-tested (round-trip).
- We must keep templates within the supported subset (no anchors, flow maps, multi-line scalars).
- If frontmatter needs grow past the subset, revisit - but prefer widening the parser with tests over adding a dep.
