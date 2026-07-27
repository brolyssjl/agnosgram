# `agnosgram advise`

The landmine-catcher. Cross-checks a plan (or spec) against everything this
project's memory store has learned - `lessons/pitfalls.md`,
`lessons/conventions.md`, `decisions/*.md` - and flags contradictions before the
plan becomes code.

Like `distill`, `advise` never calls an LLM itself. It **emits a precise prompt**
for whatever agent you have, then **mechanically validates** the JSON report the
agent writes back.

```bash
agnosgram advise plan.md                     # step 1: print the review prompt
agnosgram advise plan.md --out report.json    # same, naming a different report path
agnosgram advise --validate plan.md.advise.json           # step 2: validate the report
agnosgram advise --validate report.json --strict --json   # strict + machine-readable
```

## The two steps

### 1. Emit the prompt

`agnosgram advise <plan-path>` prints a task for the agent: a pointer to the
plan, a digest table of every lesson and decision currently in the store, the
precedence rule (verbatim, see below), and the exact JSON schema to write back
to `<plan>.advise.json` (or `--out <path>`).

### 2. Validate the result

```bash
agnosgram advise --validate <report-file> [--strict] [--json]
```

This is mechanical only - it checks *shape and provenance*, never whether a
flagged contradiction is actually correct:

- **Schema** - `agnosgram_advise` is `1`; `plan`, `generated`, `checked_ids`,
  `contradictions`, `clear` all have the right shape; each contradiction has a
  valid `kind` (`empirical|normative`), `severity` (`blocker|caution`),
  `confidence` (`low|medium|high`), and a non-empty `explanation`.
- **Provenance** (errors) - every id in `checked_ids` and every
  `contradictions[].record_id` must name a record that actually exists;
  `confidence` and `last_verified` on a contradiction must exactly match the
  cited record's real frontmatter. A report cannot claim things about a record
  it gets wrong.
- **Excerpts and coverage** (warnings, non-blocking) - `plan_excerpt` should be
  a real substring of the plan file, `record_excerpt` a real substring of the
  cited record's body; `checked_ids` should cover the whole digest, not a
  sample.

Exit codes: `0` when there are no errors, `1` when there is at least one error,
and also `1` under `--strict` when the report is valid but `clear` is `false`
(i.e. it found something worth a human's attention).

## The precedence rule

`advise`'s prompt states this exactly, and does not ask the agent to
reinterpret it:

- **Normative** contradiction (the plan proposes a different convention/policy
  than a stored decision or convention): **the spec (the plan) wins** by
  default; the agent records the exception only if a stored record explicitly
  permits deviating.
- **Empirical** contradiction (the plan assumes a fact a stored pitfall/lesson
  directly contradicts): **the contradiction wins** - it gets flagged; a human
  arbitrates.

## The `agnosgram_advise` report schema

This is a public contract, also consumed by Gate's `plan.advise` check, and is
versioned independently of the `.agnosgram/` store format:

```json
{
  "agnosgram_advise": 1,
  "plan": "<path>",
  "generated": "YYYY-MM-DD",
  "checked_ids": ["LES-001", "..."],
  "contradictions": [
    {
      "record_id": "LES-002",
      "kind": "empirical | normative",
      "severity": "blocker | caution",
      "plan_excerpt": "...",
      "record_excerpt": "...",
      "confidence": "high",
      "last_verified": "YYYY-MM-DD",
      "explanation": "one sentence"
    }
  ],
  "clear": false
}
```

A consumer that only understands `agnosgram_advise: 1` should treat a missing
or unparseable report as "no report" (advisory, not blocking) rather than fail
closed - Gate's `plan.advise` check follows this rule.

## Why the human stays in the loop

`advise` proposes; it never merges and never edits the plan. The agent writes
the report, the mechanical checks gate its shape and provenance, and a human
decides what to do about each flagged contradiction.
