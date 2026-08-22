//! Ported from `src/commands/feedback.conformance.test.ts`. See
//! `tests/CONFORMANCE_MAP.md`.
mod common;
use common::{init_store, run_cli, run_cli_stdin, Json, TempDir};
use std::fs;

fn setup() -> TempDir {
    let root = TempDir::new("agnos-feedback");
    init_store(root.path());
    root
}

fn friction_path(root: &TempDir) -> std::path::PathBuf {
    root.path().join(".agnosgram/meta/friction.md")
}

#[test]
fn feedback_creates_meta_friction_md_on_first_use_not_before() {
    let root = setup();
    assert!(!friction_path(&root).exists());
    let res = run_cli(&["feedback", "doctor's error message was confusing"], root.path());
    assert_eq!(res.status, 0);
    assert!(friction_path(&root).exists());
    let text = fs::read_to_string(friction_path(&root)).unwrap();
    assert!(text.contains("id: FRI-001"));
    assert!(text.contains("type: friction"));
    assert!(text.contains("doctor's error message was confusing"));
}

#[test]
fn feedback_allocates_sequential_fri_ids() {
    let root = setup();
    run_cli(&["feedback", "one"], root.path());
    run_cli(&["feedback", "two"], root.path());
    let text = fs::read_to_string(friction_path(&root)).unwrap();
    assert!(text.contains("id: FRI-001"));
    assert!(text.contains("id: FRI-002"));
}

#[test]
fn feedback_defaults_scope_to_cli_and_confidence_to_medium() {
    let root = setup();
    run_cli(&["feedback", "default scope test"], root.path());
    let text = fs::read_to_string(friction_path(&root)).unwrap();
    assert!(text.contains("scope: [cli]"));
    assert!(text.contains("confidence: medium"));
}

#[test]
fn feedback_scope_and_confidence_override_the_defaults() {
    let root = setup();
    run_cli(&["feedback", "custom", "--scope", "docs,ux", "--confidence", "high"], root.path());
    let text = fs::read_to_string(friction_path(&root)).unwrap();
    assert!(text.contains("scope: [docs, ux]"));
    assert!(text.contains("confidence: high"));
}

#[test]
fn feedback_rejects_an_unknown_confidence() {
    let root = setup();
    let res = run_cli(&["feedback", "x", "--confidence", "bogus"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("--confidence must be one of"));
}

#[test]
fn feedback_rejects_a_scope_tag_that_would_break_the_yaml_flow_sequence_bracket() {
    let root = setup();
    let res = run_cli(&["feedback", "x", "--scope", "cli]x"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("--scope tag \"cli]x\" must contain only"));
}

#[test]
fn feedback_rejects_a_scope_tag_containing_a_colon_space() {
    let root = setup();
    let res = run_cli(&["feedback", "x", "--scope", "a: b"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("--scope tag \"a: b\" must contain only"));
}

#[test]
fn feedback_with_no_text_throws_a_usage_error() {
    let root = setup();
    let res = run_cli(&["feedback"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("Usage: agnosgram feedback"));
}

#[test]
fn feedback_stdin_reads_the_entry_text_from_piped_input() {
    let root = setup();
    let res = run_cli_stdin(
        &["feedback", "--stdin"],
        root.path(),
        Some("captured over stdin, longer than a single positional arg\n"),
    );
    assert_eq!(res.status, 0);
    let text = fs::read_to_string(friction_path(&root)).unwrap();
    assert!(text.contains("captured over stdin, longer than a single positional arg"));
}

#[test]
fn feedback_json_prints_a_structured_envelope_and_no_gh_command_by_default() {
    let root = setup();
    let res = run_cli(&["feedback", "json output test", "--json"], root.path());
    let parsed = Json::parse(&res.stdout);
    assert_eq!(parsed.get("id").and_then(Json::as_str), Some("FRI-001"));
    assert_eq!(parsed.get("text").and_then(Json::as_str), Some("json output test"));
    assert!(parsed.get("share").map(Json::is_null).unwrap_or(false));
}

#[test]
fn feedback_share_prints_a_ready_to_run_gh_issue_create_command_targeting_the_real_repo_but_never_runs_it() {
    let root = setup();
    let res = run_cli(&["feedback", "share me", "--share"], root.path());
    assert!(res.stdout.contains("gh issue create"));
    assert!(
        res.stdout.contains("--repo brolyssjl/agnosgram"),
        "must pin --repo, or it files on whatever repo the CLI runs in"
    );
    assert!(res.stdout.contains("never executes it") || res.stdout.contains("run it yourself"));
}

#[test]
fn feedback_share_json_includes_the_command_targeting_the_real_repo_as_a_string_field() {
    let root = setup();
    let res = run_cli(&["feedback", "share me", "--share", "--json"], root.path());
    let parsed = Json::parse(&res.stdout);
    let share = parsed.get("share").and_then(Json::as_str).expect("share must be a string");
    assert!(share.starts_with("gh issue create"));
    assert!(share.contains("--repo brolyssjl/agnosgram"));
}

#[test]
fn feedback_writes_only_under_agnosgram_meta_touching_no_other_file() {
    let root = setup();
    let status_path = root.path().join(".agnosgram/state/status.md");
    let before = fs::read_to_string(&status_path).unwrap();
    let before_roadmap_exists = root.path().join("ROADMAP.md").exists();
    run_cli(&["feedback", "isolation check"], root.path());
    let after = fs::read_to_string(&status_path).unwrap();
    assert_eq!(before, after);
    assert_eq!(root.path().join("ROADMAP.md").exists(), before_roadmap_exists);
}

#[test]
fn fri_001_feedback_scope_dash_docs_accepts_a_dash_leading_but_otherwise_valid_tag_not_a_crash() {
    let root = setup();
    let res = run_cli(&["feedback", "some text", "--scope", "-docs"], root.path());
    assert_eq!(res.status, 0);
    let text = fs::read_to_string(friction_path(&root)).unwrap();
    assert!(text.contains("scope: [-docs]"));
}
