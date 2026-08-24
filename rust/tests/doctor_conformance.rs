//! Ported from `src/commands/doctor.conformance.test.ts`. See
//! `tests/CONFORMANCE_MAP.md`.
mod common;
use common::{init_store, run_cli, write_store_file, Json, TempDir};

fn setup() -> TempDir {
    let root = TempDir::new("agnos-doctor-conf");
    init_store(root.path());
    root
}

const OLD_LESSON: &str = "# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [x]\nconfidence: high\ncreated: 2000-01-01\nlast_verified: 2000-01-01\nsource: journal/2026-07.md\n---\nvery old lesson\n";

#[test]
fn doctor_on_a_healthy_store_exits_0_and_reports_no_issues() {
    let root = setup();
    let res = run_cli(&["doctor"], root.path());
    assert_eq!(res.status, 0);
    assert!(res.stdout.contains("no issues found"));
}

#[test]
fn doctor_json_prints_the_report_shape() {
    let root = setup();
    let res = run_cli(&["doctor", "--json"], root.path());
    assert_eq!(res.status, 0);
    let parsed = Json::parse(&res.stdout);
    assert_eq!(parsed.get("ok").and_then(Json::as_bool), Some(true));
    assert_eq!(parsed.get("errors").and_then(Json::as_f64), Some(0.0));
    assert!(parsed.get("findings").and_then(Json::as_array).is_some());
}

#[test]
fn warnings_alone_keep_exit_0_without_strict() {
    let root = setup();
    write_store_file(root.path(), "lessons/pitfalls.md", OLD_LESSON);
    let res = run_cli(&["doctor"], root.path());
    assert_eq!(res.status, 0);
    assert!(res.stdout.contains("warning"));
}

#[test]
fn strict_turns_warnings_into_a_failing_exit_code() {
    let root = setup();
    write_store_file(root.path(), "lessons/pitfalls.md", OLD_LESSON);
    let res = run_cli(&["doctor", "--strict"], root.path());
    assert_eq!(res.status, 1);
}

#[test]
fn doctor_without_a_store_gives_a_clean_usererror() {
    let empty = TempDir::new("agnos-doctor-nostore");
    let res = run_cli(&["doctor"], empty.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("No .agnosgram/ store found"));
}

#[test]
fn doctor_format_dash_json_gives_the_existing_validation_error_not_a_raw_parseargs_crash() {
    let root = setup();
    let res = run_cli(&["doctor", "--format", "-json"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("unknown --format \"-json\""));
}
