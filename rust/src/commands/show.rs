//! Port of `src/commands/show.ts`: print stored records matching a topic, for
//! agents with weak file navigation (or humans who just want the relevant
//! slice without opening `.agnosgram/` by hand). Read-only; never edits the
//! store.
//!
//! Match order (first non-empty wins, no fuzzy matching - deferred):
//!   1. exact record id (`LES-001`)
//!   2. case-insensitive exact scope tag (`backend`, `Backend`, ...)
//!   3. type name (`pitfall` | `convention` | `decision`)

use std::env;

use crate::core::args::{parse_cli_args, ArgsConfig, OptionDef};
use crate::core::frontmatter::KNOWN_TYPES;
use crate::core::json::Value;
use crate::core::output::{info, print_structured, warn, UserError};
use crate::core::paths::{find_project_root, has_store};
use crate::core::records::{
    all_scopes_from, load_records, render_record_block, to_flat_record, FlatRecord, StoreRecord,
};
use crate::core::serialize::{resolve_format, FormatFlags, ResolvedFormat};

pub fn match_records<'a>(
    records: &'a [StoreRecord],
    topic: &str,
    type_filter: Option<&str>,
) -> Vec<&'a StoreRecord> {
    let pool: Vec<&StoreRecord> = match type_filter {
        Some(t) => records
            .iter()
            .filter(|r| r.frontmatter.r#type == t)
            .collect(),
        None => records.iter().collect(),
    };
    let topic_lower = topic.to_lowercase();

    let by_id: Vec<&StoreRecord> = pool
        .iter()
        .copied()
        .filter(|r| r.frontmatter.id == topic)
        .collect();
    if !by_id.is_empty() {
        return by_id;
    }

    let by_scope: Vec<&StoreRecord> = pool
        .iter()
        .copied()
        .filter(|r| {
            r.frontmatter
                .scope
                .iter()
                .any(|s| s.to_lowercase() == topic_lower)
        })
        .collect();
    if !by_scope.is_empty() {
        return by_scope;
    }

    if KNOWN_TYPES.contains(&topic_lower.as_str()) {
        let by_type: Vec<&StoreRecord> = pool
            .iter()
            .copied()
            .filter(|r| r.frontmatter.r#type == topic_lower)
            .collect();
        if !by_type.is_empty() {
            return by_type;
        }
    }

    Vec::new()
}

/// Human rendering: records as they look on disk, grouped under a per-file heading.
fn render_human(records: &[&StoreRecord]) -> String {
    if records.is_empty() {
        return String::new();
    }
    let mut lines: Vec<String> = Vec::new();
    let mut current_file = String::new();
    for r in records {
        if r.file != current_file {
            current_file = r.file.clone();
            lines.push(format!("## {current_file}"));
            lines.push(String::new());
        }
        lines.push(render_record_block(r));
        lines.push(String::new());
    }
    let joined = lines.join("\n");
    format!("{}\n", joined.trim_end())
}

/// Shared with `pack.rs`: TS's `toFlatRecord` output, as a JSON/TOON `Value`.
pub fn flat_record_to_value(r: &FlatRecord) -> Value {
    let mut o = Value::object();
    o.insert("id", r.id.clone());
    o.insert("type", r.r#type.clone());
    o.insert("scope", r.scope.clone());
    o.insert("confidence", r.confidence.clone());
    o.insert("created", r.created.clone());
    o.insert("last_verified", r.last_verified.clone());
    o.insert("source", r.source.clone());
    o.insert("supersedes", r.supersedes.clone());
    o.insert("file", r.file.clone());
    o.insert("body", r.body.clone());
    o
}

fn matches_to_value(matches: &[&StoreRecord]) -> Value {
    Value::Array(
        matches
            .iter()
            .map(|r| flat_record_to_value(&to_flat_record(r)))
            .collect(),
    )
}

pub fn run(argv: Vec<String>) -> Result<(), UserError> {
    let cfg = ArgsConfig::new(true)
        .option("type", OptionDef::string())
        .option("json", OptionDef::boolean(false))
        .option("format", OptionDef::string());
    let parsed = parse_cli_args(&argv, &cfg)?;

    let format = resolve_format(&FormatFlags {
        format: parsed.str("format").map(|s| s.to_string()),
        json: parsed.bool("json"),
    })?;

    let topic = parsed.positionals.first().cloned().unwrap_or_default();
    if topic.trim().is_empty() {
        return Err(UserError::new(
            "Usage: agnosgram show <topic> [--type pitfall|convention|decision]",
        ));
    }

    if let Some(t) = parsed.str("type") {
        if !KNOWN_TYPES.contains(&t) {
            return Err(UserError::new(format!(
                "--type must be one of {}, got \"{t}\"",
                KNOWN_TYPES.join(", ")
            )));
        }
    }

    let cwd = env::current_dir().map_err(|e| UserError::new(e.to_string()))?;
    let root = find_project_root(&cwd);
    if !has_store(&root) {
        return Err(UserError::new(
            "No .agnosgram/ store found. Run `agnosgram init` first.",
        ));
    }

    let records = load_records(&root);
    let matches = match_records(&records, topic.trim(), parsed.str("type"));

    if matches.is_empty() {
        match format {
            ResolvedFormat::Human => {
                // Human hints go to stderr so a downstream pipe consuming
                // stdout never sees them mixed into the (empty) result.
                let scopes = all_scopes_from(&records);
                warn(&format!("No records match \"{topic}\"."));
                if !scopes.is_empty() {
                    warn(&format!(
                        "Known scopes: {}. Types: {}.",
                        scopes.join(", "),
                        KNOWN_TYPES.join(", ")
                    ));
                } else {
                    warn(&format!(
                        "Types: {}. (No scope tags in the store yet.)",
                        KNOWN_TYPES.join(", ")
                    ));
                }
            }
            ResolvedFormat::Structured(f) => {
                // Structured modes always print a well-formed payload, even
                // when empty, so a caller parsing stdout never sees a
                // blank/absent response on a miss - only the exit code
                // signals "no match".
                print_structured(&matches_to_value(&matches), f);
            }
        }
        std::process::exit(1);
    }

    match format {
        ResolvedFormat::Human => info(render_human(&matches).trim_end()),
        ResolvedFormat::Structured(f) => print_structured(&matches_to_value(&matches), f),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::frontmatter::{extract_records, validate_record, KNOWN_TYPES};

    fn record(text: &str, file: &str) -> StoreRecord {
        let raw = extract_records(text).remove(0);
        let validated = validate_record(&raw, &KNOWN_TYPES);
        StoreRecord {
            frontmatter: validated.frontmatter.expect("valid record"),
            body: raw.body,
            file: file.to_string(),
            store_rel: file.to_string(),
            line: raw.line,
        }
    }

    fn fixture() -> Vec<StoreRecord> {
        vec![
            record(
                "---\nid: LES-001\ntype: pitfall\nscope: [core, tooling]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nDo not use require().\n",
                "lessons/pitfalls.md",
            ),
            record(
                "---\nid: CON-001\ntype: convention\nscope: [tooling]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nAlways use node built-ins.\n",
                "lessons/conventions.md",
            ),
        ]
    }

    #[test]
    fn matches_an_exact_id_before_anything_else() {
        let records = fixture();
        let matches = match_records(&records, "LES-001", None);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].frontmatter.id, "LES-001");
    }

    #[test]
    fn matches_a_case_insensitive_scope_tag_when_no_id_matches() {
        let records = fixture();
        let matches = match_records(&records, "Tooling", None);
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn matches_a_type_name_as_a_last_resort() {
        let records = fixture();
        let matches = match_records(&records, "convention", None);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].frontmatter.id, "CON-001");
    }

    #[test]
    fn type_filter_narrows_the_pool_before_matching() {
        let records = fixture();
        let matches = match_records(&records, "tooling", Some("convention"));
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].frontmatter.id, "CON-001");
    }

    #[test]
    fn no_match_returns_an_empty_list() {
        let records = fixture();
        assert!(match_records(&records, "nonexistent-topic", None).is_empty());
    }
}
