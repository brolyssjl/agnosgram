# Status

_Small and volatile. Overwrite freely; history lives in the journal._

- **Focus:** Milestone 3 (Retrieval) merged to `main` via PR #3. Milestone 4 (Reach &
  ergonomics) complete on branch `milestone-4-reach`: Windsurf/Cline/Roo adapters,
  Codex/OpenCode routed to the `AGENTS.md` adapter, path-aware SDD coexistence
  hints, opt-in Claude Code hooks + skill (`adapt --claude-hooks`), `install.sh` +
  single-file binary (bun / Node SEA), and docs. Version bumped to `0.9.0`.
- **In flight:** branch `milestone-4-reach` pushed to origin. First coordinator
  review found 10 correctness bugs + 6 cleanup items + an em-dash sweep, all now
  fixed in small commits with regression tests (gates green from a clean `npm ci`).
  PR still not opened - awaiting coordinator re-review.
- **Next:** coordinator re-reviews; open the Milestone 4 PR once approved; then the
  `1.0.0` real-world soak (dogfood `0.9.0` on the legacy project, then a docs site
  + case study).
- **Blocked on:** nothing. (npm name reservation + first publish still deferred by owner.)

_Last updated: 2026-07-29_
