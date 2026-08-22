//! Ported from `src/commands/advise.conformance.test.ts`. See
//! `tests/CONFORMANCE_MAP.md`.
mod common;
use common::{init_store, run_cli, write_store_file, Json, TempDir};
use std::fs;

fn setup() -> TempDir {
    let root = TempDir::new("agnos-advise");
    init_store(root.path());
    write_store_file(
        root.path(),
        "lessons/pitfalls.md",
        "# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nDo not use require() in this ESM package; it fails at runtime.\n",
    );
    fs::write(
        root.path().join("plan.md"),
        "We propose to use require() everywhere for simplicity.\n",
    )
    .unwrap();
    root
}

/// A well-formed, provenance-correct advise report as raw JSON text, with a
/// single top-level string field overridden (matches how the TS fixture
/// builds a `Record<string, unknown>` and spreads overrides).
fn valid_report(overrides: &[(&str, &str)]) -> String {
    let mut plan = "plan.md".to_string();
    let mut clear = "false".to_string();
    for (k, v) in overrides {
        match *k {
            "plan" => plan = v.to_string(),
            "clear" => clear = v.to_string(),
            _ => {}
        }
    }
    format!(
        "{{\"agnosgram_advise\":1,\"plan\":{plan:?},\"generated\":\"2026-07-27\",\"checked_ids\":[\"LES-001\"],\"contradictions\":[{{\"record_id\":\"LES-001\",\"kind\":\"empirical\",\"severity\":\"blocker\",\"plan_excerpt\":\"use require() everywhere\",\"record_excerpt\":\"Do not use require() in this ESM package\",\"confidence\":\"high\",\"last_verified\":\"2026-07-21\",\"explanation\":\"Plan proposes require() but LES-001 forbids it.\"}}],\"clear\":{clear}}}"
    )
}

/// Same shape, but with one field inside the sole contradiction overridden.
fn report_with_contradiction_field(field: &str, value: &str) -> String {
    format!(
        "{{\"agnosgram_advise\":1,\"plan\":\"plan.md\",\"generated\":\"2026-07-27\",\"checked_ids\":[\"LES-001\"],\"contradictions\":[{{\"record_id\":\"LES-001\",\"kind\":\"empirical\",\"severity\":\"blocker\",\"plan_excerpt\":\"use require() everywhere\",\"record_excerpt\":\"Do not use require() in this ESM package\",\"confidence\":\"high\",\"last_verified\":\"2026-07-21\",\"explanation\":\"Plan proposes require() but LES-001 forbids it.\",{field:?}:{value}}}],\"clear\":false}}"
    )
}

fn report_with_no_contradictions(clear: bool) -> String {
    format!(
        "{{\"agnosgram_advise\":1,\"plan\":\"plan.md\",\"generated\":\"2026-07-27\",\"checked_ids\":[\"LES-001\"],\"contradictions\":[],\"clear\":{clear}}}"
    )
}

#[test]
fn advise_emits_a_prompt_naming_the_plan_the_digest_and_the_precedence_rule_verbatim() {
    let root = setup();
    let res = run_cli(&["advise", "plan.md"], root.path());
    assert_eq!(res.status, 0);
    assert!(res.stdout.contains("advise task"));
    assert!(res.stdout.contains("plan.md"));
    assert!(res.stdout.contains("LES-001"));
    assert!(res.stdout.contains("Normative contradiction"));
    assert!(res.stdout.contains("Empirical contradiction"));
    assert!(res.stdout.contains("agnosgram advise --validate plan.md.advise.json"));
}

#[test]
fn advise_out_changes_the_report_path_referenced_in_the_prompt() {
    let root = setup();
    let res = run_cli(&["advise", "plan.md", "--out", "custom.json"], root.path());
    assert!(res.stdout.contains("custom.json"));
}

#[test]
fn fri_001_advise_out_dash_custom_json_accepts_a_dash_leading_path_not_a_crash() {
    let root = setup();
    let res = run_cli(&["advise", "plan.md", "--out", "-custom.json"], root.path());
    assert_eq!(res.status, 0);
    assert!(res.stdout.contains("-custom.json"));
}

#[test]
fn advise_validate_passes_a_well_formed_provenance_correct_report() {
    let root = setup();
    fs::write(root.path().join("report.json"), valid_report(&[])).unwrap();
    let res = run_cli(&["advise", "--validate", "report.json"], root.path());
    assert!(res.stdout.contains("valid"));
    assert_ne!(res.status, 1);
}

#[test]
fn advise_validate_fails_on_a_confidence_provenance_mismatch() {
    let root = setup();
    fs::write(root.path().join("report.json"), report_with_contradiction_field("confidence", "\"low\"")).unwrap();
    let res = run_cli(&["advise", "--validate", "report.json"], root.path());
    assert_eq!(res.status, 1);
    assert!(res.stdout.contains("provenance.confidence.mismatch"));
}

#[test]
fn advise_validate_fails_when_a_cited_record_id_does_not_exist() {
    let root = setup();
    fs::write(root.path().join("report.json"), report_with_contradiction_field("record_id", "\"LES-999\"")).unwrap();
    let res = run_cli(&["advise", "--validate", "report.json"], root.path());
    assert_eq!(res.status, 1);
    assert!(res.stdout.contains("provenance.record_id.unknown"));
}

#[test]
fn advise_validate_warns_does_not_error_on_an_excerpt_substring_mismatch() {
    let root = setup();
    fs::write(
        root.path().join("report.json"),
        report_with_contradiction_field("plan_excerpt", "\"not in the plan\""),
    )
    .unwrap();
    let res = run_cli(&["advise", "--validate", "report.json"], root.path());
    assert_ne!(res.status, 1);
    assert!(res.stdout.contains("excerpt.plan_mismatch"));
}

#[test]
fn advise_validate_exit_0_without_strict_exit_1_with_strict_when_clear_is_false_but_valid() {
    let root = setup();
    fs::write(root.path().join("report.json"), report_with_no_contradictions(false)).unwrap();
    let res = run_cli(&["advise", "--validate", "report.json"], root.path());
    assert_ne!(res.status, 1);

    let res = run_cli(&["advise", "--validate", "report.json", "--strict"], root.path());
    assert_eq!(res.status, 1);
}

#[test]
fn advise_validate_json_returns_file_ok_errors_warnings_issues_report() {
    let root = setup();
    fs::write(root.path().join("report.json"), valid_report(&[])).unwrap();
    let res = run_cli(&["advise", "--validate", "report.json", "--json"], root.path());
    let parsed = Json::parse(&res.stdout);
    assert_eq!(parsed.get("file").and_then(Json::as_str), Some("report.json"));
    assert_eq!(parsed.get("ok").and_then(Json::as_bool), Some(true));
    assert_eq!(parsed.get("errors").and_then(Json::as_f64), Some(0.0));
    assert!(parsed.get("warnings").and_then(Json::as_f64).is_some());
    assert!(parsed.get("issues").and_then(Json::as_array).is_some());
    assert_eq!(
        parsed
            .get("report")
            .and_then(|r| r.get("agnosgram_advise"))
            .and_then(Json::as_f64),
        Some(1.0)
    );
}

#[test]
fn advise_validate_rejects_malformed_json() {
    let root = setup();
    fs::write(root.path().join("bad.json"), "{not json").unwrap();
    let res = run_cli(&["advise", "--validate", "bad.json"], root.path());
    assert_eq!(res.status, 1);
    assert!(res.stdout.contains("json.parse"));
}

#[test]
fn advise_validate_on_a_missing_file_throws_a_usage_error() {
    let root = setup();
    let res = run_cli(&["advise", "--validate", "nope.json"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("No such file"));
}

#[test]
fn advise_with_no_plan_path_throws_a_usage_error() {
    let root = setup();
    let res = run_cli(&["advise"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("Usage: agnosgram advise"));
}

#[test]
fn advise_validate_errors_even_without_strict_when_clear_true_coexists_with_a_blocker() {
    let root = setup();
    fs::write(root.path().join("report.json"), valid_report(&[("clear", "true")])).unwrap();
    let res = run_cli(&["advise", "--validate", "report.json"], root.path());
    assert_eq!(res.status, 1);
    assert!(res.stdout.contains("consistency.clear"));
    assert!(res.stdout.contains("blocker contradiction is present"));
}

#[test]
fn advise_validate_strict_derives_exit_mechanically_blocker_plus_clear_true_still_exits_1() {
    let root = setup();
    fs::write(root.path().join("report.json"), valid_report(&[("clear", "true")])).unwrap();
    let res = run_cli(&["advise", "--validate", "report.json", "--strict"], root.path());
    assert_eq!(res.status, 1);
}

#[test]
fn advise_validate_fails_when_checked_ids_contains_a_non_string_entry() {
    let root = setup();
    let report = "{\"agnosgram_advise\":1,\"plan\":\"plan.md\",\"generated\":\"2026-07-27\",\"checked_ids\":[\"LES-001\",42],\"contradictions\":[{\"record_id\":\"LES-001\",\"kind\":\"empirical\",\"severity\":\"blocker\",\"plan_excerpt\":\"use require() everywhere\",\"record_excerpt\":\"Do not use require() in this ESM package\",\"confidence\":\"high\",\"last_verified\":\"2026-07-21\",\"explanation\":\"Plan proposes require() but LES-001 forbids it.\"}],\"clear\":false}";
    fs::write(root.path().join("report.json"), report).unwrap();
    let res = run_cli(&["advise", "--validate", "report.json"], root.path());
    assert_eq!(res.status, 1);
    assert!(res.stdout.contains("schema.checked_ids"));
}

#[test]
fn advise_validate_falls_back_to_a_root_relative_plan_path_from_a_subdirectory() {
    let root = setup();
    fs::create_dir_all(root.path().join("sub")).unwrap();
    fs::write(root.path().join("sub/report.json"), valid_report(&[])).unwrap();
    let res = run_cli(&["advise", "--validate", "report.json"], &root.path().join("sub"));
    assert!(res.stdout.contains("valid"));
    assert!(!res.stdout.contains("coverage.plan_missing"));
}

#[test]
fn advise_validate_resolves_an_absolute_plan_path() {
    let root = setup();
    let abs_plan = root.path().join("plan.md");
    let abs_plan_str = abs_plan.to_str().unwrap().to_string();
    fs::write(root.path().join("report.json"), valid_report(&[("plan", &abs_plan_str)])).unwrap();
    let res = run_cli(&["advise", "--validate", "report.json"], root.path());
    assert!(res.stdout.contains("valid"));
    assert!(!res.stdout.contains("coverage.plan_missing"));
}

#[test]
fn advises_digest_never_includes_meta_friction_md_content() {
    let root = setup();
    write_store_file(
        root.path(),
        "meta/friction.md",
        "# Friction\n\n---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: meta/friction.md\n---\nThis is tool friction, not host-project memory.\n",
    );
    let res = run_cli(&["advise", "plan.md"], root.path());
    assert!(!res.stdout.contains("FRI-001"));
    assert!(!res.stdout.contains("tool friction, not host-project memory"));
}

#[test]
fn advise_digest_table_escapes_pipes_in_body_excerpts_and_only_appends_ellipsis_when_truncated() {
    let root = setup();
    write_store_file(
        root.path(),
        "lessons/pitfalls.md",
        "# Pitfalls\n\n---\nid: LES-002\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nShort body with a | pipe in it.\n",
    );
    let res = run_cli(&["advise", "plan.md"], root.path());
    assert!(res.stdout.contains("a \\| pipe in it."));
    let row = res.stdout.lines().find(|l| l.contains("LES-002")).expect("row for LES-002");
    assert!(!row.contains("..."));
}

#[test]
fn advise_digest_table_appends_ellipsis_only_when_the_body_actually_exceeds_80_chars() {
    let root = setup();
    let body = "x".repeat(120);
    write_store_file(
        root.path(),
        "lessons/pitfalls.md",
        &format!("# Pitfalls\n\n---\nid: LES-003\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\n{body}\n"),
    );
    let res = run_cli(&["advise", "plan.md"], root.path());
    let row = res.stdout.lines().find(|l| l.contains("LES-003")).expect("row for LES-003");
    assert!(row.contains("..."));
}
