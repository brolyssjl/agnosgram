# Schema reference (format version 1, frozen at 0.5.0)

The `.agnosgram/` store is plain Markdown + YAML. This is the stable contract:
files that validate today keep validating on every later version. `agnosgram
doctor` is the executable specification - if it passes, your store conforms.

## Directory layout

```
.agnosgram/
├── MEMORY.md            # reading protocol + freshness table
├── config.yml           # budgets, staleness window, adapter/SDD toggles
├── state/status.md      # current focus (small, volatile, overwritten freely)
├── context/             # architecture.md, stack.md, domain.md (+ optional files)
├── decisions/           # README.md + NNNN-slug.md ADRs (one record each)
├── lessons/             # pitfalls.md, conventions.md (many records each)
└── journal/             # YYYY-MM.md append-only; archive/ holds distilled months
```

## Record frontmatter

A **record** is a YAML frontmatter block fenced by `---` lines, followed by a
Markdown body. `lessons/*.md` hold many records; each `decisions/NNNN-*.md` holds
one. Frontmatter that appears inside an HTML comment or a fenced code block is
ignored (so templates and docs are not mistaken for real records).

A `---` line opens a record only when the lines up to the next `---` are all
YAML-shaped (`key: value`, list items, indented continuations). A Markdown
thematic break (`---` followed by prose) is ordinary body content - bodies may
contain horizontal rules and fenced code freely. Frontmatter values are
single-line scalars or lists; YAML block scalars (`|`, `>`) are outside the
subset and rejected.

```
---
id: LES-001
type: pitfall
scope: [core, tooling]
confidence: high
created: 2026-07-21
last_verified: 2026-07-21
source: journal/2026-07.md
supersedes: [LES-000]   # optional
---
Do not use CommonJS require() in this ESM package; it fails at runtime.
```

| Field | Required | Rule |
|---|---|---|
| `id` | yes | `^[A-Z]{2,}-\d{2,}$`, unique across the store (e.g. `LES-001`, `DEC-0002`) |
| `type` | yes | `pitfall`, `convention`, or `decision` |
| `scope` | yes | non-empty list of area tags |
| `confidence` | yes | `low`, `medium`, or `high` |
| `created` | yes | `YYYY-MM-DD` (valid calendar date) |
| `last_verified` | yes | `YYYY-MM-DD`; bump it whenever you re-confirm the record |
| `source` | yes | store-relative path the record was distilled from (usually a journal month). Whole files only - no `#L` line anchors, they break on the next append. A source that moved to `journal/archive/` still resolves. |
| `supersedes` | no | an id or list of ids this record replaces; set it when you merge duplicates |

`scope` and `supersedes` may use inline flow sequences (`[a, b]`) or block lists.

## `config.yml`

```yaml
version: 1
journal:
  committed: true
staleness_days: 120        # last_verified older than this -> doctor flags it
budgets:                   # per-file token ceilings
  state/status.md: 400
  context/architecture.md: 1500
  context/stack.md: 800
  context/domain.md: 1000
  lessons/pitfalls.md: 1000
  lessons/conventions.md: 1000
adapters:                  # auto | on | off
  claude: on
  cursor: off
  agents: on
sdd:
  openspec: auto
  speckit: auto
  bmad: auto
  agentos: auto
pack_budget: 2000          # optional; default token budget for `agnosgram pack`
```

Unknown keys are ignored on load, so future versions can add keys without breaking
older stores. Budgets are checked with a dependency-free token *estimate* (it errs
high so `doctor` warns early), not a real tokenizer - the zero-runtime-dependency
guarantee (DEC-0001) takes priority over exact counts. `pack_budget` (Milestone 3)
is one such additive key: it only sets the default `--budget` for `agnosgram pack`
and is not part of the frozen directory layout or record schema - a store without
it just uses the built-in default (2000).

## Freshness table (`MEMORY.md`)

The whole-file counterpart to per-record `last_verified`. Rows look like:

```
| File | Last verified | Budget |
|------|---------------|--------|
| state/status.md | 2026-07-21 | 400 tokens |
```

`doctor` flags any row whose date is older than `staleness_days`.

## The `meta/friction` extension

`.agnosgram/meta/` is an **additive, optional** extension to format version 1
(Milestone 5): tool-friction feedback (see [docs/feedback.md](feedback.md)),
kept strictly separate from the host-project memory described above. A store
without `meta/` remains fully valid, and adding it never changes what any
existing file means - the same compatibility guarantee as the rest of this
page.

```
.agnosgram/meta/friction.md   # many friction records; created by `feedback` on first use
```

`init` never scaffolds `meta/`; `agnosgram feedback "<text>"` creates it the
first time it is run. A friction record uses the exact same frontmatter
fields as a lesson or decision, with its own `type`:

```
---
id: FRI-003
type: friction
scope: [doctor, docs]
confidence: medium
created: 2026-07-30
last_verified: 2026-07-30
source: meta/friction.md
---
doctor's source.missing warning didn't mention archive/ is also checked.
```

| Field | Rule |
|---|---|
| `id` | `^FRI-\d{2,}$` in practice (the general `^[A-Z]{2,}-\d{2,}$` pattern), unique across the whole store |
| `type` | `friction` (the only value today) |
| `source` | conventionally `meta/friction.md` itself - friction is captured directly, not distilled from a journal month |

Everything else (`scope`, `confidence`, `created`, `last_verified`,
`supersedes`) follows the same rules as lessons/decisions. `doctor` validates
`meta/friction.md` the same way it validates lessons/decisions - schema,
duplicate ids, staleness, budgets - just against the friction type enum
instead of `pitfall | convention | decision`.

**`meta/` is never surfaced by retrieval.** `pack`, `show`, and `advise`'s
digest all read from the same shared record index, which explicitly excludes
`meta/` - it answers "what does this project need to remember," not "what is
annoying about the tool." `meta/friction.md` feeds `agnosgram reflect`
instead (see [docs/reflect.md](reflect.md)).

## `advise` reports are not store format

`agnosgram advise --validate` reads and checks a JSON report (`agnosgram_advise`,
see [docs/advise.md](advise.md)) that an agent writes to `<plan>.advise.json` (or
`--out <path>`). That file is a **CLI output contract** - versioned on its own
(`agnosgram_advise: 1`), shared with Gate's `plan.advise` check - not part of the
`.agnosgram/` store described on this page. It is never written under
`.agnosgram/`, has no frontmatter, and this document's format-version-1 guarantee
does not cover it; the store format stays untouched by `advise`.

## Compatibility guarantee

This is **format version 1**. Later releases may add optional fields or files; they
will never remove, rename, or repurpose anything above, and never make an old store
fail to load. A breaking change would require `version: 2` and a documented
migration - which the design is built to avoid. `doctor` refuses a store whose
`config.yml` declares a version it does not understand rather than validating it
as if it were v1. See `.agnosgram/decisions/0002-format-freeze.md`.
