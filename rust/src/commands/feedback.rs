//! Port of `src/commands/feedback.ts`.
//!
//! `agnosgram feedback "<text>"` - capture tool friction (using Agnosgram
//! itself, not the host project) into `.agnosgram/meta/friction.md`. This is
//! a strictly separate namespace from host-project memory: `pack`/`show`/
//! `advise` never surface it (see `core/meta.rs`, `core/records.rs`). `init`
//! never scaffolds `meta/`; this command creates it on first use.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::core::args::{parse_cli_args, ArgsConfig, OptionDef};
use crate::core::config::load_config;
use crate::core::frontmatter::KNOWN_CONFIDENCE;
use crate::core::json::Value;
use crate::core::meta::{load_friction_records, FRICTION_FILE, FRICTION_TYPE};
use crate::core::output::{info, print_structured, UserError};
use crate::core::paths::{find_project_root, has_store, memory_dir};
use crate::core::serialize::{resolve_format, FormatFlags, ResolvedFormat};
use crate::core::templates::iso_date;

/// The real remote this repo lives at - not the `agnosgram/agnosgram` org
/// slug used in docs URLs, which is currently a dead link. `--share` must
/// name this exact repo, or the printed command would file the issue on
/// whatever host-project repo the CLI happens to be run from instead.
const AGNOSGRAM_REPO: &str = "brolyssjl/agnosgram";

/// Scope tags are written into YAML frontmatter as an inline flow sequence
/// (`[a, b]`); restricting them to a safe charset keeps that output canonical
/// instead of relying on parser leniency for anything containing `]`, `:`, etc.
fn is_valid_scope_tag(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

fn read_stdin() -> String {
    let mut buf = String::new();
    match std::io::stdin().read_to_string(&mut buf) {
        Ok(_) => buf,
        Err(_) => String::new(),
    }
}

fn friction_md() -> String {
    "# Friction - tool-usage friction, not host-project memory\n\
\n\
_Captured with `agnosgram feedback`. Never read by `pack`/`show`/`advise` -\n\
this namespace is about Agnosgram itself, and feeds `agnosgram reflect`. See\n\
docs/feedback.md._\n"
        .to_string()
}

/// Create meta/friction.md on first use. `feedback` writes only under
/// `.agnosgram/meta/` - never config.yml or anything else.
fn ensure_meta_store(root: &Path) -> Result<PathBuf, UserError> {
    let dir = memory_dir(root).join("meta");
    let file = dir.join("friction.md");
    if !file.exists() {
        fs::create_dir_all(&dir).map_err(|e| UserError::new(e.to_string()))?;
        fs::write(&file, friction_md()).map_err(|e| UserError::new(e.to_string()))?;
    }
    Ok(file)
}

fn next_friction_id(root: &Path) -> String {
    let mut max = 0u64;
    for rec in load_friction_records(root) {
        if let Some(rest) = rec.frontmatter.id.strip_prefix("FRI-") {
            if let Ok(n) = rest.parse::<u64>() {
                if rest.chars().all(|c| c.is_ascii_digit()) {
                    max = max.max(n);
                }
            }
        }
    }
    format!("FRI-{:03}", max + 1)
}

/// Single-quote a shell argument, escaping embedded single quotes.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// A ready-to-run (never executed by Agnosgram) `gh issue create` command,
/// printed only behind the explicit, default-off `--share` flag.
fn share_command(id: &str, text: &str, scope: &[String]) -> String {
    // TS `text.length`/`.slice(0, 69)` count UTF-16 code units, and Node
    // writes a slice-split surrogate pair as U+FFFD - from_utf16_lossy matches.
    let units: Vec<u16> = text.encode_utf16().collect();
    let title: String = if units.len() > 72 {
        format!("{}...", String::from_utf16_lossy(&units[..69]))
    } else {
        text.to_string()
    };
    let body = format!(
        "Captured via `agnosgram feedback` ({id}).\n\n{text}\n\nScope: {}",
        scope.join(", ")
    );
    format!(
        "gh issue create --repo {AGNOSGRAM_REPO} --title {} --body {}",
        shell_quote(&title),
        shell_quote(&body)
    )
}

pub fn run(argv: Vec<String>) -> Result<(), UserError> {
    let cfg = ArgsConfig::new(true)
        .option("scope", OptionDef::string())
        .option("confidence", OptionDef::string())
        .option("stdin", OptionDef::boolean(false))
        .option("share", OptionDef::boolean(false))
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
    // Validates the store is well-formed before we write.
    load_config(&root).map_err(|e| UserError::new(e.to_string()))?;

    let mut text = parsed.positionals.join(" ").trim().to_string();
    if parsed.bool("stdin") {
        let raw = read_stdin().trim().to_string();
        if raw.is_empty() {
            return Err(UserError::new("--stdin given but nothing was piped in."));
        }
        text = raw;
    }
    if text.is_empty() {
        return Err(UserError::new(
            "Usage: agnosgram feedback \"<text>\" [--scope <tag,...>] [--confidence low|medium|high] [--share]",
        ));
    }

    let confidence = parsed
        .str("confidence")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "medium".to_string());
    if !KNOWN_CONFIDENCE.contains(&confidence.as_str()) {
        return Err(UserError::new(format!(
            "--confidence must be one of {}, got \"{confidence}\"",
            KNOWN_CONFIDENCE.join(", ")
        )));
    }
    let scope_raw = parsed.str("scope").unwrap_or("cli");
    let scope: Vec<String> = scope_raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if scope.is_empty() {
        return Err(UserError::new("--scope must not be empty when given."));
    }
    for tag in &scope {
        if !is_valid_scope_tag(tag) {
            return Err(UserError::new(format!(
                "--scope tag \"{tag}\" must contain only letters, digits, dot, dash, or underscore."
            )));
        }
    }

    let file = ensure_meta_store(&root)?;
    let id = next_friction_id(&root);
    let date = iso_date();
    let entry = format!(
        "\n---\nid: {id}\ntype: {FRICTION_TYPE}\nscope: [{}]\nconfidence: {confidence}\ncreated: {date}\nlast_verified: {date}\nsource: {FRICTION_FILE}\n---\n{text}\n",
        scope.join(", ")
    );

    let prior = fs::read_to_string(&file).map_err(|e| UserError::new(e.to_string()))?;
    let trimmed = prior.trim_end_matches('\n');
    fs::write(&file, format!("{trimmed}\n{entry}")).map_err(|e| UserError::new(e.to_string()))?;

    let rel_file = format!(".agnosgram/{FRICTION_FILE}");
    let share = if parsed.bool("share") {
        Some(share_command(&id, &text, &scope))
    } else {
        None
    };

    if let ResolvedFormat::Structured(fmt) = format {
        let mut out = Value::object();
        out.insert("id", id);
        out.insert("file", rel_file);
        out.insert("scope", scope.clone());
        out.insert("confidence", confidence);
        out.insert("text", text);
        out.insert("share", share.clone());
        print_structured(&out, fmt);
        return Ok(());
    }

    info(&format!("Logged {id} to {rel_file}"));
    if let Some(share) = &share {
        info("");
        info("Share this as a GitHub issue - run it yourself, Agnosgram never executes it:");
        info(&format!("  {share}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_root(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("agnos-feedback-rs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(".agnosgram")).unwrap();
        dir
    }

    #[test]
    fn is_valid_scope_tag_accepts_safe_charset_only() {
        assert!(is_valid_scope_tag("cli"));
        assert!(is_valid_scope_tag("a.b_c-9"));
        assert!(!is_valid_scope_tag("cli]x"));
        assert!(!is_valid_scope_tag("a: b"));
        assert!(!is_valid_scope_tag(""));
    }

    #[test]
    fn shell_quote_escapes_embedded_single_quotes() {
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
        assert_eq!(shell_quote("plain"), "'plain'");
    }

    #[test]
    fn share_command_truncates_long_titles_and_pins_the_repo() {
        let cmd = share_command("FRI-001", "short text", &["cli".to_string()]);
        assert!(cmd.contains("--repo brolyssjl/agnosgram"));
        assert!(cmd.contains("gh issue create"));

        let long_text = "x".repeat(100);
        let cmd2 = share_command("FRI-002", &long_text, &["cli".to_string()]);
        let title_start = cmd2.find("--title '").unwrap() + "--title '".len();
        let title_end = cmd2[title_start..].find("' --body").unwrap() + title_start;
        let title = &cmd2[title_start..title_end];
        assert_eq!(title.chars().count(), 72);
        assert!(title.ends_with("..."));
    }

    #[test]
    fn next_friction_id_starts_at_001_and_increments() {
        let root = tmp_root("ids");
        assert_eq!(next_friction_id(&root), "FRI-001");
        fs::create_dir_all(root.join(".agnosgram/meta")).unwrap();
        fs::write(
            root.join(".agnosgram/meta/friction.md"),
            "# Friction\n\n---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: meta/friction.md\n---\nEntry one.\n",
        )
        .unwrap();
        assert_eq!(next_friction_id(&root), "FRI-002");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn ensure_meta_store_creates_the_file_once() {
        let root = tmp_root("ensure");
        let file = ensure_meta_store(&root).unwrap();
        assert!(file.exists());
        let text = fs::read_to_string(&file).unwrap();
        assert!(text.contains("Friction - tool-usage friction"));
        fs::remove_dir_all(&root).unwrap();
    }
}
