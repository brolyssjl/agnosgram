//! Port of `src/core/writeFile.ts`: the shared write-if-changed helper every
//! agnosgram-owned or managed-block target routes through.
//!
//! Security note (agnosgram security audit, 2026-09-22, findings 1 and 2):
//! every write here is contained to `root` and refuses to follow a symlink,
//! and every write is atomic (temp file + rename). See `check_containment`
//! and `write_atomic` below, and `docs/adapters.md` for the user-facing
//! summary of the containment rule.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::core::output::UserError;

/// `src/core/writeFile.ts`'s `WriteAction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteAction {
    Created,
    Updated,
    Unchanged,
}

impl WriteAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            WriteAction::Created => "created",
            WriteAction::Updated => "updated",
            WriteAction::Unchanged => "unchanged",
        }
    }
}

/// `src/core/writeFile.ts`'s `WriteResult`.
#[derive(Debug, Clone)]
pub struct WriteResult {
    pub path: String,
    pub action: WriteAction,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WriteOpts {
    pub executable: bool,
}

/// Resolve `root.join(rel_path)`, refusing anything that would let a write
/// (or a read performed to prepare one, e.g. `adapt`'s managed-block merge)
/// land outside the project.
///
/// - An absolute `rel_path`, or one containing a `..` component, is
///   rejected outright without touching the filesystem.
/// - Otherwise the target is resolved lexically, then we walk up from it to
///   the deepest *existing* ancestor and canonicalize that ancestor. The
///   result must still be inside the canonicalized `root`. This is what
///   catches a symlinked ancestor directory (e.g. a repo-committed
///   `.claude` symlink pointing outside the project) even when the final
///   path component doesn't exist yet.
/// - If the target itself already exists, `symlink_metadata` must report a
///   plain regular file. A symlink (valid or dangling), a directory, or
///   anything else is refused - a write can never be redirected outside the
///   project via the final path component either.
pub fn check_containment(root: &Path, rel_path: &str) -> Result<PathBuf, UserError> {
    let rel = Path::new(rel_path);
    if rel.is_absolute() {
        return Err(UserError::new(format!(
            "refusing to write {rel_path}: it is an absolute path, not one inside the project"
        )));
    }
    if rel
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(UserError::new(format!(
            "refusing to write {rel_path}: it contains '..' and would escape the project"
        )));
    }

    let canonical_root = fs::canonicalize(root).map_err(|e| {
        UserError::new(format!(
            "could not resolve project root {}: {e}",
            root.display()
        ))
    })?;

    let target = root.join(rel);

    let existing_ancestor = target
        .ancestors()
        .find(|a| a.exists())
        .map(|a| a.to_path_buf())
        .unwrap_or_else(|| root.to_path_buf());
    let canonical_ancestor = fs::canonicalize(&existing_ancestor).map_err(|e| {
        UserError::new(format!(
            "could not resolve path near {}: {e}",
            target.display()
        ))
    })?;
    if !canonical_ancestor.starts_with(&canonical_root) {
        return Err(UserError::new(format!(
            "refusing to write {rel_path}: it points outside the project (resolves to {} via {})",
            canonical_ancestor.display(),
            existing_ancestor.display()
        )));
    }

    if let Ok(meta) = fs::symlink_metadata(&target) {
        if !meta.file_type().is_file() {
            return Err(UserError::new(format!(
                "refusing to write {rel_path}: it points outside the project or is a symlink"
            )));
        }
    }

    Ok(target)
}

/// A pseudo-random `u64`, best-effort from `/dev/urandom`, falling back to
/// the wall clock when that isn't available (never fails outright - this
/// only needs to make a temp file name collision-unlikely, not secure).
fn random_component() -> u64 {
    if let Ok(mut f) = fs::File::open("/dev/urandom") {
        let mut buf = [0u8; 8];
        if f.read_exact(&mut buf).is_ok() {
            return u64::from_le_bytes(buf);
        }
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Write `content` to `target` atomically: create a uniquely-named temp file
/// in the same directory with `create_new(true)` (so we never clobber a
/// concurrent writer's temp file), write and sync it, then `rename` it onto
/// `target`. A rename within the same directory is atomic on every platform
/// this crate targets, so a crash or kill mid-write can never leave `target`
/// truncated. The temp file is removed on any failure.
fn write_atomic(target: &Path, content: &str) -> Result<(), UserError> {
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let pid = std::process::id();

    for _ in 0..8 {
        let tmp_path = parent.join(format!("{file_name}.{pid}.{}.tmp", random_component()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)
        {
            Ok(mut file) => {
                let result = file
                    .write_all(content.as_bytes())
                    .and_then(|_| file.sync_all());
                drop(file);
                if let Err(e) = result {
                    let _ = fs::remove_file(&tmp_path);
                    return Err(UserError::new(format!(
                        "failed writing temp file for {}: {e}",
                        target.display()
                    )));
                }
                if let Err(e) = fs::rename(&tmp_path, target) {
                    let _ = fs::remove_file(&tmp_path);
                    return Err(UserError::new(format!(
                        "failed to finalize write to {}: {e}",
                        target.display()
                    )));
                }
                return Ok(());
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => {
                return Err(UserError::new(format!(
                    "failed to create a temp file next to {}: {e}",
                    target.display()
                )))
            }
        }
    }
    Err(UserError::new(format!(
        "failed to create a temp file next to {} after several attempts",
        target.display()
    )))
}

/// Port of `writeIfChanged`: write `content` to `root/relPath` only when it
/// differs from what is already on disk, creating parent directories as
/// needed, and report which of created/updated/unchanged happened.
///
/// Every call is contained to `root` (see `check_containment`) and every
/// on-disk write is atomic (see `write_atomic`).
pub fn write_if_changed(
    root: &Path,
    rel_path: &str,
    content: &str,
    opts: WriteOpts,
) -> Result<WriteResult, UserError> {
    let target = check_containment(root, rel_path)?;
    let existed_before = target.exists();
    let before = if existed_before {
        Some(fs::read_to_string(&target).map_err(|e| UserError::new(e.to_string()))?)
    } else {
        None
    };
    let unchanged = before.as_deref() == Some(content);
    if !unchanged {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| UserError::new(e.to_string()))?;
        }
        write_atomic(&target, content)?;
    }
    if opts.executable {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&target, fs::Permissions::from_mode(0o755))
                .map_err(|e| UserError::new(e.to_string()))?;
        }
    }
    let action = if unchanged {
        WriteAction::Unchanged
    } else if existed_before {
        WriteAction::Updated
    } else {
        WriteAction::Created
    };
    Ok(WriteResult {
        path: rel_path.to_string(),
        action,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn tmp_root(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("agnos-writefile-rs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn no_tmp_files_left(dir: &Path) -> bool {
        fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .all(|e| !e.file_name().to_string_lossy().ends_with(".tmp"))
    }

    #[test]
    fn write_if_changed_creates_then_reports_unchanged_on_identical_content() {
        let root = tmp_root("basic");
        let first = write_if_changed(&root, "a.txt", "hello\n", WriteOpts::default()).unwrap();
        assert_eq!(first.action, WriteAction::Created);
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "hello\n");
        assert!(no_tmp_files_left(&root));

        let second = write_if_changed(&root, "a.txt", "hello\n", WriteOpts::default()).unwrap();
        assert_eq!(second.action, WriteAction::Unchanged);
        assert!(no_tmp_files_left(&root));

        let third = write_if_changed(&root, "a.txt", "bye\n", WriteOpts::default()).unwrap();
        assert_eq!(third.action, WriteAction::Updated);
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "bye\n");
        assert!(no_tmp_files_left(&root));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn check_containment_rejects_an_absolute_rel_path() {
        let root = tmp_root("abs");
        let err = check_containment(&root, "/etc/passwd").unwrap_err();
        assert!(err.0.contains("absolute"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn check_containment_rejects_a_dot_dot_component() {
        let root = tmp_root("dotdot");
        let err = check_containment(&root, "../outside.txt").unwrap_err();
        assert!(err.0.contains(".."));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn check_containment_rejects_a_symlinked_ancestor_directory() {
        let root = tmp_root("symlinked-dir");
        let outside = tmp_root("symlinked-dir-outside");
        symlink(&outside, root.join(".claude")).unwrap();

        let err = check_containment(&root, ".claude/hooks/x.mjs").unwrap_err();
        assert!(err.0.contains("outside the project"));

        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&outside).unwrap();
    }

    #[test]
    fn check_containment_rejects_a_symlinked_target_file() {
        let root = tmp_root("symlinked-file");
        let outside_dir = tmp_root("symlinked-file-outside");
        let outside_file = outside_dir.join("real.json");
        fs::write(&outside_file, "{}").unwrap();
        symlink(&outside_file, root.join("settings.json")).unwrap();

        let err = check_containment(&root, "settings.json").unwrap_err();
        assert!(err.0.contains("symlink") || err.0.contains("outside"));

        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&outside_dir).unwrap();
    }

    #[test]
    fn check_containment_rejects_a_dangling_symlinked_target_file() {
        let root = tmp_root("dangling-symlink");
        symlink("/does/not/exist-agnosgram-test", root.join("settings.json")).unwrap();

        let err = check_containment(&root, "settings.json").unwrap_err();
        assert!(err.0.contains("symlink") || err.0.contains("outside"));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn check_containment_allows_a_plain_nested_new_file() {
        let root = tmp_root("nested-new");
        let target = check_containment(&root, "a/b/c.txt").unwrap();
        assert_eq!(target, root.join("a/b/c.txt"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn write_if_changed_refuses_to_follow_a_symlinked_target_and_leaves_it_untouched() {
        let root = tmp_root("write-symlink");
        let outside_dir = tmp_root("write-symlink-outside");
        let outside_file = outside_dir.join("global.json");
        fs::write(&outside_file, "{\"untouched\":true}").unwrap();
        symlink(&outside_file, root.join("settings.json")).unwrap();

        let result = write_if_changed(
            &root,
            "settings.json",
            "{\"hooks\":{}}",
            WriteOpts::default(),
        );
        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(&outside_file).unwrap(),
            "{\"untouched\":true}"
        );

        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&outside_dir).unwrap();
    }

    #[test]
    fn write_atomic_removes_the_temp_file_when_rename_fails() {
        let root = tmp_root("atomic-failure");
        // A directory sitting where the target should be a file makes the
        // final `rename` fail (EISDIR) after the temp file was already
        // created - exercise that write_atomic cleans up after itself.
        let target = root.join("occupied");
        fs::create_dir_all(&target).unwrap();
        let err = write_atomic(&target, "content");
        assert!(err.is_err());
        assert!(no_tmp_files_left(&root));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn write_if_changed_via_a_directory_target_fails_at_containment_not_atomic_write() {
        let root = tmp_root("blocked");
        fs::create_dir_all(root.join("blocked")).unwrap();
        let err = write_if_changed(&root, "blocked", "content", WriteOpts::default());
        assert!(err.is_err());
        assert!(no_tmp_files_left(&root));
        fs::remove_dir_all(&root).unwrap();
    }
}
