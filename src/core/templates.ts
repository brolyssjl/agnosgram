/**
 * Scaffold content for a fresh `.agnosgram/` store. Every file is plain Markdown
 * (with YAML frontmatter where a record schema applies). These are starting
 * points meant to be edited by humans and agents alike.
 */

export function isoDate(d: Date = new Date()): string {
  return d.toISOString().slice(0, 10);
}

export function journalMonth(d: Date = new Date()): string {
  return d.toISOString().slice(0, 7); // YYYY-MM
}

export function memoryMd(date: string): string {
  return `# Project memory - read this first

This is Agnosgram, an agent-agnostic memory store. It is plain Markdown, lives in
the repo, and is reviewed in PRs like any other code.

## Protocol for agents
1. Read \`state/status.md\` (always) - current focus and in-flight work.
2. Read \`lessons/pitfalls.md\` and \`lessons/conventions.md\` (always).
3. Read \`context/*\` only for areas you will touch.
4. Read \`decisions/\` only when about to change something architectural -
   check for an existing decision before proposing a change to settled matters.
5. Before ending a session, append a journal entry with \`agnosgram log\`
   (or by hand into \`journal/${date.slice(0, 7)}.md\`).

Specs say what the system should be; Agnosgram says what we learned making it.
A fact belongs in exactly one place; the other side links to it.

## Freshness
| File | Last verified | Budget |
|------|---------------|--------|
| state/status.md | ${date} | 400 tokens |
| context/architecture.md | ${date} | 1500 tokens |
| context/stack.md | ${date} | 800 tokens |
| context/domain.md | ${date} | 1000 tokens |
| lessons/pitfalls.md | ${date} | 1000 tokens |
| lessons/conventions.md | ${date} | 1000 tokens |
`;
}

export function statusMd(date: string): string {
  return `# Status

_Small and volatile. Overwrite freely; history lives in the journal._

- **Focus:** _what is being worked on right now_
- **In flight:** _branches / PRs / partially done work_
- **Next:** _the next concrete step_
- **Blocked on:** _nothing_

_Last updated: ${date}_
`;
}

export function architectureMd(): string {
  return `# Architecture

_System shape, module map, key invariants. Keep it to what an agent must know
before touching the code - not an exhaustive tour._

## Module map
- _module → responsibility_

## Invariants
- _things that must always hold true_
`;
}

export function stackMd(): string {
  return `# Stack

_Languages, tooling, and the exact commands to build / test / lint. Pin versions
where they matter._

## Commands
- **Build:** _..._
- **Test:** _..._
- **Lint:** _..._

## Versions
- _runtime / key deps_
`;
}

export function domainMd(): string {
  return `# Domain

_Business/domain glossary and rules an agent won't infer from the code._

- **_Term_:** _definition_
`;
}

export function pitfallsMd(): string {
  return `# Pitfalls - "do not do X"

_Distilled failures. Each entry carries frontmatter (see below) so \`doctor\` can
track staleness. Add via \`distill\`; edit by hand when you learn something now._

<!-- Example entry - replace with real lessons:

---
id: LES-001
type: pitfall
scope: [example]
confidence: high
created: ${isoDate()}
last_verified: ${isoDate()}
source: journal/${journalMonth()}.md
---
Never do X in situation Y - it causes Z. (Cost us N hours on DATE.)

-->
`;
}

export function conventionsMd(): string {
  return `# Conventions - "always do Y"

_Patterns that worked, worth repeating. Same frontmatter schema as pitfalls._

<!-- Example entry - replace with real conventions:

---
id: CON-001
type: convention
scope: [example]
confidence: high
created: ${isoDate()}
last_verified: ${isoDate()}
source: journal/${journalMonth()}.md
---
Always do Y when doing X - it keeps Z consistent.

-->
`;
}

export function decisionsReadme(): string {
  return `# Decisions

Lightweight ADRs, one per file, numbered: \`0001-short-slug.md\`.

Each decision file carries frontmatter:

\`\`\`yaml
---
id: DEC-0001
type: decision
scope: [area]
confidence: high
created: ${isoDate()}
last_verified: ${isoDate()}
source: journal/${journalMonth()}.md
supersedes: DEC-0000   # optional
---
\`\`\`

Body: **Context** (what forced the choice), **Decision**, **Consequences**.
Record a decision before changing something previously settled.
`;
}

export function journalMd(month: string): string {
  return `# Journal - ${month}

Append-only. One file per month. Four fixed slots per entry so distillation is
mechanical: \`Learned\` lines are lesson candidates, \`Decided\` lines are ADR
candidates.

<!-- Entry format (newest at the bottom; \`agnosgram log\` appends here):

## ${month}-DD HH:MM · <agent> · <branch>
- **Did:** ...
- **Learned:** ...
- **Decided:** ...
- **Avoid:** ...
- **Next:** ...

-->
`;
}
