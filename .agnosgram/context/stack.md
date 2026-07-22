# Stack

_Languages, tooling, and the exact commands to build / test / lint. Pin versions
where they matter._

## Commands
- **Build:** `npm run build` (`tsc -p tsconfig.json` → `dist/`)
- **Test:** `npm test` (`tsc -p tsconfig.test.json` then `node --test "dist/**/*.test.js"`)
- **Bench:** `npm run bench` (Tier-1 token counts; `node bench/bench.mjs --check` gates in CI)

## Versions & constraints
- TypeScript, ESM (`"type": "module"`), Node ≥ 20.
- **Zero runtime dependencies** - non-negotiable (see decisions/0001). Dev-only deps: `typescript`, `@types/node`, `gpt-tokenizer` (bench).
- CLI arg parsing: `node:util` `parseArgs`. Tests: `node:test` + `node:assert/strict`. Both built-in.
- Tests are `src/**/*.test.ts`, excluded from the published build (`tsconfig.json` excludes them; `tsconfig.test.json` includes them).
