//! Ported from `src/commands/distill.conformance.test.ts`. See
//! `tests/CONFORMANCE_MAP.md`.
mod common;
use common::{current_journal_month, init_store, run_cli, write_store_file, TempDir};

fn setup() -> TempDir {
    let root = TempDir::new("agnos-distill");
    init_store(root.path());
    root
}

#[test]
fn distill_emits_a_compaction_prompt_naming_the_schema_and_rules() {
    let root = setup();
    let res = run_cli(&["distill"], root.path());
    assert_eq!(res.status, 0);
    assert!(res.stdout.contains("distillation task"));
    assert!(res.stdout.contains("supersedes"));
    assert!(res.stdout.contains("Merge, do not append"));
    assert!(res.stdout.contains("journal/"));
}

#[test]
fn distill_validate_passes_a_well_formed_file() {
    let root = setup();
    write_store_file(
        root.path(),
        "lessons/pitfalls.md",
        "# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nA valid lesson.\n",
    );
    let res = run_cli(
        &["distill", "--validate", "lessons/pitfalls.md"],
        root.path(),
    );
    assert!(res.stdout.contains("valid"));
    assert_ne!(res.status, 1);
}

#[test]
fn distill_validate_fails_a_schema_broken_file_with_a_non_zero_exit() {
    let root = setup();
    write_store_file(
        root.path(),
        "lessons/pitfalls.md",
        "# Pitfalls\n\n---\nid: bad\ntype: pitfall\n---\nbroken\n",
    );
    let res = run_cli(
        &["distill", "--validate", "lessons/pitfalls.md"],
        root.path(),
    );
    assert_eq!(res.status, 1);
    assert!(res.stdout.contains("error"));
}

#[test]
fn distill_archive_moves_a_journal_month_into_archive() {
    let root = setup();
    let month = current_journal_month(root.path());
    let res = run_cli(&["distill", "--archive", &month], root.path());
    assert_eq!(res.status, 0);
    assert!(!root
        .path()
        .join(".agnosgram/journal")
        .join(format!("{month}.md"))
        .exists());
    assert!(root
        .path()
        .join(".agnosgram/journal/archive")
        .join(format!("{month}.md"))
        .exists());
}

#[test]
fn distill_archive_rejects_a_bad_month_argument() {
    let root = setup();
    let res = run_cli(&["distill", "--archive", "nope"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("YYYY-MM"));
}

#[test]
fn distill_archive_dash_2026_08_gives_the_existing_validation_error_not_a_raw_parseargs_crash() {
    let root = setup();
    let res = run_cli(&["distill", "--archive", "-2026-08"], root.path());
    assert_ne!(res.status, 0);
    assert!(res
        .stderr
        .contains("--archive expects a YYYY-MM month, got \"-2026-08\""));
}

#[test]
fn distill_validate_fails_frontmatter_holding_a_block_scalar() {
    let root = setup();
    write_store_file(
        root.path(),
        "lessons/pitfalls.md",
        "# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: |\n  journal/2026-07.md\n  plus trailing junk\n---\nBody.\n",
    );
    let res = run_cli(
        &["distill", "--validate", "lessons/pitfalls.md"],
        root.path(),
    );
    assert_eq!(res.status, 1);
    assert!(
        res.stdout.contains("frontmatter.parse") || res.stdout.contains("block scalars"),
        "{}",
        res.stdout
    );
}
