//! Port of `src/core/detect.ts`: advisory-only SDD framework and agent
//! detection. Nothing here is load-bearing - a missing or extra detection can
//! only change hint lines, never corrupt memory.

use std::path::Path;

pub struct SddFramework {
    pub key: &'static str,
    pub name: &'static str,
    /// Path fragments (relative to root); presence of any one counts as detected.
    pub markers: &'static [&'static str],
    /// Tail text describing coexistence, given the matched directory
    /// (`None` when forced `on` without ever being detected on disk).
    pub hint_tail: &'static str,
}

impl SddFramework {
    /// "X is present (`path`): tail" or, with no real path, "X support is
    /// enabled (no directory detected on disk): tail".
    pub fn hint(&self, matched_path: Option<&str>) -> String {
        match matched_path {
            Some(path) => format!(
                "- {} is present (`{}`): {}",
                self.name, path, self.hint_tail
            ),
            None => format!(
                "- {} support is enabled (no directory detected on disk): {}",
                self.name, self.hint_tail
            ),
        }
    }
}

pub const SDD_FRAMEWORKS: [SddFramework; 4] = [
    SddFramework {
        key: "openspec",
        name: "OpenSpec",
        markers: &["openspec"],
        hint_tail: "specs live there; memory records *why* and *what failed*, linking to specs by path.",
    },
    SddFramework {
        key: "speckit",
        name: "Spec Kit",
        markers: &[".specify"],
        hint_tail: "the constitution stays authoritative for principles; Agnosgram holds empirical lessons.",
    },
    SddFramework {
        key: "bmad",
        name: "BMAD",
        markers: &["_bmad", ".bmad-core", "_bmad-core"],
        hint_tail: "QA/review steps should read `.agnosgram/lessons/pitfalls.md`; retro output goes to the journal.",
    },
    SddFramework {
        key: "agentos",
        name: "Agent OS",
        markers: &["agent-os", ".agent-os"],
        hint_tail: "`standards/` stays authoritative for style; Agnosgram holds project-local exceptions and history.",
    },
];

pub struct AgentTarget {
    pub key: &'static str,
    pub name: &'static str,
    pub signals: &'static [&'static str],
}

pub const AGENT_TARGETS: [AgentTarget; 6] = [
    AgentTarget {
        key: "claude",
        name: "Claude Code",
        signals: &["CLAUDE.md", ".claude"],
    },
    AgentTarget {
        key: "cursor",
        name: "Cursor",
        signals: &[".cursor"],
    },
    AgentTarget {
        key: "windsurf",
        name: "Windsurf",
        signals: &[".windsurf", ".windsurfrules"],
    },
    AgentTarget {
        key: "cline",
        name: "Cline",
        signals: &[".clinerules"],
    },
    AgentTarget {
        key: "roo",
        name: "Roo Code",
        signals: &[".roo", ".roorules"],
    },
    AgentTarget {
        key: "agents",
        name: "AGENTS.md (Codex / OpenCode / generic)",
        signals: &["AGENTS.md", ".codex", ".opencode", "opencode.json"],
    },
];

/// An SDD framework found on disk, plus which marker matched.
pub struct SddDetection {
    pub key: &'static str,
    /// Only ever carries a trailing slash when it's a real, confirmed
    /// directory.
    pub matched_path: String,
}

pub fn detect_sdd(root: &Path) -> Vec<SddDetection> {
    let mut out = Vec::new();
    for f in &SDD_FRAMEWORKS {
        let Some(hit) = f.markers.iter().find(|m| root.join(m).exists()) else {
            continue;
        };
        let is_directory = root.join(hit).is_dir();
        let matched_path = if is_directory {
            format!("{hit}/")
        } else {
            hit.to_string()
        };
        out.push(SddDetection {
            key: f.key,
            matched_path,
        });
    }
    out
}

pub fn detect_agents(root: &Path) -> Vec<&'static AgentTarget> {
    AGENT_TARGETS
        .iter()
        .filter(|a| a.signals.iter().any(|s| root.join(s).exists()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("agnos-detect-rs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn detect_sdd_reports_a_trailing_slash_when_the_marker_is_a_real_directory() {
        let root = tmp_dir("dir");
        fs::create_dir_all(root.join("openspec")).unwrap();
        let hits = detect_sdd(&root);
        assert_eq!(hits[0].key, "openspec");
        assert_eq!(hits[0].matched_path, "openspec/");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn detect_sdd_never_claims_a_directory_when_the_marker_is_a_plain_file() {
        let root = tmp_dir("file");
        fs::write(root.join("openspec"), "not actually a directory").unwrap();
        let hits = detect_sdd(&root);
        assert_eq!(hits[0].key, "openspec");
        assert_eq!(hits[0].matched_path, "openspec");
        assert!(!hits[0].matched_path.ends_with('/'));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn no_markers_present_means_no_detections() {
        let root = tmp_dir("none");
        assert!(detect_sdd(&root).is_empty());
        fs::remove_dir_all(&root).unwrap();
    }
}
