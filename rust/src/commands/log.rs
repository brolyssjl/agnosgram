//! Port of `src/commands/log.ts`.

use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use crate::core::args::{parse_cli_args, ArgsConfig, OptionDef};
use crate::core::config::load_config;
use crate::core::dates::local_timestamp;
use crate::core::json::Value;
use crate::core::output::{info, print_structured, UserError};
use crate::core::paths::{find_project_root, has_store, memory_dir};
use crate::core::serialize::{resolve_format, FormatFlags, ResolvedFormat};
use crate::core::templates::{journal_md, journal_month_now};

const SLOTS: [(&str, &str); 5] = [
    ("did", "Did"),
    ("learned", "Learned"),
    ("decided", "Decided"),
    ("avoid", "Avoid"),
    ("next", "Next"),
];

struct SlotValues {
    did: Option<String>,
    learned: Option<String>,
    decided: Option<String>,
    avoid: Option<String>,
    next: Option<String>,
}

impl SlotValues {
    fn get(&self, key: &str) -> Option<&str> {
        match key {
            "did" => self.did.as_deref(),
            "learned" => self.learned.as_deref(),
            "decided" => self.decided.as_deref(),
            "avoid" => self.avoid.as_deref(),
            "next" => self.next.as_deref(),
            _ => None,
        }
    }
}

/// Best-effort current branch from `.git/HEAD`; `None` if not a repo or detached.
pub fn current_branch(root: &Path) -> Option<String> {
    let head = root.join(".git").join("HEAD");
    let text = fs::read_to_string(head).ok()?;
    let trimmed = text.trim();
    // `/ref:\s*refs\/heads\/(.+)/`: find "ref:", then (after optional
    // whitespace) require "refs/heads/" to follow immediately, then capture
    // the rest of that line.
    let idx = trimmed.find("ref:")?;
    let rest = trimmed[idx + "ref:".len()..].trim_start();
    let marker = "refs/heads/";
    let after = rest.strip_prefix(marker)?;
    let end = after.find('\n').unwrap_or(after.len());
    let captured = after[..end].trim();
    if captured.is_empty() {
        None
    } else {
        Some(captured.to_string())
    }
}

/// Port of `formatEntry`.
fn format_entry(slots: &SlotValues, agent: &str, branch: Option<&str>, timestamp: &str) -> String {
    let mut heading_parts: Vec<String> = vec![
        "##".to_string(),
        timestamp.to_string(),
        "\u{b7}".to_string(),
        agent.to_string(),
    ];
    if let Some(b) = branch {
        heading_parts.push("\u{b7}".to_string());
        heading_parts.push(b.to_string());
    }
    let mut lines: Vec<String> = vec![heading_parts.join(" ")];
    for (key, label) in SLOTS {
        if let Some(value) = slots.get(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                lines.push(format!("- **{label}:** {trimmed}"));
            }
        }
    }
    format!("{}\n", lines.join("\n"))
}

fn read_stdin() -> String {
    let mut buf = String::new();
    match std::io::stdin().read_to_string(&mut buf) {
        Ok(_) => buf,
        Err(_) => String::new(),
    }
}

pub fn run(argv: Vec<String>) -> Result<(), UserError> {
    let cfg = ArgsConfig::new(false)
        .option("did", OptionDef::string())
        .option("learned", OptionDef::string())
        .option("decided", OptionDef::string())
        .option("avoid", OptionDef::string())
        .option("next", OptionDef::string())
        .option("agent", OptionDef::string())
        .option("branch", OptionDef::string())
        .option("stdin", OptionDef::boolean(false))
        .option("json", OptionDef::boolean(false))
        .option("format", OptionDef::string());
    let parsed = parse_cli_args(&argv, &cfg)?;

    let format = resolve_format(&FormatFlags {
        format: parsed.str("format").map(|s| s.to_string()),
        json: parsed.bool("json"),
    })?;

    let cwd = env::current_dir().map_err(|e| UserError::new(e.to_string()))?;
    let root = find_project_root(&cwd);
    if !has_store(&root) {
        return Err(UserError::new(
            "No .agnosgram/ store found. Run `agnosgram init` first.",
        ));
    }
    // Validates the store is well-formed before we append.
    load_config(&root).map_err(|e| UserError::new(e.to_string()))?;

    let agent = parsed
        .str("agent")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            env::var("AGNOSGRAM_AGENT")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "agent".to_string());

    let branch = parsed
        .str("branch")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| current_branch(&root));

    let entry: String = if parsed.bool("stdin") {
        let raw_full = read_stdin();
        let raw = raw_full.trim();
        if raw.is_empty() {
            return Err(UserError::new("--stdin given but nothing was piped in."));
        }
        if raw.starts_with("##") {
            format!("{raw}\n")
        } else {
            let branch_part = branch
                .as_deref()
                .map(|b| format!(" \u{b7} {b}"))
                .unwrap_or_default();
            format!(
                "## {} \u{b7} {}{}\n{}\n",
                local_timestamp(),
                agent,
                branch_part,
                raw
            )
        }
    } else {
        let slots = SlotValues {
            did: parsed.str("did").map(|s| s.to_string()),
            learned: parsed.str("learned").map(|s| s.to_string()),
            decided: parsed.str("decided").map(|s| s.to_string()),
            avoid: parsed.str("avoid").map(|s| s.to_string()),
            next: parsed.str("next").map(|s| s.to_string()),
        };
        let has_any = SLOTS.iter().any(|(key, _)| {
            slots
                .get(key)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
        });
        if !has_any {
            return Err(UserError::new(
                "Nothing to log. Provide at least one of --did/--learned/--decided/--avoid/--next, \
                 or pipe a full entry with --stdin.",
            ));
        }
        format_entry(&slots, &agent, branch.as_deref(), &local_timestamp())
    };

    let month = journal_month_now();
    let file = memory_dir(&root)
        .join("journal")
        .join(format!("{month}.md"));
    if !file.exists() {
        fs::write(&file, journal_md(&month)).map_err(|e| UserError::new(e.to_string()))?;
    }
    {
        let mut f = fs::OpenOptions::new()
            .append(true)
            .open(&file)
            .map_err(|e| UserError::new(e.to_string()))?;
        f.write_all(format!("\n{entry}").as_bytes())
            .map_err(|e| UserError::new(e.to_string()))?;
    }

    let rel_file = format!(".agnosgram/journal/{month}.md");
    if let ResolvedFormat::Structured(f) = format {
        let mut v = Value::object();
        v.insert("file", rel_file.clone());
        v.insert("agent", agent.clone());
        v.insert("branch", branch.clone());
        v.insert("entry", entry.trim_end().to_string());
        print_structured(&v, f);
        return Ok(());
    }
    info(&format!("Logged to {rel_file}"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("agnos-log-rs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn format_entry_only_emits_provided_slots() {
        let slots = SlotValues {
            did: Some("shipped".to_string()),
            learned: None,
            decided: None,
            avoid: None,
            next: Some("monitor".to_string()),
        };
        let entry = format_entry(&slots, "claude", Some("feat/x"), "2026-07-21 14:20");
        assert!(entry.contains("\u{b7} claude \u{b7} feat/x"));
        assert!(entry.contains("- **Did:** shipped"));
        assert!(entry.contains("- **Next:** monitor"));
        assert!(!entry.contains("Learned"));
    }

    #[test]
    fn current_branch_returns_none_outside_a_git_repo() {
        let root = tmp_dir("branch");
        assert_eq!(current_branch(&root), None);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn current_branch_reads_the_branch_name_from_git_head() {
        let root = tmp_dir("branch-hit");
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/HEAD"), "ref: refs/heads/feat/x\n").unwrap();
        assert_eq!(current_branch(&root), Some("feat/x".to_string()));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn current_branch_is_none_when_head_is_detached() {
        let root = tmp_dir("branch-detached");
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/HEAD"), "abcdef1234567890\n").unwrap();
        assert_eq!(current_branch(&root), None);
        fs::remove_dir_all(&root).unwrap();
    }
}
