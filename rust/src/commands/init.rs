//! Port of `src/commands/init.ts`.

use std::fs;
use std::path::Path;

use crate::adapters::{get_adapter, ADAPTER_KEYS};
use crate::commands::adapt::{
    adapt_result_to_json, apply_adapter, resolve_sdd_hints, set_toggle, AdaptResult,
};
use crate::core::args::{parse_cli_args, ArgsConfig, OptionDef};
use crate::core::config::{default_config, save_config, Toggle};
use crate::core::detect::{detect_agents, detect_sdd, SDD_FRAMEWORKS};
use crate::core::json::Value;
use crate::core::output::{info, print_json, UserError};
use crate::core::paths::{has_store, memory_dir};
use crate::core::templates::{
    architecture_md, conventions_md, decisions_readme, domain_md, iso_date, journal_md,
    journal_month_now, memory_md, pitfalls_md, stack_md, status_md,
};

fn scaffold(root: &Path) -> Result<(), UserError> {
    let dir = memory_dir(root);
    let date = iso_date();
    let month = journal_month_now();

    for sub in ["state", "context", "decisions", "lessons", "journal"] {
        fs::create_dir_all(dir.join(sub)).map_err(|e| UserError::new(e.to_string()))?;
    }

    let files: Vec<(String, String)> = vec![
        ("MEMORY.md".to_string(), memory_md(&date)),
        ("state/status.md".to_string(), status_md(&date)),
        ("context/architecture.md".to_string(), architecture_md()),
        ("context/stack.md".to_string(), stack_md()),
        ("context/domain.md".to_string(), domain_md()),
        ("decisions/README.md".to_string(), decisions_readme()),
        ("lessons/pitfalls.md".to_string(), pitfalls_md()),
        ("lessons/conventions.md".to_string(), conventions_md()),
        (format!("journal/{month}.md"), journal_md(&month)),
    ];
    for (rel, content) in files {
        fs::write(dir.join(rel), content).map_err(|e| UserError::new(e.to_string()))?;
    }
    Ok(())
}

/// Parse `--adapt` into an explicit target list, `Some(vec![])` for "none",
/// or `None` for default (auto-detect).
fn parse_adapt_option(raw: Option<&str>) -> Result<Option<Vec<String>>, UserError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let value = raw.trim().to_lowercase();
    if value == "none" {
        return Ok(Some(Vec::new()));
    }
    let targets: Vec<String> = value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let unknown: Vec<&String> = targets
        .iter()
        .filter(|t| get_adapter(t).is_none())
        .collect();
    if !unknown.is_empty() {
        let unknown_list = unknown
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(UserError::new(format!(
            "Unknown adapter(s) in --adapt: {unknown_list}. Known: {}.",
            ADAPTER_KEYS.join(", ")
        )));
    }
    Ok(Some(targets))
}

pub fn run(argv: Vec<String>) -> Result<(), UserError> {
    let cfg = ArgsConfig::new(false)
        .option("force", OptionDef::boolean(false))
        .option("adapt", OptionDef::string())
        .option("json", OptionDef::boolean(false))
        .option("no-journal-commit", OptionDef::boolean(false));
    let parsed = parse_cli_args(&argv, &cfg)?;

    let root = std::env::current_dir().map_err(|e| UserError::new(e.to_string()))?;
    if has_store(&root) && !parsed.bool("force") {
        return Err(UserError::new(format!(
            "{}/ already exists at {}. Use --force to re-scaffold missing files, or edit the store directly.",
            crate::core::paths::MEMORY_DIR,
            root.display()
        )));
    }

    scaffold(&root)?;

    let detected_sdd = detect_sdd(&root);
    let detected_agents = detect_agents(&root);

    let mut config = default_config();
    if parsed.bool("no-journal-commit") {
        config.journal_committed = false;
    }

    // Decide which adapters to write. Default: every detected agent.
    // `--adapt` overrides.
    let explicit = parse_adapt_option(parsed.str("adapt"))?;
    let adapt_targets: Vec<String> = match explicit {
        Some(v) => v,
        None => detected_agents
            .iter()
            .map(|a| a.key.to_string())
            .filter(|k| get_adapter(k).is_some())
            .collect(),
    };
    for key in &adapt_targets {
        set_toggle(&mut config.adapters, key, Toggle::On);
    }

    save_config(&root, &config).map_err(|e| UserError::new(e.to_string()))?;

    let sdd_hints = resolve_sdd_hints(&root, &config);
    let mut adapt_results: Vec<AdaptResult> = Vec::new();
    for key in &adapt_targets {
        let adapter = get_adapter(key).expect("validated adapter key");
        adapt_results.push(apply_adapter(&root, adapter, &sdd_hints)?);
    }

    if parsed.bool("json") {
        let mut out = Value::object();
        out.insert("root", root.display().to_string());
        out.insert("created", format!("{}/.agnosgram", root.display()));
        out.insert(
            "detectedSdd",
            Value::Array(
                detected_sdd
                    .iter()
                    .map(|f| {
                        let mut o = Value::object();
                        o.insert("key", f.key);
                        o.insert("matchedPath", f.matched_path.clone());
                        o
                    })
                    .collect(),
            ),
        );
        out.insert(
            "detectedAgents",
            Value::Array(detected_agents.iter().map(|a| Value::from(a.key)).collect()),
        );
        out.insert(
            "adapters",
            Value::Array(adapt_results.iter().map(adapt_result_to_json).collect()),
        );
        print_json(&out);
        return Ok(());
    }

    info(&format!(
        "Initialized Agnosgram memory in {}/.agnosgram",
        root.display()
    ));
    info("");
    info("  Scaffolded: MEMORY.md, config.yml, state/, context/, decisions/, lessons/, journal/");
    if !detected_sdd.is_empty() {
        let names: Vec<String> = detected_sdd
            .iter()
            .map(|f| {
                let name = SDD_FRAMEWORKS
                    .iter()
                    .find(|fr| fr.key == f.key)
                    .map(|fr| fr.name)
                    .unwrap_or(f.key);
                format!("{} ({})", name, f.matched_path)
            })
            .collect();
        info(&format!("  Detected SDD: {}", names.join(", ")));
        info("  Agnosgram will not touch these files - adapter hints will point agents at them.");
    }
    if !detected_agents.is_empty() {
        let names: Vec<&str> = detected_agents.iter().map(|a| a.name).collect();
        info(&format!("  Detected agents: {}", names.join(", ")));
    }
    if !adapt_results.is_empty() {
        info("");
        info("  Adapters written:");
        for r in &adapt_results {
            let adapter = get_adapter(&r.adapter).expect("validated adapter key");
            info(&format!(
                "    {:<9} {}  ({})",
                r.action.as_str(),
                r.path,
                adapter.name
            ));
        }
    } else {
        info("");
        info(&format!(
            "  No adapters written. Add one with `agnosgram adapt {}`.",
            ADAPTER_KEYS.join("|")
        ));
    }
    info("");
    info("Next: fill in state/status.md and context/*, then commit .agnosgram/ to the repo.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_root(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("agnos-init-rs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn scaffold_writes_every_expected_file() {
        let root = tmp_root("scaffold");
        scaffold(&root).unwrap();
        let dir = memory_dir(&root);
        for rel in [
            "MEMORY.md",
            "state/status.md",
            "context/architecture.md",
            "context/stack.md",
            "context/domain.md",
            "decisions/README.md",
            "lessons/pitfalls.md",
            "lessons/conventions.md",
        ] {
            assert!(dir.join(rel).exists(), "missing {rel}");
        }
        let month = journal_month_now();
        assert!(dir.join(format!("journal/{month}.md")).exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn parse_adapt_option_none_gives_an_empty_explicit_list() {
        assert_eq!(parse_adapt_option(Some("none")).unwrap(), Some(Vec::new()));
    }

    #[test]
    fn parse_adapt_option_absent_gives_none() {
        assert_eq!(parse_adapt_option(None).unwrap(), None);
    }

    #[test]
    fn parse_adapt_option_parses_a_comma_list_case_insensitively() {
        let parsed = parse_adapt_option(Some(" Claude, Cursor ")).unwrap();
        assert_eq!(
            parsed,
            Some(vec!["claude".to_string(), "cursor".to_string()])
        );
    }

    #[test]
    fn parse_adapt_option_rejects_unknown_adapters() {
        let err = parse_adapt_option(Some("not-real")).unwrap_err();
        assert!(err.0.contains("Unknown adapter(s) in --adapt: not-real"));
    }
}
