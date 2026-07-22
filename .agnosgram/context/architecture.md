# Architecture

_System shape, module map, key invariants. Keep it to what an agent must know
before touching the code - not an exhaustive tour._

## Module map
- `src/cli.ts` → entry point; dispatches `init`/`adapt`/`log`, handles `--help`/`--version`, converts `UserError` to exit code 1.
- `src/commands/*` → one file per command (`init`, `adapt`, `log`). Each exports a `runX(argv)` and any reusable helpers.
- `src/adapters/index.ts` → adapter registry (`claude`, `cursor`, `agents`) + the single shared `buildPointerBody()` template and SDD hint lines.
- `src/core/`:
  - `yaml.ts` → minimal dependency-free YAML read/write (narrow subset; throws outside it).
  - `markers.ts` → `upsertManagedBlock` - idempotent injection between `agnosgram:start/end` markers.
  - `paths.ts` → project-root discovery (`.agnosgram/` → `.git/` → cwd).
  - `config.ts` → `AgnosgramConfig` type, load/save, defaults, normalization.
  - `detect.ts` → advisory SDD + agent detection (path presence only).
  - `templates.ts` → scaffold file contents.
  - `output.ts` → `UserError`, `info`/`warn`/`printJson`.

## Invariants
- **The CLI never calls an LLM and touches no network.** Judgment steps emit prompts (Milestone 2); they never call an API.
- **Zero runtime dependencies.** Anything reaching for a runtime dep needs a decision (see decisions/0001).
- **`adapt` is idempotent** and only ever rewrites the managed block; user content outside the markers is preserved byte-for-byte. Golden tests enforce this.
- **Detection is advisory, never load-bearing.** A wrong/missing detection can only change hint lines, never corrupt memory.
