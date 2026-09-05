//! Minimal git shell-out used by `doctor`'s `git.untracked` check (see
//! `commands/doctor.rs`). No TS counterpart - added after the 2026-08-22 TS
//! retirement, once the crate was Rust-only. The zero-dependency policy
//! (CON-001) is about `[dependencies]` in `Cargo.toml`, not about shelling
//! out to `git` - every host repo this tool ever runs against already
//! requires git on `PATH`.

use std::path::Path;
use std::process::Command;

/// Files under `scope_rel` (a path relative to `root`, e.g. `.agnosgram`)
/// that git considers untracked and not ignored: `git ls-files --others
/// --exclude-standard -- <scope_rel>`, run with `root` as the working
/// directory. `--exclude-standard` already applies `.gitignore`,
/// `.git/info/exclude`, and the user's global excludes, so a file the
/// repo's own conventions ignore is never returned here - callers never
/// need to hand-roll ignore-pattern matching.
///
/// Returns `None` when `root` is not inside a git working tree, or the
/// `git` binary is unavailable - both cases a caller should treat as "this
/// check does not apply here", not as an error to surface.
pub fn untracked_files(root: &Path, scope_rel: &str) -> Option<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--others", "--exclude-standard", "--"])
        .arg(scope_rel)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Some(
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|l| l.replace('\\', "/"))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Stdio;

    fn tmp_dir(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("agnos-git-rs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("failed to run git");
        assert!(status.success(), "git {args:?} failed in {root:?}");
    }

    fn git_repo(root: &Path) {
        git(root, &["init", "-q"]);
        git(root, &["config", "commit.gpgsign", "false"]);
        git(root, &["config", "user.email", "test@example.com"]);
        git(root, &["config", "user.name", "Test"]);
    }

    #[test]
    fn returns_none_outside_a_git_working_tree() {
        let root = tmp_dir("none");
        fs::write(root.join("a.txt"), "x").unwrap();
        assert!(untracked_files(&root, ".").is_none());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn lists_an_untracked_file_scoped_to_a_subdirectory() {
        let root = tmp_dir("untracked");
        git_repo(&root);
        fs::create_dir_all(root.join(".agnosgram/meta")).unwrap();
        fs::write(root.join(".agnosgram/meta/friction.md"), "x").unwrap();
        let found = untracked_files(&root, ".agnosgram").expect("expected Some in a git repo");
        assert_eq!(found, vec![".agnosgram/meta/friction.md".to_string()]);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_fully_committed_scope_has_no_untracked_files() {
        let root = tmp_dir("tracked");
        git_repo(&root);
        fs::create_dir_all(root.join(".agnosgram")).unwrap();
        fs::write(root.join(".agnosgram/config.yml"), "version: 1\n").unwrap();
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-q", "-m", "init"]);
        assert_eq!(untracked_files(&root, ".agnosgram"), Some(Vec::new()));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_gitignored_file_is_not_returned() {
        let root = tmp_dir("ignored");
        fs::write(root.join(".gitignore"), "ignored.md\n").unwrap();
        git_repo(&root);
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-q", "-m", "init"]);
        fs::create_dir_all(root.join(".agnosgram/meta")).unwrap();
        fs::write(root.join(".agnosgram/meta/ignored.md"), "x").unwrap();
        assert_eq!(untracked_files(&root, ".agnosgram"), Some(Vec::new()));
        fs::remove_dir_all(&root).unwrap();
    }
}
