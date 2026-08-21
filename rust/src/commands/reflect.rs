//! Port of `src/commands/reflect.ts`.
//!
//! `agnosgram reflect` - turns tool friction (`meta/friction.md`) and recent
//! journal months into improvement proposals and candidate roadmap
//! milestones. Same prompt-emitting pattern as `distill`/`advise`/`bootstrap`
//! (see those files): the CLI never calls an LLM, only assembles a prompt for
//! whatever agent is present.
//!
//! Unlike `distill`/`advise`, `reflect` has no `--validate` step - its output
//! is proposals for a human to read, not a mechanically-checkable report.
//! `reflect` itself performs no writes to the repo.

use crate::core::args::{parse_cli_args, ArgsConfig, OptionDef};
use crate::core::json::Value;
use crate::core::meta::load_friction_records;
use crate::core::output::{print_structured, UserError};
use crate::core::paths::{find_project_root, has_store};
use crate::core::records::StoreRecord;
use crate::core::serialize::{resolve_format, FormatFlags, ResolvedFormat};
use crate::core::store::journal_months;

/// Envelope version for `reflect --json` (independent of the store format version).
pub const REFLECT_SCHEMA_VERSION: i64 = 1;

/// Recent journal months included by default when `--months` is not given.
const DEFAULT_MONTHS: usize = 3;

fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn friction_digest(records: &[StoreRecord]) -> String {
    if records.is_empty() {
        return "_(no friction entries yet - run `agnosgram feedback \"...\"` to capture some)_"
            .to_string();
    }
    let mut sorted: Vec<&StoreRecord> = records.iter().collect();
    sorted.sort_by(|a, b| a.frontmatter.id.cmp(&b.frontmatter.id));
    let header = "| id | scope | confidence | created | excerpt |\n|---|---|---|---|---|";
    let rows: Vec<String> = sorted
        .iter()
        .map(|r| {
            let normalized = normalize_ws(&r.body);
            // TS `.slice(0, 100)` counts UTF-16 code units; from_utf16_lossy
            // matches Node's U+FFFD for a slice-split surrogate pair.
            let units: Vec<u16> = normalized.encode_utf16().collect();
            let truncated = units.len() > 100;
            let snippet =
                String::from_utf16_lossy(&units[..units.len().min(100)]).replace('|', "\\|");
            format!(
                "| {} | {} | {} | {} | {}{} |",
                r.frontmatter.id,
                r.frontmatter.scope.join(","),
                r.frontmatter.confidence,
                r.frontmatter.created,
                snippet,
                if truncated { "..." } else { "" }
            )
        })
        .collect();
    format!("{header}\n{}", rows.join("\n"))
}

fn build_prompt(months: &[String], friction: &[StoreRecord]) -> String {
    let months_line = if months.is_empty() {
        "(none yet)".to_string()
    } else {
        months
            .iter()
            .map(|m| format!("journal/{m}.md"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let digest = friction_digest(friction);
    format!(
        "# Agnosgram reflect task\n\
\n\
You are reviewing how well Agnosgram itself is serving this project - not the\n\
host project's own code or memory. Turn tool friction and recent session\n\
history into a small set of concrete improvement proposals and, where\n\
warranted, candidate roadmap milestones. Do NOT invent friction that was never\n\
reported; only propose what the sources below actually support.\n\
\n\
## Sources to read\n\
- Friction entries: `.agnosgram/meta/friction.md` (this project's tool\n\
\x20\x20friction, never host-project memory).\n\
- Recent journal months: {months_line}\n\
\x20\x20Look for recurring `Avoid`/`Learned` lines that point at friction with the\n\
\x20\x20tool itself, not the host project.\n\
- Current roadmap: `ROADMAP.md` (read-only context - see the rule below).\n\
\n\
## Digest: every friction entry currently captured\n\
{digest}\n\
\n\
## Output rules\n\
1. Group related friction into themes; do not propose one item per friction\n\
\x20\x20\x20entry when several describe the same underlying gap.\n\
2. For each proposal, state: the friction it addresses (cite `FRI-` ids),\n\
\x20\x20\x20the concrete change, and its expected effect. Keep each to a few sentences.\n\
3. Where a proposal is substantial enough to be a milestone rather than a\n\
\x20\x20\x20small fix, suggest it as a **candidate roadmap milestone** - a title and a\n\
\x20\x20\x20one-paragraph scope, not a full spec.\n\
4. Present every proposal in this reviewable form:\n\
\x20\x20\x20```\n\
\x20\x20\x20### Proposal: <short title>\n\
\x20\x20\x20- Addresses: FRI-001, FRI-004\n\
\x20\x20\x20- Change: <what to do>\n\
\x20\x20\x20- Effect: <why it helps>\n\
\x20\x20\x20- Candidate milestone: <yes/no - if yes, a one-line scope>\n\
\x20\x20\x20```\n\
\n\
## Rule: ROADMAP.md is owner-edited (non-negotiable)\n\
Print your proposals to stdout for a human to read, or if asked to keep a\n\
durable record, write them to a **new** file (e.g. a dated notes file the\n\
human names) - never to `ROADMAP.md` directly, and never to `lessons/`,\n\
`decisions/`, `context/`, or `state/status.md`. This command proposes; it\n\
never disposes. A human decides which proposals become real roadmap items and\n\
edits `ROADMAP.md` themselves.\n\
\n\
## When done\n\
Nothing to validate mechanically - this is a proposal, not a schema-checked\n\
report. If a proposal implies a store or CLI change, remember any schema or\n\
layout change must land together with a `doctor` change (CON-003).\n"
    )
}

pub fn run(argv: Vec<String>) -> Result<(), UserError> {
    let cfg = ArgsConfig::new(false)
        .option("months", OptionDef::string())
        .option("json", OptionDef::boolean(false))
        .option("format", OptionDef::string());
    let parsed = parse_cli_args(&argv, &cfg)?;

    let format = resolve_format(&FormatFlags {
        format: parsed.str("format").map(|s| s.to_string()),
        json: parsed.bool("json"),
    })?;

    let root =
        find_project_root(&std::env::current_dir().map_err(|e| UserError::new(e.to_string()))?);
    if !has_store(&root) {
        return Err(UserError::new(
            "No .agnosgram/ store found. Run `agnosgram init` first.",
        ));
    }

    let mut month_count = DEFAULT_MONTHS;
    if let Some(raw) = parsed.str("months") {
        // Strict: reject anything a naive parse would otherwise accept by
        // truncating or stopping early (e.g. "2.5" -> 2, "3abc" -> 3).
        if !is_positive_integer(raw) {
            return Err(UserError::new(format!(
                "--months must be a positive integer, got \"{raw}\""
            )));
        }
        month_count = raw.parse::<usize>().unwrap_or(usize::MAX);
    }

    let all_months = journal_months(&root);
    let start = all_months.len().saturating_sub(month_count);
    let months: Vec<String> = all_months[start..].to_vec();
    let friction = load_friction_records(&root);

    let prompt = build_prompt(&months, &friction);

    if let ResolvedFormat::Structured(fmt) = format {
        let mut out = Value::object();
        out.insert("agnosgram_reflect", REFLECT_SCHEMA_VERSION);
        out.insert("friction_count", friction.len());
        out.insert(
            "friction_ids",
            friction
                .iter()
                .map(|r| r.frontmatter.id.clone())
                .collect::<Vec<_>>(),
        );
        out.insert("journal_months", months.clone());
        out.insert("prompt", prompt);
        print_structured(&out, fmt);
    } else {
        use std::io::Write;
        let _ = std::io::stdout().write_all(prompt.as_bytes());
    }
    Ok(())
}

fn is_positive_integer(s: &str) -> bool {
    let bytes = s.as_bytes();
    !bytes.is_empty() && bytes[0] != b'0' && bytes.iter().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_positive_integer_accepts_only_a_bare_positive_integer() {
        assert!(is_positive_integer("3"));
        assert!(is_positive_integer("12"));
        assert!(!is_positive_integer("0"));
        assert!(!is_positive_integer("-1"));
        assert!(!is_positive_integer("2.5"));
        assert!(!is_positive_integer("3abc"));
        assert!(!is_positive_integer(""));
    }

    #[test]
    fn friction_digest_reports_a_placeholder_when_empty() {
        assert!(friction_digest(&[]).contains("no friction entries yet"));
    }

    #[test]
    fn build_prompt_lists_none_yet_when_no_months() {
        let prompt = build_prompt(&[], &[]);
        assert!(prompt.contains("Recent journal months: (none yet)"));
        assert!(prompt.contains("reflect task"));
        assert!(prompt.contains("owner-edited"));
    }

    #[test]
    fn build_prompt_lists_journal_paths_for_each_month() {
        let months = vec!["2026-06".to_string(), "2026-07".to_string()];
        let prompt = build_prompt(&months, &[]);
        assert!(prompt.contains("journal/2026-06.md, journal/2026-07.md"));
    }
}
