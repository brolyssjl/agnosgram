//! Finding 4 (2026-09-22 agnosgram security audit): the hand-rolled JSON
//! parser (`rust/src/core/json.rs`, `parse_value`/`parse_object`/
//! `parse_array`) recursed once per nesting level with no depth limit, so a
//! committed `.claude/settings.json` with tens of thousands of nested `[`
//! overflowed the call stack and made the process abort with SIGABRT
//! (exit 134) instead of returning an error. An abort cannot be caught
//! in-process (it does not unwind), so this has to be a spawned-subprocess
//! conformance test: before the fix it would observe the child process die
//! from a signal; after the fix it observes a clean non-zero exit with an
//! error message.
//!
//! Also covers finding 6 (same audit): `rust/src/core/lint.rs` caps any
//! single line/paragraph it scans at `LINT_MAX_LINE_CHARS` (16 KiB) so a
//! pathologically long line cannot make the safety lints slow or hang
//! `doctor`. `doctor` surfaces each cap hit as its own `lint.line-truncated`
//! finding (`rust/src/commands/doctor.rs`) rather than silently
//! under-scanning, so the tests below check that finding end to end through
//! `agnosgram doctor --json`.
mod common;
use common::{init_store, run_cli, write_store_file, Json, TempDir};
use std::fs;

/// 30 000 nested `[` (~60 KB), matching the audit's verified repro.
fn deeply_nested_json_array(depth: usize) -> String {
    let mut text = String::with_capacity(depth * 2);
    text.push_str(&"[".repeat(depth));
    text.push_str(&"]".repeat(depth));
    text
}

#[test]
fn deeply_nested_claude_settings_json_fails_cleanly_instead_of_aborting() {
    let root = TempDir::new("agnos-json-depth-conf");
    init_store(root.path());

    let claude_dir = root.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join("settings.json"),
        deeply_nested_json_array(30_000),
    )
    .unwrap();

    let res = run_cli(&["adapt", "--claude-hooks"], root.path());

    // Before the fix: the child process aborts (SIGABRT / stack overflow),
    // which on unix surfaces as `status.code()` being `None` - our harness
    // maps that to -1, not a normal non-zero exit. After the fix: a normal
    // `UserError` exit (1) with a descriptive message on stderr.
    assert_ne!(
        res.status, 0,
        "expected a non-zero exit for invalid JSON, got success. stdout={} stderr={}",
        res.stdout, res.stderr
    );
    assert_ne!(
        res.status, -1,
        "process appears to have crashed/aborted rather than exiting cleanly \
         (stack overflow regression?). stdout={} stderr={}",
        res.stdout, res.stderr
    );
    assert!(
        res.stderr.contains("not valid JSON"),
        "expected a clean JSON-parse error message, got stderr={}",
        res.stderr
    );
}

#[test]
fn json_parser_rejects_deep_nesting_with_a_normal_parse_error() {
    // Direct unit-level check on the parser itself (safe in-process since
    // the fix makes this a normal `Err`, not a stack overflow).
    let text = deeply_nested_json_array(30_000);
    let err = agnosgram::core::json::parse(&text).unwrap_err();
    assert!(
        err.to_string().contains("nesting too deep"),
        "unexpected error message: {err}"
    );
}

/// Finding 6: a store file with a 40 000-character line - well past
/// `LINT_MAX_LINE_CHARS` (16 384) - yields exactly one `lint.line-truncated`
/// finding from `agnosgram doctor --json`, even though the line is scanned
/// by both the secret pass and the injection pass (each of which notices
/// the same truncated line - `doctor` must dedupe them into a single
/// finding, not one per pass).
#[test]
fn a_forty_thousand_character_line_yields_exactly_one_line_truncated_finding() {
    let root = TempDir::new("agnos-lint-truncation-conf");
    init_store(root.path());

    let long_line = "x".repeat(40_000);
    write_store_file(
        root.path(),
        "context/long-line.md",
        &format!("Intro line, nothing unusual.\n\n{long_line}\n\nOutro line.\n"),
    );

    let res = run_cli(&["doctor", "--json"], root.path());
    let parsed = Json::parse(&res.stdout);
    let findings = parsed
        .get("findings")
        .and_then(Json::as_array)
        .expect("findings must be an array");

    let truncated: Vec<&Json> = findings
        .iter()
        .filter(|f| f.get("code").and_then(Json::as_str) == Some("lint.line-truncated"))
        .collect();
    assert_eq!(
        truncated.len(),
        1,
        "expected exactly one lint.line-truncated finding, got {truncated:?} \
         (full findings: {findings:?})"
    );

    let finding = truncated[0];
    assert_eq!(
        finding.get("file").and_then(Json::as_str),
        Some(".agnosgram/context/long-line.md")
    );
    assert_eq!(finding.get("line").and_then(Json::as_f64), Some(3.0));
    let message = finding
        .get("message")
        .and_then(Json::as_str)
        .expect("finding must have a message");
    assert!(
        message.contains("40000 characters") || message.contains("40,000 characters"),
        "message should mention the line's actual length: {message}"
    );
    assert!(
        message.contains("16384"),
        "message should mention the scan cap: {message}"
    );
}

/// The other half of finding 6's regression check: an ordinary store with no
/// pathologically long lines must not produce any `lint.line-truncated`
/// finding at all.
#[test]
fn a_normal_store_yields_no_line_truncated_findings() {
    let root = TempDir::new("agnos-lint-truncation-conf-normal");
    init_store(root.path());

    let res = run_cli(&["doctor", "--json"], root.path());
    let parsed = Json::parse(&res.stdout);
    let findings = parsed
        .get("findings")
        .and_then(Json::as_array)
        .expect("findings must be an array");

    assert!(
        !findings
            .iter()
            .any(|f| f.get("code").and_then(Json::as_str) == Some("lint.line-truncated")),
        "a normal store must not report line truncation: {findings:?}"
    );
}
