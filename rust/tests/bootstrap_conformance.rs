//! Ported from `src/commands/bootstrap.conformance.test.ts`. See
//! `tests/CONFORMANCE_MAP.md`.
mod common;
use common::{run_cli, TempDir};
use std::fs;

fn setup() -> TempDir {
    let root = TempDir::new("agnos-bootstrap");
    fs::write(root.path().join("package.json"), "{ \"name\": \"demo\" }\n").unwrap();
    let res = run_cli(&["init", "--adapt", "none"], root.path());
    assert_eq!(res.status, 0);
    root
}

#[test]
fn bootstrap_emits_a_prompt_targeting_context_files_and_detected_stack() {
    let root = setup();
    let res = run_cli(&["bootstrap"], root.path());
    assert_eq!(res.status, 0);
    assert!(res.stdout.contains("bootstrap task"));
    assert!(res.stdout.contains("context/architecture.md"));
    assert!(res.stdout.contains("context/domain.md"));
    assert!(res.stdout.contains("package.json"));
    assert!(res.stdout.contains("Node"));
}

#[test]
fn an_unknown_flag_gives_a_clean_usererror_not_a_raw_parseargs_crash() {
    let root = setup();
    let res = run_cli(&["bootstrap", "--nope"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("Unknown option"));
}
