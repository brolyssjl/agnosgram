//! Port of `src/core/paths.ts`: project-root and store-path resolution.

use std::path::{Component, Path, PathBuf};

pub const MEMORY_DIR: &str = ".agnosgram";

/// Lexical equivalent of Node's `path.resolve`: makes `path` absolute
/// (against the current directory when relative) and collapses `.`/`..`
/// segments, without touching the filesystem or resolving symlinks.
fn lexical_absolute(path: &Path) -> PathBuf {
    let base = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(path)
    };
    let mut out = PathBuf::new();
    for comp in base.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Find the project root: the nearest ancestor containing `.agnosgram/`, else
/// the nearest containing `.git/`, else the starting directory.
pub fn find_project_root(start: &Path) -> PathBuf {
    let start = lexical_absolute(start);
    let mut dir = start.clone();
    loop {
        if dir.join(MEMORY_DIR).exists() {
            return dir;
        }
        match dir.parent() {
            Some(parent) if parent != dir => dir = parent.to_path_buf(),
            _ => break,
        }
    }
    let mut dir = start.clone();
    loop {
        if dir.join(".git").exists() {
            return dir;
        }
        match dir.parent() {
            Some(parent) if parent != dir => dir = parent.to_path_buf(),
            _ => break,
        }
    }
    start
}

pub fn memory_dir(root: &Path) -> PathBuf {
    root.join(MEMORY_DIR)
}

pub fn config_path(root: &Path) -> PathBuf {
    memory_dir(root).join("config.yml")
}

pub fn has_store(root: &Path) -> bool {
    memory_dir(root).exists()
}
