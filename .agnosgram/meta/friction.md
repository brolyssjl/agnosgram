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
last_verified: 2026-07-30
source: meta/friction.md
---
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
last_verified: 2026-08-01
source: meta/friction.md
---
On a repo where CLAUDE.md is a symlink to AGENTS.md, init/adapt reported writing both adapters as if they were independent files - the managed block lands once on disk but the report reads as a dual write, which confused a first-time adopter during a real onboarding (constructflow-api soak, 2026-07-30). Wants: symlink detection with a single honest report line.
