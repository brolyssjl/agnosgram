# `agnosgram show`

Print stored records matching a topic - for agents with weak file navigation,
or a human who wants the relevant slice without opening `.agnosgram/` by hand.
Read-only; never edits the store.

```bash
agnosgram show LES-001                      # exact record id
agnosgram show backend                      # every record scoped "backend"
agnosgram show pitfall                      # every record of type pitfall
agnosgram show backend --type convention    # scope, but only conventions
agnosgram show backend --format toon        # structured, TOON-encoded
```

## Match order

`show <topic>` tries, in order, and stops at the first non-empty result:

1. **Exact record id** (`LES-001`) - case-sensitive, since ids are always
   uppercase by schema.
2. **Case-insensitive exact scope tag** (`backend`, `Backend`, `BACKEND` all
   match a record scoped `backend`).
3. **Type name** (`pitfall`, `convention`, or `decision`) - every record of
   that type.

There is no fuzzy matching (deferred) - a typo gets you no match, not a guess.

`--type <pitfall|convention|decision>` filters the candidate pool *before*
matching, so `show backend --type decision` only ever returns decisions, even
though step 2 would otherwise also pull in matching pitfalls and conventions.

## No match

Exits `1` and prints a hint on stderr listing every scope tag currently in the
store (`allScopes()`) plus the three type names, so the next guess is
informed:

```
No records match "fronted".
Known scopes: auth, backend, core, tooling. Types: pitfall, convention, decision.
```

## Output shapes

- **Human (default):** each matched record rendered as it looks on disk
  (frontmatter fence + body), grouped under a heading per source file.
- **`--format json` / `--format toon`:** a uniform array of flat objects -
  `scope`/`supersedes` are joined to comma strings so the array stays
  TOON-tabular:
  ```json
  [{ "id": "LES-001", "type": "pitfall", "scope": "core,tooling", "...": "..." }]
  ```
