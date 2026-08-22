//! Ported from `src/cli.conformance.test.ts`. See `tests/CONFORMANCE_MAP.md`.
mod common;
use common::{looks_like_semver, run_cli, TempDir};

#[test]
fn version_prints_a_bare_semver_looking_string() {
    let cwd = TempDir::new("agnos-cli");
    let res = run_cli(&["--version"], cwd.path());
    assert_eq!(res.status, 0);
    assert!(looks_like_semver(res.stdout.trim()), "not semver-looking: {:?}", res.stdout);
}

#[test]
fn help_prints_usage() {
    let cwd = TempDir::new("agnos-cli");
    let res = run_cli(&["--help"], cwd.path());
    assert_eq!(res.status, 0);
    assert!(res.stdout.contains("Usage:"));
}

#[test]
fn an_unknown_command_exits_non_zero_without_a_raw_stack_trace() {
    let cwd = TempDir::new("agnos-cli");
    let res = run_cli(&["not-a-command"], cwd.path());
    assert_ne!(res.status, 0);
    assert!((res.stderr.clone() + &res.stdout).contains("Unknown command"));
}
