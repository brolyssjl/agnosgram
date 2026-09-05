# `agnosgram feedback`

Capture friction with **Agnosgram itself** - a confusing command, a missing
flag, a `doctor` message that didn't help - as its own entries, kept strictly
apart from your project's own memory (`lessons/`, `decisions/`, `context/`).

**Who this is for:** the friction loop (`feedback` + `reflect`) is the
maintainer's dogfooding channel, run in projects the maintainer owns. A
regular install never creates or reads `meta/` - if the tool frustrates you,
the supported channel is opening an issue or a PR (see "The opt-in community
feedback inbox" below). Teams that do want the loop locally can use it in any
store; gitignoring `meta/` to keep it out of shared history is a fully
supported choice (the `git.untracked` doctor check respects gitignore).
Friction bodies are free text that later gets embedded into `reflect`
prompts, so they get the same untrusted-content treatment as everything else
in the store - see README's "Trust posture".

```bash
agnosgram feedback "doctor's source.missing warning didn't mention archive/"
agnosgram feedback "..." --scope docs,doctor --confidence high
echo "longer writeup..." | agnosgram feedback --stdin
agnosgram feedback "..." --share    # also prints a gh issue create command
```

## Why a separate namespace

`.agnosgram/meta/` is about the tool, not the host project. It:

- Is created on first use by `feedback` - `init` never scaffolds it, since
  most projects only ever want host-project memory.
- Is never read by `pack`, `show`, or `advise` - those exist to give an agent
  *host-project* context, and tool friction is not that.
- Is validated by `doctor` the same way lessons/decisions are (schema,
  duplicate ids, staleness, budgets), just against its own `type: friction`
  entry, not the frozen lessons/decisions enum.
- Is a documented, backward-compatible, additive extension of the frozen
  format (`.agnosgram/decisions/0002-format-freeze.md`): a store without
  `meta/` is fully valid, and adding it never changes what anything else
  means. See `.agnosgram/decisions/0003-human-in-the-loop-reflexive-loop.md`.

## Entry shape

A friction entry uses the same frontmatter shape as every other record (see
[docs/schema-reference.md](schema-reference.md#the-metafriction-extension)),
with `type: friction` and an id prefix of `FRI-`:

```
---
id: FRI-003
type: friction
scope: [doctor, docs]
confidence: medium
created: 2026-07-30
last_verified: 2026-07-30
source: meta/friction.md
---
doctor's source.missing warning didn't mention archive/ is also checked.
```

`source` conventionally points at `meta/friction.md` itself - friction is
captured directly, not distilled from a journal month.

## The opt-in community feedback inbox

`agnosgram feedback` alone is purely local: it writes to
`.agnosgram/meta/friction.md` and nothing else, no network call involved.
Sending that friction outward is a separate, opt-in step - always requiring a
human to actually do it, since `agnosgram` never calls a network API and
never runs `gh` on its own. Three ways to do that:

1. **GitHub Discussions.** For open-ended conversation about the tool -
   proposals, questions, "does anyone else hit this." A human starts the
   thread; nothing here automates it.
2. **`feedback/` PRs.** If you want to share distilled friction upstream,
   open a PR adding a Markdown file under a `feedback/` directory summarizing
   what you found - reviewed like any other contribution.
3. **Agent-filed issues, with explicit consent only.** `agnosgram feedback
   --share` prints a ready-to-run `gh issue create ...` command to stdout. It
   is never executed by Agnosgram; a human (or an agent a human has
   explicitly asked to do this) runs it themselves. See
   `.github/ISSUE_TEMPLATE/feedback.yml` for the issue form this targets.

## Feeding `reflect`

Friction entries are the primary input to `agnosgram reflect`, which turns
them (plus recent journal months) into improvement proposals and candidate
roadmap milestones for a human to review. See
[docs/reflect.md](reflect.md).
