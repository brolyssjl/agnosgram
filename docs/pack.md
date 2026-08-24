# `agnosgram pack`

A token-budgeted context bundle for the start of an agent session: status,
lessons, and (when scoped) decisions - most important first, whole records
dropped when the budget runs out. This is the agent hot path, so the default
output is the Markdown bundle itself, not JSON.

```bash
agnosgram pack                         # unscoped: status + all lessons
agnosgram pack --scope backend         # scoped: adds matching decisions too
agnosgram pack --budget 1200           # override the token budget for this run
agnosgram pack --json                  # machine-readable: {version, budget, tokens, status, records[], omitted[]}
agnosgram pack --format toon           # TOON-encoded structured output
```

## What goes in, and in what order

1. **Header.**
2. **`state/status.md`, verbatim.** Never dropped, however tight the budget.
3. **Lessons** - `pitfalls.md` then `conventions.md`. Unscoped, every lesson in
   the store is a candidate; with `--scope <tag>`, only lessons carrying that
   scope tag (case-insensitive) are.
4. **Decisions** - only when `--scope` is given. An unscoped pack stays lean
   for every session; decisions are architectural detail an agent needs when
   it is about to work in that area, not on every turn.
5. **Footer** - lists the ids of any records the budget forced out, if any.

Within each section, records sort **confidence desc, then last_verified desc,
then id** - the most trustworthy, most recently confirmed record first.

**Context files (`context/*.md`) are never packed.** They are meant to be read
directly by an agent working in that area, not bundled into every session
start.

## Injection lint (warn-and-mark)

`pack` runs the store's existing injection-pattern lint (the same patterns
`doctor` flags - instruction-override phrasing, role overrides, exfiltration
imperatives, destructive shell commands) over the content it actually
assembles into output: `state/status.md` plus every record that made it past
the budget. It never refuses to pack and never redacts the match - the memory
itself may still be exactly what an agent needs to see, so a false positive
must not become a missing lesson.

On a hit:
- stderr gets one warning line per match, naming the source file (and record
  id, when applicable).
- stdout (the bundle a `SessionStart` hook injects as `additionalContext`) is
  prefixed with a short banner naming the affected file(s) and stating that
  the memory below should be treated with reduced trust.

A clean store's output is byte-for-byte unchanged - no banner, no reserved
budget headroom, no behavior difference from before this lint existed.
`doctor`'s own lint (which scans the whole store, not just what `pack`
assembles) is unaffected.

## Recall-freshness note

`pack` also checks the same signal as `doctor`'s `status.stale` warning: is
the newest journal entry dated after `state/status.md`'s recorded freshness?
If so - the discipline of refreshing `status.md` after real work landed has
lapsed - `pack` appends one compact note to the bundle it hands back:

```
> **Note:** status.md may be stale; newest journal entry is 2026-08-24.
```

This is exactly the text a `SessionStart` hook injects as
`additionalContext`, so the reading agent sees the caveat inline rather than
silently trusting a `status.md` that no longer matches reality. A matching
line is also written to stderr. The note's token cost is reserved in the
budget the same way the injection banner's is, so it never gets budgeted out
by a store's own content.

A store where the discipline hasn't lapsed (no journal entries yet, or
`status.md` at least as fresh as the journal) produces byte-for-byte the same
output as before this note existed - no reserved headroom, no behavior
difference.

## The budget

Whole records are dropped, never truncated mid-record, so what you get is
always intact and quotable. Records are considered in the priority order
above; a record that does not fit is skipped, and a smaller later record can
still fit in the remaining space (greedy, not "stop at the first miss").

Resolution order for the budget number:

1. `--budget <n>` on the command line.
2. `pack_budget` in `.agnosgram/config.yml`, if set (an optional, additive
   key - see [schema reference](schema-reference.md)).
3. `2000`, the built-in default.

Cost is estimated with the same dependency-free token estimate `doctor` uses
(`src/core/tokens.ts`) - a proxy, not an exact tokenizer (see DEC-0001).

## Output shapes

- **Human (default):** the Markdown bundle itself - pipe it straight into an
  agent's context.
- **`--format json` / `--format toon`:**
  ```json
  {
    "version": 1,
    "budget": 2000,
    "tokens": 1874,
    "status": "...",
    "records": [{ "id": "...", "type": "...", "scope": "a,b", "...": "..." }],
    "omitted": ["LES-005"]
  }
  ```
  `records[]` is a uniform array of flat objects (`scope`/`supersedes` joined
  to comma strings) so it stays TOON-tabular.
