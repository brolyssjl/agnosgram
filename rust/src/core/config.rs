//! Port of `src/core/config.ts`: `.agnosgram/config.yml` load/save/defaults.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use super::paths::config_path;
use super::yaml::{parse_yaml, stringify_yaml, YamlValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    Auto,
    On,
    Off,
}

impl Toggle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Toggle::Auto => "auto",
            Toggle::On => "on",
            Toggle::Off => "off",
        }
    }

    fn parse(s: &str) -> Option<Toggle> {
        match s {
            "auto" => Some(Toggle::Auto),
            "on" => Some(Toggle::On),
            "off" => Some(Toggle::Off),
            _ => None,
        }
    }
}

/// Ordered exactly like `ADAPTER_KEYS` in `src/adapters/index.ts`, so a
/// freshly generated config's `adapters:` block matches the TS output.
pub const ADAPTER_KEYS: [&str; 6] = ["claude", "cursor", "windsurf", "cline", "roo", "agents"];

#[derive(Debug, Clone)]
pub struct AgnosgramConfig {
    pub version: i64,
    pub journal_committed: bool,
    pub staleness_days: i64,
    /// Insertion-ordered, mirroring the TS `Record<string, number>` which
    /// iterates in insertion order for string keys.
    pub budgets: Vec<(String, i64)>,
    pub adapters: Vec<(String, Toggle)>,
    pub sdd: Vec<(String, Toggle)>,
    pub pack_budget: Option<i64>,
}

pub const DEFAULT_STALENESS_DAYS: i64 = 120;

pub fn default_budgets() -> Vec<(String, i64)> {
    vec![
        ("state/status.md".to_string(), 400),
        ("context/architecture.md".to_string(), 1500),
        ("context/stack.md".to_string(), 800),
        ("context/domain.md".to_string(), 1000),
        ("lessons/pitfalls.md".to_string(), 1000),
        ("lessons/conventions.md".to_string(), 1000),
    ]
}

pub fn default_config() -> AgnosgramConfig {
    AgnosgramConfig {
        version: 1,
        journal_committed: true,
        staleness_days: DEFAULT_STALENESS_DAYS,
        budgets: default_budgets(),
        adapters: ADAPTER_KEYS
            .iter()
            .map(|k| (k.to_string(), Toggle::Off))
            .collect(),
        sdd: vec![
            ("openspec".to_string(), Toggle::Auto),
            ("speckit".to_string(), Toggle::Auto),
            ("bmad".to_string(), Toggle::Auto),
            ("agentos".to_string(), Toggle::Auto),
        ],
        pack_budget: None,
    }
}

pub fn load_config(root: &Path) -> Result<AgnosgramConfig, io::Error> {
    let text = fs::read_to_string(config_path(root))?;
    let raw = parse_yaml(&text).unwrap_or(YamlValue::Map(Vec::new()));
    Ok(normalize_config(&raw))
}

pub fn save_config(root: &Path, config: &AgnosgramConfig) -> io::Result<()> {
    fs::write(config_path(root), serialize_config(config))
}

pub fn serialize_config(config: &AgnosgramConfig) -> String {
    let header = "# Agnosgram configuration. Plain YAML, reviewable in PRs.\n\
                  # adapters/sdd toggles: auto = follow detection, on = force, off = never.\n\n";
    format!("{header}{}", stringify_yaml(&config_to_yaml(config), 0))
}

fn config_to_yaml(config: &AgnosgramConfig) -> YamlValue {
    let mut entries: Vec<(String, YamlValue)> = vec![
        ("version".to_string(), YamlValue::Int(config.version)),
        (
            "journal".to_string(),
            YamlValue::Map(vec![(
                "committed".to_string(),
                YamlValue::Bool(config.journal_committed),
            )]),
        ),
        (
            "staleness_days".to_string(),
            YamlValue::Int(config.staleness_days),
        ),
        (
            "budgets".to_string(),
            YamlValue::Map(
                config
                    .budgets
                    .iter()
                    .map(|(k, v)| (k.clone(), YamlValue::Int(*v)))
                    .collect(),
            ),
        ),
        (
            "adapters".to_string(),
            YamlValue::Map(
                config
                    .adapters
                    .iter()
                    .map(|(k, v)| (k.clone(), YamlValue::String(v.as_str().to_string())))
                    .collect(),
            ),
        ),
        (
            "sdd".to_string(),
            YamlValue::Map(
                config
                    .sdd
                    .iter()
                    .map(|(k, v)| (k.clone(), YamlValue::String(v.as_str().to_string())))
                    .collect(),
            ),
        ),
    ];
    if let Some(pb) = config.pack_budget {
        entries.push(("pack_budget".to_string(), YamlValue::Int(pb)));
    }
    YamlValue::Map(entries)
}

fn normalize_config(raw: &YamlValue) -> AgnosgramConfig {
    let mut base = default_config();
    let YamlValue::Map(obj) = raw else {
        return base;
    };
    let get = |key: &str| obj.iter().find(|(k, _)| k == key).map(|(_, v)| v);

    if let Some(YamlValue::Int(v)) = get("version") {
        base.version = *v;
    }
    if let Some(YamlValue::Int(v)) = get("staleness_days") {
        if *v > 0 {
            base.staleness_days = *v;
        }
    }
    if let Some(YamlValue::Map(journal)) = get("journal") {
        if let Some((_, YamlValue::Bool(committed))) =
            journal.iter().find(|(k, _)| k == "committed")
        {
            base.journal_committed = *committed;
        }
    }

    base.budgets = number_map(get("budgets"), &base.budgets);
    base.adapters = toggle_map(get("adapters"), &base.adapters);
    base.sdd = toggle_map(get("sdd"), &base.sdd);
    if let Some(YamlValue::Int(v)) = get("pack_budget") {
        if *v > 0 {
            base.pack_budget = Some(*v);
        }
    }
    base
}

fn number_map(raw: Option<&YamlValue>, fallback: &[(String, i64)]) -> Vec<(String, i64)> {
    let Some(YamlValue::Map(entries)) = raw else {
        return fallback.to_vec();
    };
    let out: Vec<(String, i64)> = entries
        .iter()
        .filter_map(|(k, v)| {
            if let YamlValue::Int(n) = v {
                Some((k.clone(), *n))
            } else {
                None
            }
        })
        .collect();
    if out.is_empty() {
        fallback.to_vec()
    } else {
        out
    }
}

fn toggle_map(raw: Option<&YamlValue>, fallback: &[(String, Toggle)]) -> Vec<(String, Toggle)> {
    let Some(YamlValue::Map(entries)) = raw else {
        return fallback.to_vec();
    };
    // Preserve fallback order first, then append any new keys in their
    // original YAML order - matches `{ ...fallback }` followed by
    // `Object.entries(raw)` overwrites in JS (insertion order kept for
    // existing keys, new keys appended in raw's order).
    let mut order: Vec<String> = fallback.iter().map(|(k, _)| k.clone()).collect();
    let mut out: BTreeMap<String, Toggle> = fallback.iter().map(|(k, v)| (k.clone(), *v)).collect();
    for (k, v) in entries {
        if let YamlValue::String(s) = v {
            if let Some(toggle) = Toggle::parse(s) {
                if !out.contains_key(k) {
                    order.push(k.clone());
                }
                out.insert(k.clone(), toggle);
            }
        }
    }
    order.into_iter().map(|k| (k.clone(), out[&k])).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_off_adapters_and_auto_sdd() {
        let cfg = default_config();
        assert!(cfg.adapters.iter().all(|(_, t)| *t == Toggle::Off));
        assert!(cfg.sdd.iter().all(|(_, t)| *t == Toggle::Auto));
        assert_eq!(cfg.staleness_days, DEFAULT_STALENESS_DAYS);
    }

    #[test]
    fn serialize_then_normalize_round_trips() {
        let cfg = default_config();
        let text = serialize_config(&cfg);
        let raw = parse_yaml(&text).unwrap();
        let round = normalize_config(&raw);
        assert_eq!(round.version, cfg.version);
        assert_eq!(round.staleness_days, cfg.staleness_days);
        assert_eq!(round.budgets, cfg.budgets);
        assert_eq!(round.adapters, cfg.adapters);
        assert_eq!(round.sdd, cfg.sdd);
    }

    #[test]
    fn normalize_config_falls_back_to_defaults_for_a_non_map() {
        let cfg = normalize_config(&YamlValue::Null);
        assert_eq!(cfg.version, default_config().version);
    }

    #[test]
    fn normalize_config_overrides_individual_adapter_toggles() {
        let raw = parse_yaml("adapters:\n  claude: on\n").unwrap();
        let cfg = normalize_config(&raw);
        let claude = cfg.adapters.iter().find(|(k, _)| k == "claude").unwrap().1;
        assert_eq!(claude, Toggle::On);
        let cursor = cfg.adapters.iter().find(|(k, _)| k == "cursor").unwrap().1;
        assert_eq!(cursor, Toggle::Off);
    }
}
