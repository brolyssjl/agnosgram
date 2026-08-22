//! Ported from `src/commands/init.conformance.test.ts`. See
//! `tests/CONFORMANCE_MAP.md`.
mod common;
use common::{current_journal_month, has_exact_line, run_cli, TempDir};
use std::fs;

const SCAFFOLD: &[&str] = &[
    "MEMORY.md",
    "config.yml",
    "state/status.md",
    "context/architecture.md",
    "context/stack.md",
    "context/domain.md",
    "decisions/README.md",
    "lessons/pitfalls.md",
    "lessons/conventions.md",
];

#[test]
fn init_scaffolds_the_full_store() {
    let root = TempDir::new("agnos-init");
    let res = run_cli(&["init", "--json"], root.path());
    assert_eq!(res.status, 0);
    for rel in SCAFFOLD {
        assert!(root.path().join(".agnosgram").join(rel).exists(), "missing {rel}");
    }
    let month = current_journal_month(root.path());
    assert!(root
        .path()
        .join(".agnosgram")
        .join("journal")
        .join(format!("{month}.md"))
        .exists());
}

#[test]
fn init_refuses_to_overwrite_without_force() {
    let root = TempDir::new("agnos-init");
    assert_eq!(run_cli(&["init", "--adapt", "none"], root.path()).status, 0);
    let second = run_cli(&["init", "--adapt", "none"], root.path());
    assert_ne!(second.status, 0);
    assert!(second.stderr.contains("already exists"));
}

#[test]
fn init_auto_adapts_detected_agents() {
    let root = TempDir::new("agnos-init");
    fs::write(root.path().join("CLAUDE.md"), "# existing\n").unwrap();
    assert_eq!(run_cli(&["init"], root.path()).status, 0);
    let claude = fs::read_to_string(root.path().join("CLAUDE.md")).unwrap();
    assert!(claude.contains(".agnosgram/MEMORY.md"));
    let config = fs::read_to_string(root.path().join(".agnosgram").join("config.yml")).unwrap();
    assert!(has_exact_line(&config, "claude: on"));
}

#[test]
fn init_adapt_none_writes_no_adapters() {
    let root = TempDir::new("agnos-init");
    fs::write(root.path().join("AGENTS.md"), "# existing\n").unwrap();
    assert_eq!(run_cli(&["init", "--adapt", "none"], root.path()).status, 0);
    let agents = fs::read_to_string(root.path().join("AGENTS.md")).unwrap();
    assert!(!agents.contains(".agnosgram/MEMORY.md"));
}

#[test]
fn init_does_not_scaffold_meta_it_is_opt_in_via_feedback_on_first_use() {
    let root = TempDir::new("agnos-init");
    assert_eq!(run_cli(&["init", "--adapt", "none"], root.path()).status, 0);
    assert!(!root.path().join(".agnosgram").join("meta").exists());
}

#[test]
fn init_records_sdd_detection_into_config_driven_hints() {
    let root = TempDir::new("agnos-init");
    fs::create_dir_all(root.path().join("openspec")).unwrap();
    fs::write(root.path().join("AGENTS.md"), "").unwrap();
    assert_eq!(run_cli(&["init"], root.path()).status, 0);
    let agents = fs::read_to_string(root.path().join("AGENTS.md")).unwrap();
    assert!(agents.contains("openspec/"));
}

#[test]
fn fri_001_init_adapt_none_gives_the_existing_validation_error_not_a_raw_parseargs_crash() {
    let root = TempDir::new("agnos-init");
    let res = run_cli(&["init", "--adapt", "-none"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("Unknown adapter(s) in --adapt: -none"));
}
