---
id: DEC-0002
type: decision
scope: [core, format]
confidence: high
created: 2026-07-23
last_verified: 2026-07-23
source: journal/2026-07.md
---

# Freeze the `.agnosgram/` layout and frontmatter schema at `0.5.0`

## Context
`0.5.0` is the first release meant to be adopted on a real, long-lived project.
For memory to be trustworthy it must not break under the user: files written today
must still be readable by every later version. Milestone 2 added the last pieces of
the on-disk contract (the record schema, `staleness_days`, the archive location),
so this is the moment to declare it stable.

## Decision
The following on-disk contract is **frozen as format version 1**. Changes may only
*add* optional fields or files; they may never remove, rename, or repurpose what is
below, and never make an old store fail to load.

### Directory layout
```
.agnosgram/
  MEMORY.md            reading protocol + freshness table (file | last verified | budget)
  config.yml           version, journal, staleness_days, budgets, adapters, sdd
  state/status.md      small, volatile, overwritten freely
  context/             architecture.md, stack.md, domain.md (+ optional files)
  decisions/           README.md + NNNN-slug.md ADRs (one record each)
  lessons/             pitfalls.md, conventions.md (many records each)
  journal/             YYYY-MM.md append-only; archive/ holds distilled months
```

### Record frontmatter (lessons and decisions)
A record is a YAML frontmatter block fenced by `---` lines, then a Markdown body.
Required: `id` (`^[A-Z]{2,}-\d{2,}$`), `type` (`pitfall|convention|decision`),
`scope` (non-empty list), `confidence` (`low|medium|high`), `created` and
`last_verified` (`YYYY-MM-DD`), `source` (a store-relative path). Optional:
`supersedes` (an id or list of ids). `scope`/`supersedes` may use inline flow
sequences (`[a, b]`). Frontmatter inside HTML comments or fenced code is not a
record.

### config.yml
Keys: `version` (1), `journal.committed` (bool), `staleness_days` (int, default
120), `budgets` (path -> token int), `adapters` and `sdd` (name -> `auto|on|off`).
Unknown keys are ignored on load, so adding keys is backward compatible.

## Consequences
- `doctor` is the executable specification of this contract: if `doctor` passes,
  the store conforms. Any schema change must land with a `doctor` change and tests.
- New capabilities (Milestone 3+) must extend, never break, the above. A breaking
  change would require a `version: 2` and a documented migration - avoided by design.
- Token budgets use a heuristic estimate (see `src/core/tokens.ts`), not a real
  tokenizer, to preserve the zero-runtime-dependency guarantee (DEC-0001).
