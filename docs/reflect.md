# `agnosgram reflect`

Turns tool friction (`.agnosgram/meta/friction.md`, see
[docs/feedback.md](feedback.md)) and recent journal months into improvement
proposals and candidate roadmap milestones - Agnosgram using Agnosgram to
evolve Agnosgram, the Milestone 5 reflexive loop.

Like `distill`/`advise`/`bootstrap`, `reflect` never calls an LLM. It **emits
a precise prompt** for whatever agent you have and stops there - there is no
`--validate` step, because its output (proposals) is for a human to read, not
a schema the CLI can mechanically check.

```bash
agnosgram reflect                # print the reflection prompt (last 3 months)
agnosgram reflect --months 6     # widen the journal window
agnosgram reflect --json         # same prompt, wrapped in a versioned envelope
```

## What the prompt contains

- A digest table of every friction entry currently captured (id, scope,
  confidence, created, excerpt).
- Pointers to the last `--months` (default 3) journal files, so the agent can
  look for `Avoid`/`Learned` lines about the tool itself.
- The exact reviewable form to present each proposal in: the friction it
  addresses (by `FRI-` id), the concrete change, its expected effect, and
  whether it rises to a candidate roadmap milestone.
- The rule below, verbatim.

## The rule: durable records are owner-edited

`reflect`'s prompt states this explicitly, and `reflect` itself performs no
writes to the repo at all - it only reads `meta/friction.md` and journal
files:

- Proposals go to stdout, or to a **new** file the human names, if they want
  a durable record.
- The agent must never write proposals directly into any roadmap or planning
  doc the host project keeps (in this repo, that's `ROADMAP.md` - but the
  prompt itself names no specific file, since a host project may not keep
  one at all), or write to `lessons/`, `decisions/`, `context/`, or
  `state/status.md` as a side effect of reflecting.
- The reflexive loop **proposes; it never disposes.** A human decides which
  proposals get acted on and edits their own planning docs themselves.

The prompt template is deliberately host-generic: it names only artifacts
every `.agnosgram/` store has (`meta/friction.md`, `journal/`). Earlier
versions cited this repo's own `ROADMAP.md` and a `CON-003` convention id as
if every host store had them too - a dangling reference in any project that
doesn't (see the friction filed during the 2026-08-24 soak).

See `.agnosgram/decisions/0003-human-in-the-loop-reflexive-loop.md` for the
full guarantee this command is built to uphold.

## The `--json` envelope

```json
{
  "agnosgram_reflect": 1,
  "friction_count": 3,
  "friction_ids": ["FRI-001", "FRI-002", "FRI-003"],
  "journal_months": ["2026-05", "2026-06", "2026-07"],
  "prompt": "..."
}
```

`friction_count`/`friction_ids`/`journal_months` let Gate or another tool see
at a glance how much `reflect` actually had to work with, without parsing the
prompt text itself.
