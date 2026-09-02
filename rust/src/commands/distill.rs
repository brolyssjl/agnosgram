//! Port of `src/commands/distill.ts`.

use std::fs;
use std::path::Path;

use crate::core::args::{parse_cli_args, ArgsConfig, OptionDef};
use crate::core::config::{load_config, AgnosgramConfig};
use crate::core::frontmatter::{extract_records, validate_record, KNOWN_CONFIDENCE, KNOWN_TYPES};
use crate::core::json::Value;
use crate::core::output::{info, print_json, UserError};
use crate::core::paths::{find_project_root, has_store, memory_dir};
use crate::core::records::iter_raw_records;
use crate::core::store::journal_months;
use crate::core::tokens::estimate_tokens;

/// Every record id already in the store, so the distiller allocates fresh
/// ones. `exclude`, when given, is a project-root-relative path (e.g.
/// `.agnosgram/lessons/pitfalls.md`) to skip - matches `iterRawRecords`'
/// `file.rel` shape.
fn existing_ids(root: &Path, exclude: Option<&str>) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for (file, raw) in iter_raw_records(root) {
        if Some(file.rel.as_str()) == exclude {
            continue;
        }
        if let Some((_, v)) = raw.data.iter().find(|(k, _)| k == "id") {
            if let Some(s) = v.as_str() {
                ids.push(s.to_string());
            }
        }
    }
    ids.sort();
    ids
}

fn build_prompt(root: &Path, config: &AgnosgramConfig) -> String {
    let months = journal_months(root);
    let ids = existing_ids(root, None);
    let budget_lines = config
        .budgets
        .iter()
        .map(|(f, b)| format!("  - {f}: {b} tokens"))
        .collect::<Vec<_>>()
        .join("\n");

    let months_line = if months.is_empty() {
        "(none yet)".to_string()
    } else {
        months
            .iter()
            .map(|m| format!("journal/{m}.md"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let ids_line = if ids.is_empty() {
        "(none)".to_string()
    } else {
        ids.join(", ")
    };
    let known_types = KNOWN_TYPES.join(" | ");
    let known_confidence = KNOWN_CONFIDENCE.join(" | ");

    format!(
        "# Agnosgram distillation task\n\
\n\
You are curating this project's memory store at `.agnosgram/`. Capture is cheap\n\
(the append-only journal); distillation is deliberate. Turn raw journal entries\n\
into a small set of durable, non-overlapping lessons and decisions - and shrink\n\
what is already there. Do NOT invent facts; only distill what the sources support.\n\
\n\
## Sources to read\n\
- Journal months: {months_line}\n\
\x20\x20`Learned`/`Avoid` lines are pitfall candidates; `Decided` lines are decision candidates.\n\
- Existing curated memory: lessons/pitfalls.md, lessons/conventions.md, decisions/*.md.\n\
\n\
## Output rules\n\
1. Write into: lessons/pitfalls.md (type: pitfall), lessons/conventions.md\n\
\x20\x20\x20(type: convention), decisions/NNNN-slug.md (type: decision).\n\
2. Every record carries this frontmatter, fenced by `---` lines:\n\
\x20\x20\x20```\n\
\x20\x20\x20---\n\
\x20\x20\x20id: <PREFIX-NNN>          # LES-*/CON-* for lessons, DEC-* for decisions; unique\n\
\x20\x20\x20type: <{known_types}>\n\
\x20\x20\x20scope: [area, ...]        # non-empty list\n\
\x20\x20\x20confidence: <{known_confidence}>\n\
\x20\x20\x20created: YYYY-MM-DD\n\
\x20\x20\x20last_verified: YYYY-MM-DD\n\
\x20\x20\x20source: journal/YYYY-MM.md\n\
\x20\x20\x20supersedes: [OLD-ID, ...] # optional; REQUIRED when you merge/replace records\n\
\x20\x20\x20---\n\
\x20\x20\x20<one tight paragraph of body>\n\
\x20\x20\x20```\n\
3. **Merge, do not append.** If a new insight overlaps an existing record, rewrite\n\
\x20\x20\x20the existing one and list the ids it replaces under `supersedes:`. Never leave\n\
\x20\x20\x20two near-duplicate records side by side.\n\
4. Do not reuse an existing id. Ids already taken: {ids_line}.\n\
5. Stay within per-file token budgets:\n\
{budget_lines}\n\
6. **Required output if you touch state/status.md or lessons/*.md:** also update the\n\
\x20\x20\x20matching row(s) in MEMORY.md's `## Freshness` table (the `Last verified` column)\n\
\x20\x20\x20to today's date. That table - not status.md's own `_Last updated:_` line - is\n\
\x20\x20\x20what `doctor`'s `status.stale` check and `file.stale` check both read; updating\n\
\x20\x20\x20only the file itself leaves the store looking stale to `doctor --strict`.\n\
7. The human reviews this in a PR. Keep bodies terse and factual.\n\
\n\
## Validate your result (mechanical, no LLM)\n\
Run these and fix anything they report before finishing:\n\
- `agnosgram distill --validate lessons/pitfalls.md` (repeat per file you touched)\n\
- `agnosgram doctor --strict` (also confirms MEMORY.md's Freshness table agrees with\n\
\x20\x20status.md; see `freshness.mismatch` if it doesn't)\n\
\n\
## After the human accepts the distilled records\n\
Archive the journal months you fully absorbed so they stop counting against budgets\n\
and re-distillation: `agnosgram distill --archive <YYYY-MM>`.\n"
    )
}

#[derive(Debug)]
struct ValidateIssue {
    level: &'static str,
    code: String,
    line: Option<usize>,
    message: String,
}

#[derive(Debug)]
struct ValidateResult {
    file: String,
    ok: bool,
    errors: usize,
    warnings: usize,
    issues: Vec<ValidateIssue>,
}

fn validate_file(root: &Path, rel_arg: &str) -> Result<ValidateResult, UserError> {
    let store_rel = rel_arg
        .strip_prefix(".agnosgram/")
        .unwrap_or(rel_arg)
        .to_string();
    let abs = memory_dir(root).join(&store_rel);
    if !abs.exists() {
        return Err(UserError::new(format!(
            "No such file: .agnosgram/{store_rel}"
        )));
    }
    let text = fs::read_to_string(&abs).map_err(|e| UserError::new(e.to_string()))?;
    let mut issues: Vec<ValidateIssue> = Vec::new();

    let records = extract_records(&text);
    let mut seen: Vec<(String, usize)> = Vec::new();
    // `existingIds` compares against slash-normalized project-relative paths, so
    // build the exclusion the same way.
    let exclude = format!(".agnosgram/{store_rel}");
    let other_ids = existing_ids(root, Some(&exclude));

    for raw in &records {
        let validated = validate_record(raw, &KNOWN_TYPES);
        for i in validated.issues {
            let level = match i.level {
                crate::core::frontmatter::IssueLevel::Error => "error",
                crate::core::frontmatter::IssueLevel::Warn => "warn",
            };
            issues.push(ValidateIssue {
                level,
                code: format!("schema.{}", i.code),
                line: Some(i.line),
                message: i.message,
            });
        }
        let id = raw
            .data
            .iter()
            .find(|(k, _)| k == "id")
            .and_then(|(_, v)| v.as_str());
        if let Some(id) = id {
            if let Some((_, first_line)) = seen.iter().find(|(existing, _)| existing == id) {
                issues.push(ValidateIssue {
                    level: "error",
                    code: "id.duplicate".to_string(),
                    line: Some(raw.line),
                    message: format!("id \"{id}\" repeats in this file (line {first_line})"),
                });
            } else {
                seen.push((id.to_string(), raw.line));
            }
            if other_ids.iter().any(|o| o == id) {
                issues.push(ValidateIssue {
                    level: "error",
                    code: "id.collision".to_string(),
                    line: Some(raw.line),
                    message: format!("id \"{id}\" already exists elsewhere in the store"),
                });
            }
        }
    }

    // Budget check when this file has a configured budget.
    let config = load_config(root).map_err(|e| UserError::new(e.to_string()))?;
    if let Some((_, budget)) = config.budgets.iter().find(|(k, _)| k == &store_rel) {
        let tokens = estimate_tokens(&text);
        if tokens > *budget {
            issues.push(ValidateIssue {
                level: "error",
                code: "budget.over".to_string(),
                line: None,
                message: format!("~{tokens} tokens over the {budget}-token budget"),
            });
        }
    }

    let errors = issues.iter().filter(|i| i.level == "error").count();
    let warnings = issues.iter().filter(|i| i.level == "warn").count();
    Ok(ValidateResult {
        file: format!(".agnosgram/{store_rel}"),
        ok: errors == 0,
        errors,
        warnings,
        issues,
    })
}

fn validate_result_to_json(r: &ValidateResult) -> Value {
    let mut out = Value::object();
    out.insert("file", r.file.clone());
    out.insert("ok", r.ok);
    out.insert("errors", r.errors);
    out.insert("warnings", r.warnings);
    let mut issues = Value::array();
    for i in &r.issues {
        let mut o = Value::object();
        o.insert("level", i.level);
        o.insert("code", i.code.clone());
        if let Some(line) = i.line {
            o.insert("line", line);
        }
        o.insert("message", i.message.clone());
        issues.push(o);
    }
    out.insert("issues", issues);
    out
}

fn archive_month(root: &Path, month: &str) -> Result<String, UserError> {
    if !is_year_month(month) {
        return Err(UserError::new(format!(
            "--archive expects a YYYY-MM month, got \"{month}\"."
        )));
    }
    let src = memory_dir(root).join("journal").join(format!("{month}.md"));
    if !src.exists() {
        return Err(UserError::new(format!(
            "No journal for {month} (looked for .agnosgram/journal/{month}.md)."
        )));
    }
    let archive_dir = memory_dir(root).join("journal").join("archive");
    fs::create_dir_all(&archive_dir).map_err(|e| UserError::new(e.to_string()))?;
    let dest = archive_dir.join(format!("{month}.md"));
    if dest.exists() {
        return Err(UserError::new(format!(
            "Already archived: .agnosgram/journal/archive/{month}.md exists."
        )));
    }
    fs::rename(&src, &dest).map_err(|e| UserError::new(e.to_string()))?;
    Ok(format!(".agnosgram/journal/archive/{month}.md"))
}

fn is_year_month(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() == 7
        && bytes[0..4].iter().all(|b| b.is_ascii_digit())
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
}

pub fn run(argv: Vec<String>) -> Result<(), UserError> {
    let cfg = ArgsConfig::new(false)
        .option("validate", OptionDef::string())
        .option("archive", OptionDef::string())
        .option("json", OptionDef::boolean(false));
    let parsed = parse_cli_args(&argv, &cfg)?;

    let root =
        find_project_root(&std::env::current_dir().map_err(|e| UserError::new(e.to_string()))?);
    if !has_store(&root) {
        return Err(UserError::new(
            "No .agnosgram/ store found. Run `agnosgram init` first.",
        ));
    }

    if let Some(month_raw) = parsed.str("archive") {
        let dest = archive_month(&root, month_raw.trim())?;
        if parsed.bool("json") {
            let mut out = Value::object();
            out.insert("archived", dest);
            print_json(&out);
        } else {
            info(&format!("Archived to {dest}"));
        }
        return Ok(());
    }

    if let Some(validate_arg) = parsed.str("validate") {
        let result = validate_file(&root, validate_arg.trim())?;
        if parsed.bool("json") {
            print_json(&validate_result_to_json(&result));
        } else if result.issues.is_empty() {
            info(&format!("{}: valid.", result.file));
        } else {
            for i in &result.issues {
                let where_ = i.line.map(|l| format!(":{l}")).unwrap_or_default();
                let level = if i.level == "error" { "error" } else { "warn " };
                info(&format!("  {level} {}{where_}  {}", i.code, i.message));
            }
            info(&format!(
                "\n{}: {} error(s), {} warning(s).",
                result.file, result.errors, result.warnings
            ));
        }
        if result.errors > 0 {
            use std::io::Write;
            let _ = std::io::stdout().flush();
            std::process::exit(1);
        }
        return Ok(());
    }

    let config = load_config(&root).map_err(|e| UserError::new(e.to_string()))?;
    let prompt = build_prompt(&root, &config);
    if parsed.bool("json") {
        let mut out = Value::object();
        out.insert("prompt", prompt);
        print_json(&out);
    } else {
        use std::io::Write;
        let _ = std::io::stdout().write_all(prompt.as_bytes());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::default_config;
    use std::fs;

    fn tmp_root(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("agnos-distill-rs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(".agnosgram/lessons")).unwrap();
        fs::create_dir_all(dir.join(".agnosgram/journal")).unwrap();
        dir
    }

    #[test]
    fn is_year_month_accepts_and_rejects() {
        assert!(is_year_month("2026-08"));
        assert!(!is_year_month("nope"));
        assert!(!is_year_month("-2026-08"));
        assert!(!is_year_month("2026-8"));
    }

    #[test]
    fn existing_ids_collects_sorted_ids_and_excludes_the_given_file() {
        let root = tmp_root("ids");
        fs::write(
            root.join(".agnosgram/lessons/pitfalls.md"),
            "---\nid: LES-002\ntype: pitfall\nscope: [x]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nbody\n\n---\nid: LES-001\ntype: pitfall\nscope: [x]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nbody2\n",
        )
        .unwrap();
        let ids = existing_ids(&root, None);
        assert_eq!(ids, vec!["LES-001".to_string(), "LES-002".to_string()]);
        let excluded = existing_ids(&root, Some(".agnosgram/lessons/pitfalls.md"));
        assert!(excluded.is_empty());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn validate_file_reports_no_such_file() {
        let root = tmp_root("missing");
        let err = validate_file(&root, "lessons/nope.md").unwrap_err();
        assert_eq!(err.0, "No such file: .agnosgram/lessons/nope.md");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn validate_file_accepts_a_well_formed_record() {
        let root = tmp_root("valid");
        fs::write(
            root.join(".agnosgram/config.yml"),
            crate::core::config::serialize_config(&default_config()),
        )
        .unwrap();
        fs::write(
            root.join(".agnosgram/lessons/pitfalls.md"),
            "# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nA valid lesson.\n",
        )
        .unwrap();
        let result = validate_file(&root, "lessons/pitfalls.md").unwrap();
        assert!(result.ok);
        assert_eq!(result.errors, 0);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn validate_file_flags_a_broken_record_and_leading_prefix_strip() {
        let root = tmp_root("broken");
        fs::write(
            root.join(".agnosgram/config.yml"),
            crate::core::config::serialize_config(&default_config()),
        )
        .unwrap();
        fs::write(
            root.join(".agnosgram/lessons/pitfalls.md"),
            "# Pitfalls\n\n---\nid: bad\ntype: pitfall\n---\nbroken\n",
        )
        .unwrap();
        let result = validate_file(&root, ".agnosgram/lessons/pitfalls.md").unwrap();
        assert!(!result.ok);
        assert!(result.errors > 0);
        assert_eq!(result.file, ".agnosgram/lessons/pitfalls.md");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn archive_month_rejects_a_bad_month() {
        let root = tmp_root("archive-bad");
        let err = archive_month(&root, "nope").unwrap_err();
        assert!(err.0.contains("--archive expects a YYYY-MM month"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn archive_month_moves_the_journal_file() {
        let root = tmp_root("archive-ok");
        fs::write(root.join(".agnosgram/journal/2026-08.md"), "# Journal\n").unwrap();
        let dest = archive_month(&root, "2026-08").unwrap();
        assert_eq!(dest, ".agnosgram/journal/archive/2026-08.md");
        assert!(!root.join(".agnosgram/journal/2026-08.md").exists());
        assert!(root.join(".agnosgram/journal/archive/2026-08.md").exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn archive_month_rejects_already_archived() {
        let root = tmp_root("archive-dup");
        fs::write(root.join(".agnosgram/journal/2026-08.md"), "# Journal\n").unwrap();
        archive_month(&root, "2026-08").unwrap();
        fs::write(root.join(".agnosgram/journal/2026-08.md"), "# Journal 2\n").unwrap();
        let err = archive_month(&root, "2026-08").unwrap_err();
        assert!(err.0.contains("Already archived"));
        fs::remove_dir_all(&root).unwrap();
    }
}
