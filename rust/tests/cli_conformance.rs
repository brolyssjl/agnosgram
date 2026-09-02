//! Ported from `src/cli.conformance.test.ts`. See `tests/CONFORMANCE_MAP.md`.
mod common;
use common::{looks_like_semver, run_cli, TempDir};

#[test]
fn version_prints_a_bare_semver_looking_string() {
    let cwd = TempDir::new("agnos-cli");
    let res = run_cli(&["--version"], cwd.path());
    assert_eq!(res.status, 0);
    assert!(
        looks_like_semver(res.stdout.trim()),
        "not semver-looking: {:?}",
        res.stdout
    );
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

// agnosgram#26: every subcommand used to reject -h/--help with "Unknown
// option", because no subcommand's ArgsConfig declared it - the request
// fell straight into parse_cli_args' strict parser. Each of these prints
// that subcommand's own usage block and exits 0, checked for both spellings
// and without requiring a store to exist (help never touches disk).
const SUBCOMMANDS: [&str; 11] = [
    "init",
    "adapt",
    "log",
    "doctor",
    "distill",
    "bootstrap",
    "show",
    "pack",
    "advise",
    "feedback",
    "reflect",
];

#[test]
fn every_subcommand_help_flag_prints_its_own_usage_and_exits_0() {
    let cwd = TempDir::new("agnos-cli-help");
    for cmd in SUBCOMMANDS {
        for flag in ["--help", "-h"] {
            let res = run_cli(&[cmd, flag], cwd.path());
            assert_eq!(res.status, 0, "{cmd} {flag}: {:?}", res.stderr);
            assert!(
                res.stdout.contains("Usage:") && res.stdout.contains(&format!("agnosgram {cmd}")),
                "{cmd} {flag} stdout did not look like a usage block: {:?}",
                res.stdout
            );
            assert!(
                !res.stdout.contains("Unknown option") && !res.stderr.contains("Unknown option"),
                "{cmd} {flag} still rejected help: stdout={:?} stderr={:?}",
                res.stdout,
                res.stderr
            );
        }
    }
}

#[test]
fn subcommand_help_prints_only_that_commands_usage_not_the_full_command_list() {
    let cwd = TempDir::new("agnos-cli-help-scope");
    let res = run_cli(&["distill", "--help"], cwd.path());
    assert_eq!(res.status, 0);
    assert!(res.stdout.contains("agnosgram distill"));
    // The full top-level help lists every other command by name under
    // "Commands:" - a scoped usage block must not leak that section.
    assert!(!res.stdout.contains("Commands:"));
    assert!(!res.stdout.contains("agnosgram doctor "));
}

// agnosgram#28c: doctor --strict exits non-zero on warnings alone, which
// reads as a pass at a glance ("0 error(s), N warning(s)") while the exit
// code says fail - the top-level help text now documents that.
#[test]
fn help_documents_that_doctor_strict_fails_on_warnings_alone() {
    let cwd = TempDir::new("agnos-cli-strict-doc");
    let res = run_cli(&["--help"], cwd.path());
    assert_eq!(res.status, 0);
    assert!(
        res.stdout.contains("--strict") && res.stdout.contains("warnings"),
        "{}",
        res.stdout
    );
}
