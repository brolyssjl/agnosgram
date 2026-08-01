---
id: DEC-0003
type: decision
scope: [core, tooling]
confidence: high
created: 2026-07-30
last_verified: 2026-07-30
source: journal/2026-07.md
---

# Human-in-the-loop guarantee for the reflexive loop (Milestone 5)

## Context
Milestone 5 lets Agnosgram use Agnosgram to evolve Agnosgram: `feedback`
captures tool friction, `reflect` turns it into improvement proposals and
candidate roadmap milestones. A tool that proposes changes to its own
roadmap is exactly the kind of feature that could quietly start disposing
instead of proposing - editing `ROADMAP.md` itself, writing into the memory
it is supposed to only observe, or reaching the network without asking. That
would break the project's founding constraints (`ROADMAP.md` Milestone 5:
"the CLI never calls an LLM, sends no telemetry, and the human decides") and
the wider promise that Agnosgram is inert until a human or agent explicitly
runs it.

## Decision
The reflexive loop **proposes; it never disposes.** Specifically, and
permanently:

1. **`reflect` never writes to the repo.** It is a pure read: it loads
   `meta/friction.md` and recent `journal/*.md` files and emits a prompt.
   It never edits `ROADMAP.md`, and never writes to `lessons/`, `decisions/`,
   `context/`, or `state/status.md` - not even as a side effect. Its prompt
   states this rule to whatever agent responds to it, so the constraint holds
   even when a human hands the prompt to an LLM: proposals go to stdout or a
   new, human-named file, never applied automatically.
2. **`feedback` writes only under `.agnosgram/meta/`.** It never touches
   `lessons/`, `decisions/`, `context/`, `state/`, or the journal - those are
   host-project memory, and friction about the tool is a different kind of
   fact entirely.
3. **`meta/` is a compatible, additive extension of the frozen format**
   (`.agnosgram/decisions/0002-format-freeze.md`). A store without `meta/`
   remains fully valid; nothing this milestone adds changes the meaning of
   anything that existed before it. `doctor` validates `meta/friction.md`
   the same way it validates lessons/decisions (schema, ids, staleness,
   budgets), against its own type enum, never the frozen lessons/decisions
   one.
4. **No network calls, ever, from the CLI itself.** `feedback --share`
   prints a ready-to-run `gh issue create ...` command; it does not invoke
   `gh` or any HTTP client. Sharing friction upstream (Discussions, a
   `feedback/` PR, or an agent-filed issue) always requires a human's
   explicit action, matching the project's zero-telemetry guarantee.

## Consequences
- Any future addition to the reflexive loop (a new proposal channel, a
  richer `reflect` digest, an automated triage step) must preserve points
  1-4 above. A change that has `reflect` or `feedback` write outside their
  documented scope, or that has the CLI itself reach the network, would
  need a new decision record explicitly superseding this one - not a quiet
  patch.
- The regression test asserting `reflect` leaves the working tree untouched
  and `feedback` writes only under `.agnosgram/meta/` (see
  `src/commands/reflect.test.ts` and `src/commands/feedback.test.ts`) is the
  executable specification of this guarantee, the same role `doctor` plays
  for the frozen format itself.
