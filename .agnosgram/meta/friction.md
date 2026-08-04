# Friction - tool-usage friction, not host-project memory

_Captured with `agnosgram feedback`. Never read by `pack`/`show`/`advise` -
this namespace is about Agnosgram itself, and feeds `agnosgram reflect`. See
docs/feedback.md._

---
id: FRI-001
type: friction
scope: [cli]
confidence: medium
created: 2026-07-30
last_verified: 2026-08-02
source: meta/friction.md
---
RESOLVED (0.11.0): every command now routes its argv through a shared
`parseCliArgs` wrapper (`src/core/args.ts`). It rewrites a dash-leading option
value to the `--opt=value` form before handing off to `node:util`'s
`parseArgs` - unless the following token is itself a recognized flag, which
still means a genuinely missing value - so `pack --budget -1` and `reflect
--months -1` reach their existing "must be a positive integer" validation
instead of crashing, and `log --learned "--x"` stores the literal string. Any
remaining `parseArgs` failure (unknown option, missing value, wrong type) is
converted to a clean `UserError` instead of a raw stack trace. Every command
has at least one dash-leading-value conformance test
(`src/commands/*.conformance.test.ts`).

The rewrite stops dead at the first bare `--` end-of-options terminator -
everything after it is left untouched and never treated as a rewrite
candidate, and `--` itself is never swallowed as a preceding string option's
value - so `show -- --type -x` still keeps `--type` as an ordinary
(unmatched) positional exactly as on 0.10.0, and `--type -- pitfall` still
raises `parseArgs`'s own genuine ambiguity, just as a clean `UserError`
instead of a raw crash. Pinned by unit tests in `src/core/args.test.ts` and
conformance tests in `src/commands/show.conformance.test.ts`.

Commands with numeric options crash with a raw parseArgs stack trace when the value starts with a dash, e.g. `agnosgram pack --budget -1`. parseArgs interprets the leading dash as another option rather than a negative number. Affects every command with a numeric flag (pack --budget, reflect --months, advise timeouts if any are added later), not just the ones added in Milestone 5. Pre-existing on main; noted here for the first reflect run rather than fixed in this branch.

---
id: FRI-002
type: friction
scope: [tooling]
confidence: medium
created: 2026-08-01
last_verified: 2026-08-01
source: meta/friction.md
---
Install story failed twice on a real machine (nix-managed Node): npm install -g dies on the read-only /nix/store global prefix with npm's misleading run-as-root advice, and install.sh's primary path 404s because no GitHub Release assets exist for the tagged versions despite the release workflow being expected to build them on tag push. A newcomer has no working install path; the workaround (npm ci in a tag-pinned worktree + a shim in ~/.local/bin) is undiscoverable. Wants: publish release assets for existing tags, and docs for the read-only-prefix case (NPM_CONFIG_PREFIX or pointing at install.sh binaries).

---
id: FRI-003
type: friction
scope: [adapters]
confidence: medium
created: 2026-08-01
last_verified: 2026-08-02
source: meta/friction.md
---
RESOLVED (0.11.0): `agnosgram adapt` now resolves each target's real path
(following symlinks even when the link target doesn't exist yet) and groups
requested adapters by it. When two or more targets - in either symlink
direction - collapse onto the same real file, the managed block is written
once and reported as one honest line, e.g. `CLAUDE.md -> AGENTS.md (symlink),
managed block written once`, both in the human output and in `--json`
(`symlinkAliases` on the result). Golden test with a symlinked fixture plus a
run-twice-is-identical check (CON-002) in
`src/commands/adapt.conformance.test.ts`.

On a repo where CLAUDE.md is a symlink to AGENTS.md, init/adapt reported writing both adapters as if they were independent files - the managed block lands once on disk but the report reads as a dual write, which confused a first-time adopter during a real onboarding (constructflow-api soak, 2026-07-30). Wants: symlink detection with a single honest report line.
