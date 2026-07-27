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
