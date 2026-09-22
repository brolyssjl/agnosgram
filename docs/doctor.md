# `agnosgram doctor`

Lints the memory store and reports what has rotted, drifted, or become unsafe. It
never calls an LLM and never edits your files - it only reports. `doctor` is the
executable specification of the frozen format: a clean run means the store
conforms.

```bash
agnosgram doctor            # human-readable report
agnosgram doctor --json     # machine-readable report (for CI / Gate)
agnosgram doctor --strict   # exit non-zero on warnings too, not just errors
```

## Exit codes

- `0` - no errors (warnings may still be printed).
- `1` - at least one **error**, or any warning when `--strict` is set.

Run it in CI to keep memory healthy the same way you keep code healthy.

## Anti-rot

Every curated entry carries `confidence` + `last_verified`; `config.yml` sets a
`staleness_days` window and per-file token budgets. `doctor` turns that into
an executable check you can run in CI - it is the specification of the frozen
format. Full field-by-field contract: [docs/schema-reference.md](schema-reference.md).

## What it checks

**Errors** (block a clean run):

| Code | Meaning |
|---|---|
| `schema.*` | a record's frontmatter violates the schema (missing/invalid field) |
| `id.duplicate` | the same id appears on more than one record |
| `secret.*` | a probable committed secret (AWS key, PEM block, token, ...) - remove and rotate |
| `config.version.unsupported` | `config.yml` declares a format version this release does not understand |

**Warnings** (surfaced, non-blocking unless `--strict`):

| Code | Meaning |
|---|---|
| `record.stale` | a record's `last_verified` is older than `staleness_days` |
| `file.stale` | a freshness-table row is older than `staleness_days` |
| `budget.over` | a file exceeds its configured token budget |
| `link.broken` | a Markdown link points to a missing local file |
| `link.outside-project` | a Markdown link target is absolute or normalizes outside the project root - never probed for existence, so it cannot be used to test whether an arbitrary path exists |
| `supersedes.orphan` | `supersedes:` references an id no record defines |
| `record.near-duplicate` | two same-type records overlap heavily - merge them via `supersedes:` |
| `source.missing` | a record's `source:` path does not exist under `.agnosgram/` (archived journal months in `journal/archive/` still resolve) |
| `source.anchor` | a record's `source:` carries a `#L` line anchor, which breaks on the next append - reference the whole file |
| `injection.*` | stored memory contains an imperative that could hijack an agent |
| `status.stale` | the newest journal entry is dated after `state/status.md`'s recorded freshness - the journal moved on but status.md was never refreshed |
| `freshness.mismatch` | `MEMORY.md`'s freshness-table row for `state/status.md` disagrees with `status.md`'s own `Last updated:` line - update the table row, since that is what `status.stale` (and `pack`) actually read |
| `distill.lag` | the journal has real entries but `lessons/pitfalls.md` and `lessons/conventions.md` were never distilled, or the newest journal entry runs more than ~30 days ahead of the newest distilled lesson |
| `git.untracked` | a file under `.agnosgram/` exists but git does not track it - commit it so other worktrees and `reflect` runs elsewhere can see it |

## The safety lints

Because agents read the store at the start of every session, it is its own
supply-chain surface:

- **Secret scan** catches keys and credentials committed by mistake. Treated as an
  error: a leak in memory is a real leak.
- **Prompt-injection guard** flags imperative phrasings ("ignore previous
  instructions", role overrides, exfiltration or destructive-shell commands).
  Memory should record facts and lessons, never issue commands to the agent.

Both lean sensitive - a false positive is cheaper than a miss, and the human
reviews each hit in the PR.

## Recall freshness

`pack`'s whole promise is that useful memory gets recalled at the start of a
session - and that promise silently breaks when the surrounding discipline
lapses: `status.md` stops being updated, or the journal fills up with real
content that never gets curated into `lessons/`. Two warnings catch this
before it costs a session:

- **`status.stale`** compares the newest journal entry's date against
  `state/status.md`'s own recorded freshness (the `MEMORY.md` freshness-table
  row for it, falling back to the `Last updated:` line inside `status.md`
  itself when the table has no row). Both are content dates, read from the
  files themselves - never file mtimes, which git does not preserve across a
  clone or checkout.
- **`distill.lag`** fires once the journal has real entries (not just the
  scaffold's commented-out example) but `lessons/pitfalls.md` and
  `lessons/conventions.md` have zero records between them, or the newest
  journal entry is more than ~30 days newer than the newest distilled lesson.

Both suggest running `agnosgram distill` (and reviewing `status.md` by hand)
as the fix. `pack` surfaces the same `status.stale` signal too - see
[pack](pack.md#recall-freshness-note).

A related but distinct failure mode: an agent refreshes `status.md`'s own
`_Last updated:_` line but forgets the `MEMORY.md` freshness-table row is the
value `status.stale` (and `pack`) actually trust. **`freshness.mismatch`**
catches exactly that - it compares the two sources directly and warns when
they disagree, even before the journal itself has run ahead of either one.
`agnosgram distill`'s emitted prompt now calls out the `MEMORY.md` table as a
required edit whenever `status.md` or a lessons file is touched, precisely to
avoid producing this state.

## Git tracking

`.agnosgram/` is plain Markdown meant to be reviewed and shared through git
like any other source file - but a file can be written to disk (by hand, or
by an agent) and never `git add`ed, and nothing about the store itself would
look wrong. This bit twice on 2026-09-02: both distill agents running in the
host repos produced a `meta/friction.md` that sat untracked, which made it
invisible to any other worktree of the same repo and to a `reflect` run in a
fresh checkout elsewhere.

**`git.untracked`** shells out to git, scoped to `.agnosgram/`, to catch
this: any file under the store that git considers untracked and not
gitignored gets a warning naming that file, with the fix being to commit it
(or add it to `.gitignore` if it is genuinely meant to stay local to that
checkout). It skips cleanly - no findings, no error - when the project is
not a git repository at all, or `git` is unavailable; this is a hygiene
nudge on top of git, not a replacement for it. A file your `.gitignore`
already excludes is never flagged, since that is the store's own decision to
keep it local.

## Typical workflow

```bash
agnosgram doctor            # see what needs attention
# ... fix stale records (bump last_verified), trim over-budget files, or run distill
agnosgram doctor --strict   # confirm clean before committing
```
