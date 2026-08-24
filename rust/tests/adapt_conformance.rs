//! Ported from `src/commands/adapt.conformance.test.ts`. See
//! `tests/CONFORMANCE_MAP.md`.
mod common;
use common::{init_store, run_cli, Json, TempDir};
use std::fs;
use std::os::unix::fs::symlink;

fn setup() -> TempDir {
    let root = TempDir::new("agnos-adapt-conf");
    init_store(root.path());
    root
}

#[test]
fn a_dash_leading_positional_gives_a_clean_usererror_not_a_raw_parseargs_crash() {
    let root = setup();
    let res = run_cli(&["adapt", "-claude"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("Unknown option"));
}

#[test]
fn claude_md_symlinked_to_agents_md_reports_one_honest_write_not_two() {
    let root = setup();
    symlink("AGENTS.md", root.path().join("CLAUDE.md")).unwrap();

    let first = run_cli(&["adapt", "claude", "agents"], root.path());
    assert_eq!(first.status, 0);
    assert!(first
        .stdout
        .contains("CLAUDE.md -> AGENTS.md (symlink), managed block written once"));
    assert!(!first.stdout.contains("created CLAUDE.md (Claude Code)"));
    assert!(!first.stdout.contains("created AGENTS.md (AGENTS.md)"));

    let agents_content = fs::read_to_string(root.path().join("AGENTS.md")).unwrap();
    assert_eq!(agents_content.matches("agnosgram:start").count(), 1);

    let second = run_cli(&["adapt", "claude", "agents"], root.path());
    assert_eq!(second.status, 0);
    assert!(second.stdout.contains("unchanged"));
    assert!(second.stdout.contains("CLAUDE.md -> AGENTS.md (symlink)"));
    assert_eq!(
        fs::read_to_string(root.path().join("AGENTS.md")).unwrap(),
        agents_content
    );
}

#[test]
fn the_reverse_symlink_direction_is_also_honest() {
    let root = setup();
    symlink("CLAUDE.md", root.path().join("AGENTS.md")).unwrap();

    let res = run_cli(&["adapt", "claude", "agents"], root.path());
    assert_eq!(res.status, 0);
    assert!(res
        .stdout
        .contains("AGENTS.md -> CLAUDE.md (symlink), managed block written once"));
}

#[test]
fn json_reports_the_write_once_with_the_symlinked_adapter_as_an_alias() {
    let root = setup();
    symlink("AGENTS.md", root.path().join("CLAUDE.md")).unwrap();

    let res = run_cli(&["adapt", "claude", "agents", "--json"], root.path());
    assert_eq!(res.status, 0);
    let parsed = Json::parse(&res.stdout);
    let adapters = parsed.get("adapters").and_then(Json::as_array).unwrap();
    assert_eq!(adapters.len(), 1);
    assert_eq!(
        adapters[0].get("path").and_then(Json::as_str),
        Some("AGENTS.md")
    );
    let aliases = adapters[0]
        .get("symlinkAliases")
        .and_then(Json::as_array)
        .unwrap();
    assert_eq!(aliases.len(), 1);
    assert_eq!(
        aliases[0].get("adapter").and_then(Json::as_str),
        Some("claude")
    );
    assert_eq!(
        aliases[0].get("path").and_then(Json::as_str),
        Some("CLAUDE.md")
    );
}

#[test]
fn non_symlinked_claude_md_and_agents_md_are_still_reported_independently() {
    let root = setup();
    let res = run_cli(&["adapt", "claude", "agents"], root.path());
    assert_eq!(res.status, 0);
    assert!(
        res.stdout.contains("created")
            && res.stdout.contains("CLAUDE.md")
            && res.stdout.contains("(Claude Code)")
    );
    assert!(
        res.stdout.contains("created")
            && res.stdout.contains("AGENTS.md")
            && res.stdout.contains("(AGENTS.md)")
    );
}
