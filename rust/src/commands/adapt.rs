//! Port of `src/commands/adapt.ts`.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::adapters::{build_pointer_body, get_adapter, Adapter, SddHint, ADAPTER_KEYS};
use crate::claude_hooks::{hooks_installed, install_claude_hooks};
use crate::core::args::{parse_cli_args, ArgsConfig, OptionDef};
use crate::core::config::{load_config, save_config, AgnosgramConfig, Toggle};
use crate::core::detect::{detect_agents, detect_sdd};
use crate::core::json::Value;
use crate::core::markers::upsert_managed_block;
use crate::core::output::{info, print_json, UserError};
use crate::core::paths::{find_project_root, has_store};
use crate::core::write_file::{write_if_changed, WriteAction, WriteOpts};

pub type AdaptAction = WriteAction;

/// Another adapter key collapsed into a result because their target path is
/// a symlink resolving to the same real file.
#[derive(Clone, Debug)]
pub struct SymlinkAlias {
    pub adapter: String,
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct AdaptResult {
    pub adapter: String,
    pub path: String,
    pub action: AdaptAction,
    pub symlink_aliases: Option<Vec<SymlinkAlias>>,
}

pub fn adapt_result_to_json(r: &AdaptResult) -> Value {
    let mut o = Value::object();
    o.insert("adapter", r.adapter.clone());
    o.insert("path", r.path.clone());
    o.insert("action", r.action.as_str());
    if let Some(aliases) = &r.symlink_aliases {
        o.insert(
            "symlinkAliases",
            Value::Array(
                aliases
                    .iter()
                    .map(|a| {
                        let mut ao = Value::object();
                        ao.insert("adapter", a.adapter.clone());
                        ao.insert("path", a.path.clone());
                        ao
                    })
                    .collect(),
            ),
        );
    }
    o
}

pub fn sdd_hint_to_json(h: &SddHint) -> Value {
    let mut o = Value::object();
    o.insert("key", h.key.clone());
    o.insert("matchedPath", h.matched_path.clone());
    o
}

/// Which SDD frameworks are active for hint lines, given config + detection,
/// each paired with the specific directory that was actually found on disk.
pub fn resolve_sdd_hints(root: &Path, config: &AgnosgramConfig) -> Vec<SddHint> {
    let detected = detect_sdd(root);
    config
        .sdd
        .iter()
        .filter(|(key, toggle)| {
            *toggle == Toggle::On
                || (*toggle == Toggle::Auto && detected.iter().any(|d| d.key == key))
        })
        .map(|(key, _)| {
            let matched_path = detected
                .iter()
                .find(|d| d.key == key)
                .map(|d| d.matched_path.clone());
            SddHint {
                key: key.clone(),
                matched_path,
            }
        })
        .collect()
}

/// Resolve which path an adapter actually writes to for this project.
/// Normally `target_path`, but a legacy single-file convention takes over
/// when that path already exists as a plain file - never `mkdir` a
/// directory over an existing file.
fn resolve_adapter_path(root: &Path, adapter: &Adapter) -> String {
    if let Some(legacy) = adapter.legacy_target_path {
        if root.join(legacy).is_file() {
            return legacy.to_string();
        }
    }
    adapter.target_path.to_string()
}

/// Inject or refresh one adapter's managed block. Idempotent.
pub fn apply_adapter(
    root: &Path,
    adapter: &Adapter,
    sdd_hints: &[SddHint],
) -> Result<AdaptResult, UserError> {
    let rel_path = resolve_adapter_path(root, adapter);
    let target = root.join(&rel_path);
    let body = build_pointer_body(sdd_hints);

    let existing = if target.exists() {
        fs::read_to_string(&target).map_err(|e| UserError::new(e.to_string()))?
    } else if adapter.dedicated_file {
        adapter.preamble.unwrap_or("").to_string()
    } else {
        String::new()
    };

    let next = upsert_managed_block(&existing, &body, &rel_path)?;

    match write_if_changed(root, &rel_path, &next, WriteOpts::default()) {
        Ok(result) => Ok(AdaptResult {
            adapter: adapter.key.to_string(),
            path: result.path,
            action: result.action,
            symlink_aliases: None,
        }),
        Err(err) => Err(UserError::new(format!(
            "Could not write {}'s adapter file at {}: {}. Resolve the conflict (e.g. a file where a directory is expected) and re-run `agnosgram adapt {}`.",
            adapter.name, rel_path, err, adapter.key
        ))),
    }
}

fn is_symlink(abs: &Path) -> bool {
    fs::symlink_metadata(abs)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Lexically join and normalize `target` against `base` without touching the
/// filesystem or resolving symlinks - a local equivalent of Node's
/// `path.resolve(base, target)`.
fn lexical_join(base: &Path, target: &Path) -> PathBuf {
    let joined = if target.is_absolute() {
        target.to_path_buf()
    } else {
        base.join(target)
    };
    let mut out = PathBuf::new();
    for comp in joined.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// The real file a target path ultimately refers to, resolving through a
/// symlink even when its target doesn't exist yet (e.g. CLAUDE.md is a
/// symlink to a not-yet-created AGENTS.md) - plain canonicalization alone
/// would fail in that case.
fn resolve_real_path(root: &Path, rel_path: &str) -> PathBuf {
    let abs = root.join(rel_path);
    match fs::canonicalize(&abs) {
        Ok(real) => real,
        Err(_) => {
            if is_symlink(&abs) {
                if let Ok(link) = fs::read_link(&abs) {
                    let parent = abs.parent().unwrap_or_else(|| Path::new("/"));
                    return lexical_join(parent, &link);
                }
            }
            abs
        }
    }
}

/// Group requested adapter keys by the real file they resolve to, so a
/// symlinked pair (CLAUDE.md -> AGENTS.md, or the reverse) is written and
/// reported once instead of twice. Only collapses a group when at least one
/// member's path is an actual on-disk symlink.
fn group_by_symlink(root: &Path, keys: &[String]) -> Vec<Vec<String>> {
    let mut by_real_path: Vec<(PathBuf, Vec<String>)> = Vec::new();
    for key in keys {
        let Some(adapter) = get_adapter(key) else {
            continue;
        };
        let rel_path = resolve_adapter_path(root, adapter);
        let real = resolve_real_path(root, &rel_path);
        if let Some(entry) = by_real_path.iter_mut().find(|(p, _)| *p == real) {
            entry.1.push(key.clone());
        } else {
            by_real_path.push((real, vec![key.clone()]));
        }
    }

    let mut groups: Vec<Vec<String>> = Vec::new();
    for (_, group) in by_real_path {
        let is_alias = group.len() > 1
            && group.iter().any(|key| {
                get_adapter(key)
                    .map(|a| is_symlink(&root.join(resolve_adapter_path(root, a))))
                    .unwrap_or(false)
            });
        if is_alias {
            groups.push(group);
        } else {
            for key in group {
                groups.push(vec![key]);
            }
        }
    }
    groups
}

/// Apply one adapter, or a symlink-aliased group of adapters that all
/// resolve to the same real file - every adapter injects the same generic
/// pointer body, so writing once through the non-symlink member is
/// equivalent to writing through every alias.
fn apply_adapter_group(
    root: &Path,
    keys: &[String],
    sdd_hints: &[SddHint],
) -> Result<AdaptResult, UserError> {
    if keys.len() == 1 {
        let adapter = get_adapter(&keys[0]).expect("validated adapter key");
        return apply_adapter(root, adapter, sdd_hints);
    }

    let with_paths: Vec<(String, String)> = keys
        .iter()
        .map(|k| {
            let adapter = get_adapter(k).expect("validated adapter key");
            (k.clone(), resolve_adapter_path(root, adapter))
        })
        .collect();
    let primary = with_paths
        .iter()
        .find(|(_, p)| !is_symlink(&root.join(p)))
        .cloned()
        .unwrap_or_else(|| with_paths[0].clone());
    let aliases: Vec<(String, String)> = with_paths
        .iter()
        .filter(|(k, _)| *k != primary.0)
        .cloned()
        .collect();

    let primary_adapter = get_adapter(&primary.0).expect("validated adapter key");
    let result = apply_adapter(root, primary_adapter, sdd_hints)?;
    Ok(AdaptResult {
        symlink_aliases: Some(
            aliases
                .into_iter()
                .map(|(adapter, path)| SymlinkAlias { adapter, path })
                .collect(),
        ),
        ..result
    })
}

/// Adapters that should be written when no explicit targets are given.
pub fn resolve_enabled_adapters(root: &Path, config: &AgnosgramConfig) -> Vec<String> {
    let detected: HashSet<&str> = detect_agents(root).iter().map(|a| a.key).collect();
    ADAPTER_KEYS
        .iter()
        .filter(|key| {
            let toggle = config
                .adapters
                .iter()
                .find(|(k, _)| k == *key)
                .map(|(_, t)| *t)
                .unwrap_or(Toggle::Off);
            toggle == Toggle::On || (toggle == Toggle::Auto && detected.contains(*key))
        })
        .map(|k| k.to_string())
        .collect()
}

fn validate_targets(targets: &[String]) -> Result<(), UserError> {
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
            "Unknown adapter(s): {unknown_list}. Known: {}.",
            ADAPTER_KEYS.join(", ")
        )));
    }
    Ok(())
}

pub(crate) fn set_toggle(list: &mut Vec<(String, Toggle)>, key: &str, value: Toggle) {
    if let Some(slot) = list.iter_mut().find(|(k, _)| k == key) {
        slot.1 = value;
    } else {
        list.push((key.to_string(), value));
    }
}

pub fn run(argv: Vec<String>) -> Result<(), UserError> {
    let cfg = ArgsConfig::new(true)
        .option("all", OptionDef::boolean(false))
        .option("refresh", OptionDef::boolean(false))
        .option("json", OptionDef::boolean(false))
        .option("claude-hooks", OptionDef::boolean(false));
    let parsed = parse_cli_args(&argv, &cfg)?;

    let cwd = std::env::current_dir().map_err(|e| UserError::new(e.to_string()))?;
    let root = find_project_root(&cwd);
    if !has_store(&root) {
        return Err(UserError::new(
            "No .agnosgram/ store found. Run `agnosgram init` first.",
        ));
    }

    let mut config = load_config(&root).map_err(|e| UserError::new(e.to_string()))?;
    validate_targets(&parsed.positionals)?;

    let targets: Vec<String> = if !parsed.positionals.is_empty() {
        let targets = parsed.positionals.clone();
        for t in &targets {
            set_toggle(&mut config.adapters, t, Toggle::On);
        }
        save_config(&root, &config).map_err(|e| UserError::new(e.to_string()))?;
        targets
    } else if parsed.bool("all") {
        ADAPTER_KEYS.iter().map(|s| s.to_string()).collect()
    } else {
        resolve_enabled_adapters(&root, &config)
    };

    // --refresh also picks up hooks a prior run already installed, so an
    // upgrade (which regenerates the hook scripts' content) doesn't require
    // remembering to pass --claude-hooks again.
    let claude_hooks =
        parsed.bool("claude-hooks") || (parsed.bool("refresh") && hooks_installed(&root));

    if targets.is_empty() && !claude_hooks {
        let detected_hint = detect_agents(&root)
            .iter()
            .map(|a| a.name)
            .collect::<Vec<_>>()
            .join(", ");
        let mut msg = String::from(
            "No adapters to write. Name one explicitly (e.g. `agnosgram adapt claude`), \
             use `--all`, enable adapters in config.yml, or pass `--claude-hooks`.",
        );
        if !detected_hint.is_empty() {
            msg.push_str(&format!("\nDetected agents in this repo: {detected_hint}."));
        }
        return Err(UserError::new(msg));
    }

    let sdd_hints = resolve_sdd_hints(&root, &config);
    let groups = group_by_symlink(&root, &targets);
    let mut results: Vec<AdaptResult> = Vec::new();
    for group in &groups {
        results.push(apply_adapter_group(&root, group, &sdd_hints)?);
    }
    let hooks_result = if claude_hooks {
        Some(install_claude_hooks(&root)?)
    } else {
        None
    };

    if parsed.bool("json") {
        let mut out = Value::object();
        out.insert(
            "adapters",
            Value::Array(results.iter().map(adapt_result_to_json).collect()),
        );
        out.insert(
            "sdd",
            Value::Array(sdd_hints.iter().map(sdd_hint_to_json).collect()),
        );
        if let Some(hr) = &hooks_result {
            out.insert(
                "claudeHooks",
                Value::Array(
                    hr.written
                        .iter()
                        .map(|w| {
                            let mut o = Value::object();
                            o.insert("path", w.path.clone());
                            o.insert("action", w.action.as_str());
                            o
                        })
                        .collect(),
                ),
            );
        }
        print_json(&out);
        return Ok(());
    }

    for r in &results {
        let verb = r.action.as_str();
        if let Some(aliases) = &r.symlink_aliases {
            if !aliases.is_empty() {
                let alias_paths = aliases
                    .iter()
                    .map(|a| a.path.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                info(&format!(
                    "  {:<9} {} -> {} (symlink), managed block written once",
                    verb, alias_paths, r.path
                ));
                continue;
            }
        }
        let adapter = get_adapter(&r.adapter).expect("validated adapter key");
        info(&format!("  {:<9} {}  ({})", verb, r.path, adapter.name));
    }
    if !sdd_hints.is_empty() && !results.is_empty() {
        let summary = sdd_hints
            .iter()
            .map(|h| match &h.matched_path {
                Some(p) => format!("{} ({})", h.key, p),
                None => format!("{} (no directory detected)", h.key),
            })
            .collect::<Vec<_>>()
            .join(", ");
        info(&format!("\nSDD hints included: {summary}"));
    }
    if let Some(hr) = &hooks_result {
        info("");
        info("Claude Code hooks + skill (SessionStart runs `agnosgram pack`, Stop reminds `agnosgram log`):");
        for w in &hr.written {
            info(&format!("  {:<9} {}", w.action.as_str(), w.path));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::ADAPTERS;
    use crate::core::config::default_config;
    use std::fs;

    fn tmp_root(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("agnos-adapt-rs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn apply_adapter_creates_claude_md_and_is_idempotent() {
        let root = tmp_root("claude-idempotent");
        let adapter = get_adapter("claude").unwrap();
        let first = apply_adapter(&root, adapter, &[]).unwrap();
        assert_eq!(first.action, WriteAction::Created);
        let content_a = fs::read_to_string(root.join("CLAUDE.md")).unwrap();

        let second = apply_adapter(&root, adapter, &[]).unwrap();
        assert_eq!(second.action, WriteAction::Unchanged);
        let content_b = fs::read_to_string(root.join("CLAUDE.md")).unwrap();
        assert_eq!(content_a, content_b);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn apply_adapter_preserves_user_content_outside_the_markers() {
        let root = tmp_root("preserve-user");
        let user_text = "# House rules\n\nBe excellent to each other.\n";
        fs::write(root.join("CLAUDE.md"), user_text).unwrap();

        let adapter = get_adapter("claude").unwrap();
        apply_adapter(&root, adapter, &[]).unwrap();
        let out = fs::read_to_string(root.join("CLAUDE.md")).unwrap();
        assert!(out.starts_with("# House rules\n\nBe excellent to each other."));
        assert!(out.contains(".agnosgram/MEMORY.md"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn cursor_adapter_writes_a_dedicated_mdc_file_with_frontmatter() {
        let root = tmp_root("cursor");
        let adapter = get_adapter("cursor").unwrap();
        let res = apply_adapter(&root, adapter, &[]).unwrap();
        assert_eq!(res.action, WriteAction::Created);
        let content = fs::read_to_string(root.join(".cursor/rules/agnosgram.mdc")).unwrap();
        assert!(content.contains("alwaysApply: true"));
        assert!(content.contains(".agnosgram/MEMORY.md"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn sdd_hints_appear_in_the_pointer_body_when_passed() {
        let root = tmp_root("sdd-hints");
        let adapter = get_adapter("agents").unwrap();
        apply_adapter(
            &root,
            adapter,
            &[SddHint {
                key: "openspec".to_string(),
                matched_path: Some("openspec/".to_string()),
            }],
        )
        .unwrap();
        let out = fs::read_to_string(root.join("AGENTS.md")).unwrap();
        assert!(out.contains("openspec/"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_forced_on_sdd_hint_with_no_detected_directory_never_fabricates_a_path() {
        let root = tmp_root("sdd-forced");
        let adapter = get_adapter("agents").unwrap();
        apply_adapter(
            &root,
            adapter,
            &[SddHint {
                key: "openspec".to_string(),
                matched_path: None,
            }],
        )
        .unwrap();
        let out = fs::read_to_string(root.join("AGENTS.md")).unwrap();
        assert!(out.contains("no directory detected on disk"));
        assert!(!out.contains("openspec/"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn resolve_sdd_hints_reports_none_when_a_framework_is_forced_on_but_never_detected() {
        let root = tmp_root("resolve-sdd");
        let mut config = default_config();
        set_toggle(&mut config.sdd, "openspec", Toggle::On);
        let hints = resolve_sdd_hints(&root, &config);
        let hint = hints.iter().find(|h| h.key == "openspec").unwrap();
        assert_eq!(hint.matched_path, None);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn windsurf_cline_roo_adapters_create_a_dedicated_file_and_are_idempotent() {
        for key in ["windsurf", "cline", "roo"] {
            let root = tmp_root(&format!("dedicated-{key}"));
            let adapter = get_adapter(key).unwrap();
            let first = apply_adapter(&root, adapter, &[]).unwrap();
            assert_eq!(first.action, WriteAction::Created);
            let file = root.join(adapter.target_path);
            assert!(file.exists());
            let content_a = fs::read_to_string(&file).unwrap();
            assert!(content_a.contains(".agnosgram/MEMORY.md"));

            let second = apply_adapter(&root, adapter, &[]).unwrap();
            assert_eq!(second.action, WriteAction::Unchanged);
            assert_eq!(fs::read_to_string(&file).unwrap(), content_a);
            fs::remove_dir_all(&root).unwrap();
        }
    }

    #[test]
    fn windsurf_adapter_writes_always_on_trigger_frontmatter() {
        let root = tmp_root("windsurf-frontmatter");
        let adapter = get_adapter("windsurf").unwrap();
        apply_adapter(&root, adapter, &[]).unwrap();
        let content = fs::read_to_string(root.join(".windsurf/rules/agnosgram.md")).unwrap();
        assert!(content.starts_with("---\ntrigger: always_on\n---\n"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn cline_legacy_single_file_gets_the_managed_block_merged_in() {
        let root = tmp_root("cline-legacy");
        let user_text = "# my old cline rules\n\nBe terse.\n";
        fs::write(root.join(".clinerules"), user_text).unwrap();

        let adapter = get_adapter("cline").unwrap();
        let res = apply_adapter(&root, adapter, &[]).unwrap();
        assert_eq!(res.action, WriteAction::Updated);
        assert_eq!(res.path, ".clinerules");
        assert!(!root.join(".clinerules/agnosgram.md").exists());

        let out = fs::read_to_string(root.join(".clinerules")).unwrap();
        assert!(out.starts_with("# my old cline rules\n\nBe terse."));
        assert!(out.contains(".agnosgram/MEMORY.md"));

        let second = apply_adapter(&root, adapter, &[]).unwrap();
        assert_eq!(second.action, WriteAction::Unchanged);
        assert_eq!(second.path, ".clinerules");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn cline_no_legacy_file_still_gets_the_directory_form() {
        let root = tmp_root("cline-dir");
        let adapter = get_adapter("cline").unwrap();
        let res = apply_adapter(&root, adapter, &[]).unwrap();
        assert_eq!(res.action, WriteAction::Created);
        assert_eq!(res.path, ".clinerules/agnosgram.md");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn apply_adapter_raises_a_user_error_when_a_file_occupies_the_adapters_directory() {
        let root = tmp_root("conflict");
        fs::write(root.join(".windsurf"), "not a directory").unwrap();
        let adapter = get_adapter("windsurf").unwrap();
        assert!(apply_adapter(&root, adapter, &[]).is_err());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn resolve_enabled_adapters_reads_config_toggles() {
        let root = tmp_root("enabled");
        let mut config = default_config();
        set_toggle(&mut config.adapters, "claude", Toggle::On);
        let enabled = resolve_enabled_adapters(&root, &config);
        assert_eq!(enabled, vec!["claude".to_string()]);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn validate_targets_rejects_unknown_adapter_keys() {
        assert!(validate_targets(&["not-real".to_string()]).is_err());
        assert!(validate_targets(&["claude".to_string()]).is_ok());
    }

    #[test]
    fn adapter_keys_cover_every_registered_adapter() {
        assert_eq!(ADAPTERS.len(), ADAPTER_KEYS.len());
    }
}
