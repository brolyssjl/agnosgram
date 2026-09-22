//! New tests (no TS counterpart - see `tests/CONFORMANCE_MAP.md`'s scope:
//! that map is the frozen TS->Rust port, and this is a Rust-only addition)
//! reproducing agnosgram security audit findings 1 and 2 (2026-09-22) at the
//! CLI level, by spawning the real binary against a repo laid out exactly as
//! the audit describes.
//!
//! Finding 1: a committed `.claude/settings.json` symlink pointing at the
//! user's global Claude Code config let `agnosgram adapt --refresh` write a
//! user-scoped hook into it, because (a) `write_if_changed` had no
//! containment check and followed the symlink, and (b) `--refresh` treated
//! "a file exists at `.claude/hooks/agnosgram-session-start.mjs`" as proof
//! hooks were already installed, when it was really committed decoy
//! content.
//!
//! Finding 2: `apply_adapter` read and then wrote through any adapter
//! target with no containment check, so a symlinked `CLAUDE.md` (or any
//! other adapter target) let `agnosgram adapt` corrupt an arbitrary
//! user-writable file outside the project.
mod common;
use common::{init_store, run_cli, TempDir};
use std::fs;
use std::os::unix::fs::symlink;

fn setup() -> TempDir {
    let root = TempDir::new("agnos-write-containment-conf");
    init_store(root.path());
    root
}

#[test]
fn finding_1_a_symlinked_claude_settings_json_is_refused_not_written_through() {
    let root = setup();
    // Stand-in for the user's global ~/.claude/settings.json: a file in a
    // separate temp dir, with its own pre-existing content.
    let home = TempDir::new("agnos-write-containment-home");
    let home_settings = home.path().join("settings.json");
    fs::write(&home_settings, "{\"untouched\":true}\n").unwrap();

    fs::create_dir_all(root.path().join(".claude")).unwrap();
    symlink(&home_settings, root.path().join(".claude/settings.json")).unwrap();

    // Explicit --claude-hooks so this test exercises write_if_changed's
    // containment check directly (see the sibling test below for the
    // --refresh content-signature gate, finding 1's other half).
    let res = run_cli(&["adapt", "--claude-hooks"], root.path());

    assert_ne!(
        res.status, 0,
        "stdout: {}\nstderr: {}",
        res.stdout, res.stderr
    );
    assert!(
        res.stderr.contains("outside the project") || res.stderr.contains("symlink"),
        "expected a containment refusal, got stderr: {}",
        res.stderr
    );

    // The real fix: the user's global config was never touched.
    assert_eq!(
        fs::read_to_string(&home_settings).unwrap(),
        "{\"untouched\":true}\n",
        "the file outside the project must be byte-identical after the refused write"
    );

    // The hook scripts themselves are ordinary in-repo files (not
    // symlinked), so they were written successfully before the
    // settings.json write was refused.
    assert!(root
        .path()
        .join(".claude/hooks/agnosgram-session-start.mjs")
        .exists());
}

#[test]
fn finding_1_bare_refresh_never_installs_hooks_from_a_committed_decoy_script() {
    let root = setup();
    let home = TempDir::new("agnos-write-containment-home2");
    let home_settings = home.path().join("settings.json");
    fs::write(&home_settings, "{\"untouched\":true}\n").unwrap();

    // A repo-committed file at the hook script's path with unrelated
    // ("decoy") content and no agnosgram signature - this is exactly what
    // made the old path-existence-only `hooks_installed` check return true.
    fs::create_dir_all(root.path().join(".claude/hooks")).unwrap();
    fs::write(
        root.path()
            .join(".claude/hooks/agnosgram-session-start.mjs"),
        "#!/usr/bin/env node\nconsole.log('not actually agnosgram');\n",
    )
    .unwrap();
    symlink(&home_settings, root.path().join(".claude/settings.json")).unwrap();

    // No --claude-hooks: this is the exact repro from the audit report.
    let res = run_cli(&["adapt", "--refresh"], root.path());

    // Whatever the exit code (there may be nothing else to do), hooks must
    // never have been (re)installed, and the outside file must be
    // untouched.
    assert!(
        !res.stdout.contains("Claude Code hooks + skill"),
        "hooks must not be installed by a bare --refresh against a decoy script; stdout: {}",
        res.stdout
    );
    assert_eq!(
        fs::read_to_string(&home_settings).unwrap(),
        "{\"untouched\":true}\n"
    );
}

#[test]
fn finding_1_refresh_does_reinstall_once_the_signature_is_present() {
    let root = setup();
    // A real, prior `--claude-hooks` run: install once for real, then a
    // bare --refresh should pick the hooks back up (existing behavior,
    // still required - this is the positive counterpart of the two tests
    // above).
    let first = run_cli(&["adapt", "--claude-hooks"], root.path());
    assert_eq!(first.status, 0, "stderr: {}", first.stderr);
    assert!(root
        .path()
        .join(".claude/hooks/agnosgram-session-start.mjs")
        .exists());

    let second = run_cli(&["adapt", "--refresh"], root.path());
    assert_eq!(second.status, 0, "stderr: {}", second.stderr);
    assert!(second.stdout.contains("Claude Code hooks + skill"));
}

#[test]
fn finding_2_a_symlinked_claude_md_is_refused_not_written_through() {
    let root = setup();
    let outside_dir = TempDir::new("agnos-write-containment-outside");
    let outside_file = outside_dir.path().join(".zshrc");
    fs::write(&outside_file, "# my precious zshrc\n").unwrap();
    symlink(&outside_file, root.path().join("CLAUDE.md")).unwrap();

    let res = run_cli(&["adapt", "claude"], root.path());

    assert_ne!(
        res.status, 0,
        "stdout: {}\nstderr: {}",
        res.stdout, res.stderr
    );
    assert!(
        res.stderr.contains("outside the project") || res.stderr.contains("symlink"),
        "expected a containment refusal, got stderr: {}",
        res.stderr
    );
    assert_eq!(
        fs::read_to_string(&outside_file).unwrap(),
        "# my precious zshrc\n",
        "the file outside the project must be byte-identical after the refused write"
    );
}

#[test]
fn finding_2_one_escaping_adapter_target_does_not_abort_the_others() {
    let root = setup();
    let outside_dir = TempDir::new("agnos-write-containment-outside2");
    let outside_file = outside_dir.path().join(".zshrc");
    fs::write(&outside_file, "# do not touch\n").unwrap();
    symlink(&outside_file, root.path().join("CLAUDE.md")).unwrap();

    // "claude" (CLAUDE.md, symlinked outside) should be refused; "cursor"
    // (its own dedicated, in-repo file) should still be written.
    let res = run_cli(&["adapt", "claude", "cursor"], root.path());

    assert_ne!(res.status, 0, "the run must still fail overall");
    assert!(res.stderr.contains("outside the project") || res.stderr.contains("symlink"));

    assert_eq!(
        fs::read_to_string(&outside_file).unwrap(),
        "# do not touch\n"
    );

    let cursor_file = root.path().join(".cursor/rules/agnosgram.mdc");
    assert!(
        cursor_file.exists(),
        "the non-escaping adapter must still have been written"
    );
    assert!(fs::read_to_string(&cursor_file)
        .unwrap()
        .contains(".agnosgram/MEMORY.md"));
}

#[test]
fn finding_2_a_claude_md_to_agents_md_symlink_inside_the_repo_still_works() {
    let root = setup();
    symlink("AGENTS.md", root.path().join("CLAUDE.md")).unwrap();

    let res = run_cli(&["adapt", "claude", "agents"], root.path());

    assert_eq!(res.status, 0, "stderr: {}", res.stderr);
    assert!(res
        .stdout
        .contains("CLAUDE.md -> AGENTS.md (symlink), managed block written once"));
    let agents_content = fs::read_to_string(root.path().join("AGENTS.md")).unwrap();
    assert_eq!(agents_content.matches("agnosgram:start").count(), 1);
}
