//! Ported from `src/commands/doctor.conformance.test.ts`. See
//! `tests/CONFORMANCE_MAP.md`.
mod common;
use common::{init_store, run_cli, write_store_file, Json, TempDir};
use std::fs;

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

#[test]
fn doctor_flags_distill_lag_once_the_journal_has_real_entries_but_nothing_was_distilled() {
    let root = setup();
    let log = run_cli(&["log", "--did", "soak test entry"], root.path());
    assert_eq!(log.status, 0, "log failed: {}", log.stderr);

    let res = run_cli(&["doctor"], root.path());
    assert_eq!(
        res.status, 0,
        "warnings alone should not fail without --strict"
    );
    assert!(res.stdout.contains("distill.lag"), "{}", res.stdout);
    assert!(res.stdout.contains("agnosgram distill"), "{}", res.stdout);
}

#[test]
fn doctor_flags_status_stale_when_the_journal_outpaces_status_mds_recorded_freshness() {
    let root = setup();
    let memory_path = root.path().join(".agnosgram/MEMORY.md");
    let memory = fs::read_to_string(&memory_path).unwrap();
    let today_row = memory
        .lines()
        .find(|l| l.trim_start().starts_with("| state/status.md"))
        .expect("expected a state/status.md freshness row");
    let stale_row =
        today_row.replacen(today_row.split('|').nth(2).unwrap().trim(), "2000-01-01", 1);
    fs::write(&memory_path, memory.replace(today_row, &stale_row)).unwrap();

    let log = run_cli(&["log", "--did", "soak test entry"], root.path());
    assert_eq!(log.status, 0, "log failed: {}", log.stderr);

    let res = run_cli(&["doctor"], root.path());
    assert!(res.stdout.contains("status.stale"), "{}", res.stdout);
}

#[test]
fn doctor_on_a_fresh_store_with_no_journal_entries_never_flags_recall_freshness() {
    let root = setup();
    let res = run_cli(&["doctor"], root.path());
    assert_eq!(res.status, 0);
    assert!(!res.stdout.contains("status.stale"));
    assert!(!res.stdout.contains("distill.lag"));
}

// agnosgram#27: reword the status.stale message to say where the recorded
// date actually lives, since the old wording read as if it came from
// status.md itself.
#[test]
fn status_stale_finding_names_memory_md_s_freshness_table_as_the_recorded_source() {
    let root = setup();
    let memory_path = root.path().join(".agnosgram/MEMORY.md");
    let memory = fs::read_to_string(&memory_path).unwrap();
    let today_row = memory
        .lines()
        .find(|l| l.trim_start().starts_with("| state/status.md"))
        .expect("expected a state/status.md freshness row");
    let stale_row =
        today_row.replacen(today_row.split('|').nth(2).unwrap().trim(), "2000-01-01", 1);
    fs::write(&memory_path, memory.replace(today_row, &stale_row)).unwrap();

    let log = run_cli(&["log", "--did", "soak test entry"], root.path());
    assert_eq!(log.status, 0, "log failed: {}", log.stderr);

    let res = run_cli(&["doctor"], root.path());
    assert!(res.stdout.contains("MEMORY.md"), "{}", res.stdout);
    assert!(res.stdout.contains("Freshness table"), "{}", res.stdout);
}

// agnosgram#27: doctor cross-references MEMORY.md's Freshness table row for
// state/status.md against status.md's own "Last updated:" line, and warns
// on disagreement - exactly the state a literal-minded distill run produces
// when it refreshes status.md's own line without touching the table.
#[test]
fn doctor_flags_freshness_mismatch_when_the_table_disagrees_with_status_mds_own_line() {
    let root = setup();
    let memory_path = root.path().join(".agnosgram/MEMORY.md");
    let memory = fs::read_to_string(&memory_path).unwrap();
    let today_row = memory
        .lines()
        .find(|l| l.trim_start().starts_with("| state/status.md"))
        .expect("expected a state/status.md freshness row");
    let stale_row =
        today_row.replacen(today_row.split('|').nth(2).unwrap().trim(), "2000-01-01", 1);
    fs::write(&memory_path, memory.replace(today_row, &stale_row)).unwrap();
    // status.md's own "Last updated:" line is left untouched by init's
    // scaffold - it still carries today's date, so the two now disagree.

    let res = run_cli(&["doctor"], root.path());
    assert_eq!(
        res.status, 0,
        "warnings alone should not fail without --strict"
    );
    assert!(res.stdout.contains("freshness.mismatch"), "{}", res.stdout);

    let strict = run_cli(&["doctor", "--strict"], root.path());
    assert_eq!(strict.status, 1);
}

#[test]
fn doctor_does_not_flag_freshness_mismatch_when_the_two_sources_agree() {
    let root = setup();
    let res = run_cli(&["doctor"], root.path());
    assert_eq!(res.status, 0);
    assert!(!res.stdout.contains("freshness.mismatch"));
}
