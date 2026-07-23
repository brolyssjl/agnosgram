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

## What it checks

**Errors** (block a clean run):

| Code | Meaning |
|---|---|
| `schema.*` | a record's frontmatter violates the schema (missing/invalid field) |
| `id.duplicate` | the same id appears on more than one record |
| `secret.*` | a probable committed secret (AWS key, PEM block, token, ...) - remove and rotate |

**Warnings** (surfaced, non-blocking unless `--strict`):

| Code | Meaning |
|---|---|
| `record.stale` | a record's `last_verified` is older than `staleness_days` |
| `file.stale` | a freshness-table row is older than `staleness_days` |
| `budget.over` | a file exceeds its configured token budget |
| `link.broken` | a Markdown link points to a missing local file |
| `supersedes.orphan` | `supersedes:` references an id no record defines |
| `record.near-duplicate` | two same-type records overlap heavily - merge them via `supersedes:` |
| `injection.*` | stored memory contains an imperative that could hijack an agent |

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

## Typical workflow

```bash
agnosgram doctor            # see what needs attention
# ... fix stale records (bump last_verified), trim over-budget files, or run distill
agnosgram doctor --strict   # confirm clean before committing
```
