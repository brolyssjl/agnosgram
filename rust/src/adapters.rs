//! Port of `src/adapters/index.ts`: one adapter per agent config file, every
//! adapter injecting the same shared pointer body.

use crate::core::detect::SDD_FRAMEWORKS;

/// An SDD framework to hint at, plus the specific directory to point the
/// agent to. `matched_path` is `None` when the framework was forced `on` in
/// `config.yml` without ever being detected on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SddHint {
    pub key: String,
    pub matched_path: Option<String>,
}

/// An adapter targets one agent's config file. Every adapter injects the
/// *same* ~10-line pointer body. Shared files (CLAUDE.md, AGENTS.md) get the
/// managed block merged into user content; a dedicated file (Cursor `.mdc`)
/// is fully ours.
pub struct Adapter {
    pub key: &'static str,
    pub name: &'static str,
    /// Target path relative to the project root.
    pub target_path: &'static str,
    /// True when the whole file belongs to Agnosgram (dedicated rule file).
    pub dedicated_file: bool,
    /// Content written above the managed block when creating a dedicated file.
    pub preamble: Option<&'static str>,
    /// A legacy single-file convention this tool also reads (e.g. Cline's
    /// older `.clinerules` file, before the `.clinerules/` directory
    /// convention). If this path already exists as a plain file when
    /// applying, write there instead of `target_path`.
    pub legacy_target_path: Option<&'static str>,
}

pub const ADAPTERS: [Adapter; 6] = [
    Adapter {
        key: "claude",
        name: "Claude Code",
        target_path: "CLAUDE.md",
        dedicated_file: false,
        preamble: None,
        legacy_target_path: None,
    },
    Adapter {
        key: "cursor",
        name: "Cursor",
        target_path: ".cursor/rules/agnosgram.mdc",
        dedicated_file: true,
        preamble: Some(
            "---\ndescription: Project memory protocol (Agnosgram)\nalwaysApply: true\n---\n",
        ),
        legacy_target_path: None,
    },
    Adapter {
        key: "windsurf",
        name: "Windsurf",
        target_path: ".windsurf/rules/agnosgram.md",
        dedicated_file: true,
        preamble: Some("---\ntrigger: always_on\n---\n"),
        legacy_target_path: None,
    },
    Adapter {
        key: "cline",
        name: "Cline",
        target_path: ".clinerules/agnosgram.md",
        dedicated_file: true,
        preamble: None,
        legacy_target_path: Some(".clinerules"),
    },
    Adapter {
        key: "roo",
        name: "Roo Code",
        target_path: ".roo/rules/agnosgram.md",
        dedicated_file: true,
        preamble: None,
        legacy_target_path: None,
    },
    Adapter {
        key: "agents",
        name: "AGENTS.md",
        target_path: "AGENTS.md",
        dedicated_file: false,
        preamble: None,
        legacy_target_path: None,
    },
];

/// Same order as `ADAPTERS` - mirrors `Object.keys(ADAPTERS)` in the TS
/// source, which iterates in insertion order.
pub const ADAPTER_KEYS: [&str; 6] = ["claude", "cursor", "windsurf", "cline", "roo", "agents"];

pub fn get_adapter(key: &str) -> Option<&'static Adapter> {
    ADAPTERS.iter().find(|a| a.key == key)
}

/// Hint lines describing coexistence with each detected SDD framework,
/// pointing at the actual matched directory.
fn sdd_hint_lines(hints: &[SddHint]) -> Vec<String> {
    hints
        .iter()
        .filter_map(|h| {
            SDD_FRAMEWORKS
                .iter()
                .find(|f| f.key == h.key)
                .map(|f| f.hint(h.matched_path.as_deref()))
        })
        .collect()
}

/// The shared pointer body injected into every adapter target.
pub fn build_pointer_body(sdd_hints: &[SddHint]) -> String {
    let mut lines: Vec<String> = vec![
        "## Project memory (Agnosgram)".to_string(),
        String::new(),
        "This project keeps durable, agent-agnostic memory in `.agnosgram/` (plain".to_string(),
        "Markdown, reviewed in PRs). **Before doing any work:**".to_string(),
        String::new(),
        "1. Read `.agnosgram/MEMORY.md` and follow its reading protocol.".to_string(),
        "2. Always load `.agnosgram/state/status.md` and `.agnosgram/lessons/*`.".to_string(),
        "3. Read `.agnosgram/context/*` and `.agnosgram/decisions/` only for the".to_string(),
        "   areas you are about to touch.".to_string(),
        "4. Before ending the session, record what happened with `agnosgram log`.".to_string(),
    ];
    let hint_lines = sdd_hint_lines(sdd_hints);
    if !hint_lines.is_empty() {
        lines.push(String::new());
        lines.push("Coexisting tools detected:".to_string());
        lines.extend(hint_lines);
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sdd_framework_produces_a_hint_line_with_a_real_matched_path() {
        for framework in SDD_FRAMEWORKS.iter() {
            let body = build_pointer_body(&[SddHint {
                key: framework.key.to_string(),
                matched_path: Some(format!("{}/", framework.key)),
            }]);
            assert!(
                body.contains(&format!("{}/", framework.key)),
                "expected a hint line for {} pointing at its matched path",
                framework.key
            );
            assert!(body.contains(framework.name));
        }
    }

    #[test]
    fn every_sdd_framework_produces_a_hint_line_with_no_matched_path() {
        for framework in SDD_FRAMEWORKS.iter() {
            let body = build_pointer_body(&[SddHint {
                key: framework.key.to_string(),
                matched_path: None,
            }]);
            assert!(body.contains(framework.name));
            assert!(body.contains("no directory detected on disk"));
            assert!(!body.contains(&format!("{}/", framework.key)));
        }
    }

    #[test]
    fn an_unknown_sdd_key_produces_no_hint_line_instead_of_crashing() {
        let body = build_pointer_body(&[SddHint {
            key: "not-a-real-framework".to_string(),
            matched_path: Some("whatever/".to_string()),
        }]);
        assert!(!body.contains("Coexisting tools detected"));
    }

    #[test]
    fn adapter_keys_matches_adapters_order() {
        let keys: Vec<&str> = ADAPTERS.iter().map(|a| a.key).collect();
        assert_eq!(keys, ADAPTER_KEYS.to_vec());
    }
}
