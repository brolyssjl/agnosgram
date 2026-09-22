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
mod common;
use common::{init_store, run_cli, TempDir};
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
