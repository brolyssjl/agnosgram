# `agnosgram bootstrap`

Fast onboarding for a repository that has no memory yet - especially a legacy
codebase. It emits a prompt that seeds `context/architecture.md` and
`context/domain.md` (and `stack.md` where determinable) from the code that already
exists, so future sessions start with real project knowledge instead of
re-discovering it every time.

Like every judgment step, `bootstrap` never calls an LLM: it emits a prompt for
your agent and lets the agent do the reading.

```bash
agnosgram bootstrap          # print the bootstrap prompt
agnosgram bootstrap --json   # same prompt, wrapped in JSON
```

## What the prompt contains

The command inspects the repo and hands the agent a running start:

- **Top-level layout** - the entries worth exploring first (ignoring `node_modules`,
  `dist`, `.git`, and friends).
- **Stack signals** - detected from marker files (`package.json`, `go.mod`,
  `Cargo.toml`, `pyproject.toml`, `pom.xml`, ...).
- **Detected SDD frameworks and agent config files.**

It then tasks the agent to fill each context file within its token budget, grounded
in what it actually reads - and explicitly *not* to touch `lessons/`, `decisions/`,
or the journal, which come from real sessions via `log` and `distill`, never from a
one-shot bootstrap.

## Typical workflow

```bash
agnosgram init                 # scaffold the store (if not already present)
agnosgram bootstrap | your-agent
# review the agent's edits to context/*.md in a PR
agnosgram doctor               # confirm budgets and links are clean
```

Bootstrap is a one-time jump-start. From there, memory grows the durable way:
capture with `log`, curate with `distill`, keep it honest with `doctor`.
