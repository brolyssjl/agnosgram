//! Ported from `src/commands/show.conformance.test.ts`. See
//! `tests/CONFORMANCE_MAP.md`.
mod common;
use common::{init_store, run_cli, write_store_file, Json, TempDir};

fn setup() -> TempDir {
    let root = TempDir::new("agnos-show");
    init_store(root.path());
    write_store_file(
        root.path(),
        "lessons/pitfalls.md",
        "# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [core, tooling]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nDo not use require().\n",
    );
    write_store_file(
        root.path(),
        "lessons/conventions.md",
        "# Conventions\n\n---\nid: CON-001\ntype: convention\nscope: [tooling]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nAlways use node built-ins.\n",
    );
    root
}

#[test]
fn show_matches_an_exact_record_id_first() {
    let root = setup();
    let res = run_cli(&["show", "LES-001"], root.path());
    assert!(res.stdout.contains("id: LES-001"));
    assert!(res.stdout.contains("Do not use require()."));
    assert!(!res.stdout.contains("CON-001"));
}

#[test]
fn show_matches_a_case_insensitive_scope_tag_when_no_id_matches() {
    let root = setup();
    let res = run_cli(&["show", "Tooling"], root.path());
    assert!(res.stdout.contains("LES-001"));
    assert!(res.stdout.contains("CON-001"));
}

#[test]
fn show_matches_a_type_name_as_a_last_resort() {
    let root = setup();
    let res = run_cli(&["show", "convention"], root.path());
    assert!(res.stdout.contains("CON-001"));
    assert!(!res.stdout.contains("LES-001"));
}

#[test]
fn type_filters_the_pool_before_matching() {
    let root = setup();
    let res = run_cli(&["show", "tooling", "--type", "convention"], root.path());
    assert!(res.stdout.contains("CON-001"));
    assert!(!res.stdout.contains("LES-001"));
}

#[test]
fn no_match_exits_1_and_hints_known_scopes_on_stderr() {
    let root = setup();
    let res = run_cli(&["show", "nonexistent-topic"], root.path());
    assert_eq!(res.status, 1);
    assert!(res.stderr.contains("No records match"));
    assert!(res.stderr.contains("core"));
}

#[test]
fn format_json_prints_a_uniform_flat_array() {
    let root = setup();
    let res = run_cli(&["show", "LES-001", "--format", "json"], root.path());
    let parsed = Json::parse(&res.stdout);
    let arr = parsed.as_array().expect("expected a JSON array");
    assert_eq!(arr[0].get("id").and_then(Json::as_str), Some("LES-001"));
    assert_eq!(
        arr[0].get("scope").and_then(Json::as_str),
        Some("core,tooling")
    );
}

#[test]
fn format_toon_renders_a_tabular_header_for_multiple_matches() {
    let root = setup();
    let res = run_cli(&["show", "tooling", "--format", "toon"], root.path());
    assert!(res.stdout.starts_with("[2]{"));
}

#[test]
fn missing_topic_argument_throws_a_usage_error() {
    let root = setup();
    let res = run_cli(&["show"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("Usage: agnosgram show"));
}

#[test]
fn invalid_type_throws_a_usage_error() {
    let root = setup();
    let res = run_cli(&["show", "tooling", "--type", "bogus"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("--type must be one of"));
}

#[test]
fn fri_001_show_type_dash_bogus_gives_the_existing_validation_error_not_a_raw_parseargs_crash() {
    let root = setup();
    let res = run_cli(&["show", "tooling", "--type", "-bogus"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("--type must be one of") && res.stderr.contains("got \"-bogus\""));
}

#[test]
fn a_bare_terminator_everything_after_it_stays_a_literal_positional_not_a_rewritten_option() {
    // Empirically pinned against the 0.10.0 binary: `show -- --type -x` must
    // treat "--type" as the (unmatched) topic positional, not fold it and
    // the trailing "-x" into a single --type=-x rewrite.
    let root = setup();
    let res = run_cli(&["show", "--", "--type", "-x"], root.path());
    assert_eq!(res.status, 1);
    assert!(res.stderr.contains("No records match \"--type\""));
}

#[test]
fn type_dash_dash_pitfall_dash_dash_is_never_swallowed_as_types_value() {
    let root = setup();
    let res = run_cli(&["show", "tooling", "--type", "--", "pitfall"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("Option '--type' argument is ambiguous"));
}

#[test]
fn format_json_on_no_match_prints_an_empty_json_array_to_stdout_and_still_exits_1() {
    let root = setup();
    let res = run_cli(
        &["show", "nonexistent-topic", "--format", "json"],
        root.path(),
    );
    assert_eq!(res.status, 1);
    let parsed = Json::parse(&res.stdout);
    assert_eq!(parsed.as_array(), Some(&Vec::new()));
    assert_eq!(res.stderr, "");
}

#[test]
fn show_never_surfaces_a_meta_friction_entry_even_by_exact_id_or_its_scope_tag() {
    let root = setup();
    write_store_file(
        root.path(),
        "meta/friction.md",
        "# Friction\n\n---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: meta/friction.md\n---\nTool friction, not host-project memory.\n",
    );
    let res = run_cli(&["show", "FRI-001", "--format", "json"], root.path());
    let parsed = Json::parse(&res.stdout);
    assert_eq!(parsed.as_array(), Some(&Vec::new()));
    assert_eq!(res.status, 1);
}

#[test]
fn format_toon_on_no_match_prints_an_empty_toon_array_to_stdout_and_still_exits_1() {
    let root = setup();
    let res = run_cli(
        &["show", "nonexistent-topic", "--format", "toon"],
        root.path(),
    );
    assert_eq!(res.status, 1);
    assert!(
        !res.stdout.trim().is_empty(),
        "expected a non-empty structured payload on stdout"
    );
    assert_eq!(res.stderr, "");
}
