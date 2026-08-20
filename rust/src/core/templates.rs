//! Port of `src/core/templates.ts`: scaffold content for a fresh
//! `.agnosgram/` store. Every literal is copied byte-for-byte from the TS
//! template strings.

use super::dates::{journal_month, today_iso};

pub fn iso_date() -> String {
    today_iso()
}

pub fn journal_month_now() -> String {
    journal_month()
}

pub fn memory_md(date: &str) -> String {
    let month = &date[..7];
    format!(
        "# Project memory - read this first\n\
\n\
This is Agnosgram, an agent-agnostic memory store. It is plain Markdown, lives in\n\
the repo, and is reviewed in PRs like any other code.\n\
\n\
## Protocol for agents\n\
1. Read `state/status.md` (always) - current focus and in-flight work.\n\
2. Read `lessons/pitfalls.md` and `lessons/conventions.md` (always).\n\
3. Read `context/*` only for areas you will touch.\n\
4. Read `decisions/` only when about to change something architectural -\n\
   check for an existing decision before proposing a change to settled matters.\n\
5. Before ending a session, append a journal entry with `agnosgram log`\n\
   (or by hand into `journal/{month}.md`).\n\
\n\
Specs say what the system should be; Agnosgram says what we learned making it.\n\
A fact belongs in exactly one place; the other side links to it.\n\
\n\
## Freshness\n\
| File | Last verified | Budget |\n\
|------|---------------|--------|\n\
| state/status.md | {date} | 400 tokens |\n\
| context/architecture.md | {date} | 1500 tokens |\n\
| context/stack.md | {date} | 800 tokens |\n\
| context/domain.md | {date} | 1000 tokens |\n\
| lessons/pitfalls.md | {date} | 1000 tokens |\n\
| lessons/conventions.md | {date} | 1000 tokens |\n"
    )
}

pub fn status_md(date: &str) -> String {
    format!(
        "# Status\n\
\n\
_Small and volatile. Overwrite freely; history lives in the journal._\n\
\n\
- **Focus:** _what is being worked on right now_\n\
- **In flight:** _branches / PRs / partially done work_\n\
- **Next:** _the next concrete step_\n\
- **Blocked on:** _nothing_\n\
\n\
_Last updated: {date}_\n"
    )
}

pub fn architecture_md() -> String {
    "# Architecture\n\
\n\
_System shape, module map, key invariants. Keep it to what an agent must know\n\
before touching the code - not an exhaustive tour._\n\
\n\
## Module map\n\
- _module \u{2192} responsibility_\n\
\n\
## Invariants\n\
- _things that must always hold true_\n"
        .to_string()
}

pub fn stack_md() -> String {
    "# Stack\n\
\n\
_Languages, tooling, and the exact commands to build / test / lint. Pin versions\n\
where they matter._\n\
\n\
## Commands\n\
- **Build:** _..._\n\
- **Test:** _..._\n\
- **Lint:** _..._\n\
\n\
## Versions\n\
- _runtime / key deps_\n"
        .to_string()
}

pub fn domain_md() -> String {
    "# Domain\n\
\n\
_Business/domain glossary and rules an agent won't infer from the code._\n\
\n\
- **_Term_:** _definition_\n"
        .to_string()
}

pub fn pitfalls_md() -> String {
    let date = iso_date();
    let month = journal_month_now();
    format!(
        "# Pitfalls - \"do not do X\"\n\
\n\
_Distilled failures. Each entry carries frontmatter (see below) so `doctor` can\n\
track staleness. Add via `distill`; edit by hand when you learn something now._\n\
\n\
<!-- Example entry - replace with real lessons:\n\
\n\
---\n\
id: LES-001\n\
type: pitfall\n\
scope: [example]\n\
confidence: high\n\
created: {date}\n\
last_verified: {date}\n\
source: journal/{month}.md\n\
---\n\
Never do X in situation Y - it causes Z. (Cost us N hours on DATE.)\n\
\n\
-->\n"
    )
}

pub fn conventions_md() -> String {
    let date = iso_date();
    let month = journal_month_now();
    format!(
        "# Conventions - \"always do Y\"\n\
\n\
_Patterns that worked, worth repeating. Same frontmatter schema as pitfalls._\n\
\n\
<!-- Example entry - replace with real conventions:\n\
\n\
---\n\
id: CON-001\n\
type: convention\n\
scope: [example]\n\
confidence: high\n\
created: {date}\n\
last_verified: {date}\n\
source: journal/{month}.md\n\
---\n\
Always do Y when doing X - it keeps Z consistent.\n\
\n\
-->\n"
    )
}

pub fn decisions_readme() -> String {
    let date = iso_date();
    let month = journal_month_now();
    format!(
        "# Decisions\n\
\n\
Lightweight ADRs, one per file, numbered: `0001-short-slug.md`.\n\
\n\
Each decision file carries frontmatter:\n\
\n\
```yaml\n\
---\n\
id: DEC-0001\n\
type: decision\n\
scope: [area]\n\
confidence: high\n\
created: {date}\n\
last_verified: {date}\n\
source: journal/{month}.md\n\
supersedes: DEC-0000   # optional\n\
---\n\
```\n\
\n\
Body: **Context** (what forced the choice), **Decision**, **Consequences**.\n\
Record a decision before changing something previously settled.\n"
    )
}

pub fn journal_md(month: &str) -> String {
    format!(
        "# Journal - {month}\n\
\n\
Append-only. One file per month. Four fixed slots per entry so distillation is\n\
mechanical: `Learned` lines are lesson candidates, `Decided` lines are ADR\n\
candidates.\n\
\n\
<!-- Entry format (newest at the bottom; `agnosgram log` appends here):\n\
\n\
## {month}-DD HH:MM \u{b7} <agent> \u{b7} <branch>\n\
- **Did:** ...\n\
- **Learned:** ...\n\
- **Decided:** ...\n\
- **Avoid:** ...\n\
- **Next:** ...\n\
\n\
-->\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_md_embeds_the_date_and_derived_month() {
        let out = memory_md("2026-08-20");
        assert!(out.contains("journal/2026-08.md"));
        assert!(out.contains("| state/status.md | 2026-08-20 | 400 tokens |"));
    }

    #[test]
    fn status_md_embeds_the_date() {
        assert!(status_md("2026-08-20").contains("_Last updated: 2026-08-20_"));
    }

    #[test]
    fn journal_md_embeds_the_month_in_heading_and_example() {
        let out = journal_md("2026-08");
        assert!(out.starts_with("# Journal - 2026-08\n"));
        assert!(out.contains("## 2026-08-DD HH:MM"));
    }

    #[test]
    fn pitfalls_and_conventions_templates_carry_a_valid_example_frontmatter_shape() {
        assert!(pitfalls_md().contains("id: LES-001"));
        assert!(conventions_md().contains("id: CON-001"));
    }
}
