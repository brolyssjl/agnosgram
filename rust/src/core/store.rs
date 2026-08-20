//! Port of `src/core/store.ts`: read-side helpers for walking a
//! `.agnosgram/` store.

use std::fs;
use std::path::{Path, PathBuf};

use super::paths::memory_dir;

#[derive(Clone)]
pub struct StoreFile {
    /// Absolute path on disk.
    pub path: PathBuf,
    /// Path relative to the project root, e.g. `.agnosgram/lessons/pitfalls.md`.
    pub rel: String,
    /// Path relative to the store dir, e.g. `lessons/pitfalls.md`.
    pub store_rel: String,
    pub text: String,
    /// True when records should be extracted (lessons/*.md, decisions
    /// NNNN-*.md, meta/*.md).
    pub record_bearing: bool,
}

fn decision_file_re_matches(base: &str) -> bool {
    // `^\d{4}-.*\.md$`
    if !base.ends_with(".md") {
        return false;
    }
    let bytes = base.as_bytes();
    bytes.len() >= 4 + 1 + 3 && bytes[0..4].iter().all(|b| b.is_ascii_digit()) && bytes[4] == b'-'
}

fn walk(dir: &Path, acc: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let full = entry.path();
        if let Ok(file_type) = entry.file_type() {
            if file_type.is_dir() {
                walk(&full, acc);
            } else if file_type.is_file() {
                acc.push(full);
            }
        }
    }
}

fn is_record_bearing(store_rel: &str, base: &str) -> bool {
    let mut parts = store_rel.split('/');
    let first = parts.next().unwrap_or("");
    if first == "lessons" && store_rel.ends_with(".md") {
        return true;
    }
    if first == "decisions" && decision_file_re_matches(base) {
        return true;
    }
    if first == "meta" && store_rel.ends_with(".md") {
        return true;
    }
    false
}

fn to_forward_slashes(s: &str) -> String {
    s.replace('\\', "/")
}

pub fn read_store(root: &Path) -> Vec<StoreFile> {
    let dir = memory_dir(root);
    let mut files = Vec::new();
    walk(&dir, &mut files);

    let mut out: Vec<StoreFile> = Vec::new();
    for path in files {
        if !path.to_string_lossy().ends_with(".md") {
            continue;
        }
        let store_rel =
            to_forward_slashes(&path.strip_prefix(&dir).unwrap_or(&path).to_string_lossy());
        if store_rel.starts_with("journal/archive/") {
            continue;
        }
        let base = store_rel
            .rsplit('/')
            .next()
            .unwrap_or(&store_rel)
            .to_string();
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let rel = to_forward_slashes(&path.strip_prefix(root).unwrap_or(&path).to_string_lossy());
        let record_bearing = is_record_bearing(&store_rel, &base);
        out.push(StoreFile {
            path: path.clone(),
            rel,
            store_rel,
            text,
            record_bearing,
        });
    }
    out.sort_by(|a, b| a.store_rel.cmp(&b.store_rel));
    out
}

pub fn path_exists(path: &Path) -> bool {
    path.exists()
}

/// Journal months present in the store (`journal/YYYY-MM.md`), oldest first.
pub fn journal_months(root: &Path) -> Vec<String> {
    let dir = memory_dir(root).join("journal");
    if !path_exists(&dir) {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut months: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if is_journal_month_file(&name) {
                Some(name[..7].to_string())
            } else {
                None
            }
        })
        .collect();
    months.sort();
    months
}

fn is_journal_month_file(name: &str) -> bool {
    // `^\d{4}-\d{2}\.md$`
    let bytes = name.as_bytes();
    bytes.len() == 10
        && bytes[0..4].iter().all(|b| b.is_ascii_digit())
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && &name[7..] == ".md"
}
