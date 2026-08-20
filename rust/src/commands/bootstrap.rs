//! Port of `src/commands/bootstrap.ts`.

use std::fs;
use std::path::Path;

use crate::core::args::{parse_cli_args, ArgsConfig, OptionDef};
use crate::core::config::{load_config, AgnosgramConfig};
use crate::core::detect::{detect_agents, detect_sdd};
use crate::core::json::Value;
use crate::core::output::{print_json, UserError};
use crate::core::paths::{find_project_root, has_store};

/// Files whose presence names a stack, so the prompt can point the agent at them.
const STACK_SIGNALS: &[(&str, &str)] = &[
    (
        "package.json",
        "Node/JavaScript or TypeScript (package.json)",
    ),
    ("tsconfig.json", "TypeScript (tsconfig.json)"),
    ("go.mod", "Go (go.mod)"),
    ("Cargo.toml", "Rust (Cargo.toml)"),
    ("pyproject.toml", "Python (pyproject.toml)"),
    ("requirements.txt", "Python (requirements.txt)"),
    ("pom.xml", "Java/Maven (pom.xml)"),
    ("build.gradle", "JVM/Gradle (build.gradle)"),
    ("Gemfile", "Ruby (Gemfile)"),
    ("composer.json", "PHP (composer.json)"),
];

const IGNORE_ENTRIES: &[&str] = &[
    ".git",
    ".agnosgram",
    "node_modules",
    "dist",
    "build",
    ".next",
    "target",
    "vendor",
    ".venv",
    "__pycache__",
];

fn top_level(root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut out: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if IGNORE_ENTRIES.contains(&name.as_str()) || name.starts_with('.') {
                return None;
            }
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            Some(if is_dir { format!("{name}/") } else { name })
        })
        .collect();
    out.sort();
    out
}

fn all_entry_names(root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect()
}

fn budget_str(config: &AgnosgramConfig, key: &str) -> String {
    match config.budgets.iter().find(|(k, _)| k == key) {
        Some((_, v)) => v.to_string(),
        None => "n/a".to_string(),
    }
}

fn build_prompt(root: &Path) -> String {
    let entries = top_level(root);
    let dir_names = all_entry_names(root);
    let stack: Vec<&str> = STACK_SIGNALS
        .iter()
        .filter(|(file, _)| dir_names.iter().any(|n| n == file))
        .map(|(_, hint)| *hint)
        .collect();
    let sdd: Vec<String> = detect_sdd(root)
        .iter()
        .map(|f| {
            crate::core::detect::SDD_FRAMEWORKS
                .iter()
                .find(|fr| fr.key == f.key)
                .map(|fr| fr.name.to_string())
                .unwrap_or_else(|| f.key.to_string())
        })
        .collect();
    let agents: Vec<&str> = detect_agents(root).iter().map(|a| a.name).collect();
    let config = load_config(root).unwrap_or_else(|_| crate::core::config::default_config());

    let entries_line = if entries.is_empty() {
        "(none)".to_string()
    } else {
        entries.join(", ")
    };
    let stack_line = if stack.is_empty() {
        "(none detected)".to_string()
    } else {
        stack.join("; ")
    };
    let sdd_line = if sdd.is_empty() {
        "(none)".to_string()
    } else {
        sdd.join(", ")
    };
    let agents_line = if agents.is_empty() {
        "(none)".to_string()
    } else {
        agents.join(", ")
    };

    let arch_budget = budget_str(&config, "context/architecture.md");
    let domain_budget = budget_str(&config, "context/domain.md");
    let stack_budget = budget_str(&config, "context/stack.md");

    format!(
        "# Agnosgram bootstrap task (legacy onboarding)\n\
\n\
This repository has an Agnosgram store but its context files are still empty\n\
templates. Seed them from the code that already exists, so future sessions start\n\
with real project knowledge instead of re-discovering it every time.\n\
\n\
## What the tool already sees\n\
- Top-level entries: {entries_line}\n\
- Stack signals: {stack_line}\n\
- SDD frameworks: {sdd_line}\n\
- Agent config files: {agents_line}\n\
\n\
## Your task\n\
Explore the codebase (start from the entries above, read entry points, build\n\
files, and the largest modules) and fill in these files. Keep each within its\n\
token budget - terse and high-signal, not exhaustive:\n\
\n\
1. **context/architecture.md** (budget {arch_budget} tokens)\n\
\x20\x20\x20- Module map: top-level component -> responsibility (one line each).\n\
\x20\x20\x20- Invariants: rules that must always hold (data flow, boundaries, \"never do X\").\n\
\x20\x20\x20- Only what an agent must know before touching the code.\n\
\n\
2. **context/domain.md** (budget {domain_budget} tokens)\n\
\x20\x20\x20- Business/domain glossary: terms and rules an agent will NOT infer from code.\n\
\x20\x20\x20- Skip anything obvious from the source.\n\
\n\
3. **context/stack.md** (budget {stack_budget} tokens) if you can\n\
\x20\x20\x20determine it: exact build/test/lint commands and pinned key versions.\n\
\n\
## Rules\n\
- Ground every statement in something you actually read; do not guess. Where you\n\
\x20\x20are unsure, say so briefly rather than inventing detail.\n\
- Do not touch lessons/, decisions/, or the journal - those come from real\n\
\x20\x20sessions via `agnosgram log` and `agnosgram distill`, not from bootstrap.\n\
- Preserve each file's existing headings; replace only the placeholder bullets.\n\
\n\
## When done\n\
Run `agnosgram doctor` and resolve anything it flags (budgets, links).\n"
    )
}

pub fn run(argv: Vec<String>) -> Result<(), UserError> {
    let cfg = ArgsConfig::new(false).option("json", OptionDef::boolean(false));
    let parsed = parse_cli_args(&argv, &cfg)?;

    let root =
        find_project_root(&std::env::current_dir().map_err(|e| UserError::new(e.to_string()))?);
    if !has_store(&root) {
        return Err(UserError::new(
            "No .agnosgram/ store found. Run `agnosgram init` first.",
        ));
    }

    let prompt = build_prompt(&root);
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
    use std::fs;

    fn tmp_root(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("agnos-bootstrap-rs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn top_level_hides_ignored_entries_and_dotfiles_and_marks_directories() {
        let root = tmp_root("top-level");
        fs::write(root.join("package.json"), "{}").unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("node_modules")).unwrap();
        fs::write(root.join(".hidden"), "x").unwrap();
        let entries = top_level(&root);
        assert_eq!(
            entries,
            vec!["package.json".to_string(), "src/".to_string()]
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn budget_str_falls_back_to_na_when_unbudgeted() {
        let config = crate::core::config::default_config();
        assert_eq!(budget_str(&config, "context/architecture.md"), "1500");
        assert_eq!(budget_str(&config, "no/such/file.md"), "n/a");
    }

    #[test]
    fn build_prompt_surfaces_a_detected_stack_signal() {
        let root = tmp_root("stack");
        fs::write(root.join("Cargo.toml"), "[package]\n").unwrap();
        let prompt = build_prompt(&root);
        assert!(prompt.contains("Rust (Cargo.toml)"));
        assert!(prompt.contains("bootstrap task"));
        fs::remove_dir_all(&root).unwrap();
    }
}
