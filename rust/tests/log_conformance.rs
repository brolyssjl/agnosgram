//! Ported from `src/commands/log.conformance.test.ts`. See
//! `tests/CONFORMANCE_MAP.md`.
mod common;
use common::{current_journal_month, init_store, run_cli, TempDir};
use std::fs;

fn setup() -> TempDir {
    let root = TempDir::new("agnos-log");
    init_store(root.path());
    root
}

#[test]
fn log_appends_an_entry_to_the_current_months_journal() {
    let root = setup();
    let res = run_cli(
        &["log", "--did", "wired the CLI", "--learned", "parseArgs is enough", "--json"],
        root.path(),
    );
    assert_eq!(res.status, 0);
    let month = current_journal_month(root.path());
    let journal =
        fs::read_to_string(root.path().join(".agnosgram").join("journal").join(format!("{month}.md"))).unwrap();
    assert!(journal.contains("- **Did:** wired the CLI"));
    assert!(journal.contains("- **Learned:** parseArgs is enough"));
}

#[test]
fn log_with_no_slots_throws_a_helpful_error() {
    let root = setup();
    let res = run_cli(&["log"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("Nothing to log"));
}

#[test]
fn fri_001_log_learned_dash_x_stores_the_literal_value_instead_of_crashing() {
    let root = setup();
    let res = run_cli(&["log", "--learned", "--x", "--json"], root.path());
    assert_eq!(res.status, 0);
    let month = current_journal_month(root.path());
    let journal =
        fs::read_to_string(root.path().join(".agnosgram").join("journal").join(format!("{month}.md"))).unwrap();
    assert!(journal.contains("- **Learned:** --x"));
}
