# `agnosgram distill`

Turns the raw, append-only journal into a small set of durable, non-overlapping
lessons and decisions - and shrinks what is already curated. Capture is cheap;
distillation is deliberate.

Following the project's core rule, `distill` never calls an LLM. It **emits a
precise prompt** for whatever agent you have, then **mechanically validates** the
result the agent writes back.

```bash
agnosgram distill                              # print the compaction prompt
agnosgram distill --json                       # same prompt, wrapped in JSON
agnosgram distill --validate lessons/pitfalls.md   # check a distilled file
agnosgram distill --archive 2026-07            # retire an absorbed journal month
```

## The three phases

### 1. Emit the prompt

`agnosgram distill` prints a task for the agent. The prompt lists the journal
months to read, the exact frontmatter schema, the ids already taken (so it
allocates fresh ones), and every file's token budget. Its core rules:

- **Merge, do not append.** When a new insight overlaps an existing record,
  rewrite the existing one and list what it replaces under `supersedes:`. Never
  leave two near-duplicate records side by side.
- Ground every record in a source; do not invent facts.
- Stay within the token budgets.
- **If you touch `state/status.md` or any `lessons/*.md` file, also update its
  row in `MEMORY.md`'s `## Freshness` table** (the `Last verified` column) to
  today's date. That table - not the file's own dated line - is what
  `doctor`'s `status.stale`/`file.stale` checks read; skipping it is the most
  common way a distill run "passes" but `doctor --strict` still fails.

Pipe it to your agent, e.g. `agnosgram distill | your-agent`, or paste it in.

### 2. Validate the result

After the agent writes records, check each file it touched:

```bash
agnosgram distill --validate lessons/pitfalls.md
```

This verifies frontmatter schema, unique ids (within the file and against the rest
of the store), and the file's token budget. It exits non-zero if anything is
wrong, so it fits in a script. Run `agnosgram doctor --strict` for the full sweep
(near-duplicates, staleness, links, safety).

### 3. Archive absorbed months

Once distilled records are accepted, retire the journal months they came from so
they stop counting against budgets and re-distillation:

```bash
agnosgram distill --archive 2026-07
```

This moves `journal/2026-07.md` to `journal/archive/2026-07.md`. Archived months
are frozen history: `doctor` ignores them, and records whose `source:` names the
original month keep resolving (`doctor` also checks `journal/archive/`), so
archiving never turns a healthy store red.

## Why the human stays in the loop

`distill` proposes; it never merges. The agent writes records, the mechanical
checks gate them, and you review the diff in a PR like any other change. The tool
guarantees the *shape* is correct; you guarantee the *content* is true.
