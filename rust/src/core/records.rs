//! Port of `src/core/records.ts`: shared record index for `show`, `pack`,
//! `advise`. Built on `store.rs` (file enumeration) + `frontmatter.rs`
//! (parsing and schema validation). Invalid records are silently skipped -
//! `doctor` owns diagnosing them.

use super::frontmatter::{extract_records, validate_record, Frontmatter, RawRecord, KNOWN_TYPES};
use super::store::{read_store, StoreFile};
use std::path::Path;

pub struct StoreRecord {
    pub frontmatter: Frontmatter,
    pub body: String,
    /// Path relative to the project root, e.g. `.agnosgram/lessons/pitfalls.md`.
    pub file: String,
    /// Path relative to the store dir, e.g. `lessons/pitfalls.md`.
    #[allow(dead_code)]
    // populated for the TS StoreRecord shape; unread downstream (same in TS)
    pub store_rel: String,
    /// 1-based line of the record's opening `---` fence.
    #[allow(dead_code)]
    pub line: usize,
}

/// Low-level walk shared by every command that needs raw (unvalidated)
/// records. `meta/` is tool-friction feedback, not host-project memory - it
/// is excluded here explicitly, regardless of `StoreFile::record_bearing`.
pub fn iter_raw_records(root: &Path) -> Vec<(StoreFile, RawRecord)> {
    let mut out = Vec::new();
    for file in read_store(root) {
        if !file.record_bearing {
            continue;
        }
        if file.store_rel == "meta" || file.store_rel.starts_with("meta/") {
            continue;
        }
        for raw in extract_records(&file.text) {
            out.push((file.clone(), raw));
        }
    }
    out
}

/// Every schema-valid record across the store's record-bearing files.
pub fn load_records(root: &Path) -> Vec<StoreRecord> {
    let mut out = Vec::new();
    for (file, raw) in iter_raw_records(root) {
        let validated = validate_record(&raw, &KNOWN_TYPES);
        let Some(frontmatter) = validated.frontmatter else {
            continue;
        };
        out.push(StoreRecord {
            frontmatter,
            body: raw.body,
            file: file.rel,
            store_rel: file.store_rel,
            line: raw.line,
        });
    }
    out
}

/// Every distinct scope tag used across a set of records, sorted.
pub fn all_scopes_from(records: &[StoreRecord]) -> Vec<String> {
    let mut scopes: Vec<String> = Vec::new();
    for rec in records {
        for s in &rec.frontmatter.scope {
            if !scopes.contains(s) {
                scopes.push(s.clone());
            }
        }
    }
    scopes.sort();
    scopes
}

/// Every distinct scope tag used across the store's valid records, sorted.
#[cfg(test)]
pub fn all_scopes(root: &Path) -> Vec<String> {
    all_scopes_from(&load_records(root))
}

/// Case-insensitive: does this record carry `tag` among its scope list?
pub fn matches_scope(rec: &StoreRecord, tag: &str) -> bool {
    let lower = tag.to_lowercase();
    rec.frontmatter
        .scope
        .iter()
        .any(|s| s.to_lowercase() == lower)
}

/// Render a record's frontmatter + body as it looks on disk (canonical form).
pub fn render_record_block(r: &StoreRecord) -> String {
    let fm = &r.frontmatter;
    let mut lines = vec![
        format!("id: {}", fm.id),
        format!("type: {}", fm.r#type),
        format!("scope: [{}]", fm.scope.join(", ")),
        format!("confidence: {}", fm.confidence),
        format!("created: {}", fm.created),
        format!("last_verified: {}", fm.last_verified),
        format!("source: {}", fm.source),
    ];
    if !fm.supersedes.is_empty() {
        lines.push(format!("supersedes: [{}]", fm.supersedes.join(", ")));
    }
    format!("---\n{}\n---\n{}", lines.join("\n"), r.body)
}

/// Flat, tabular-friendly shape for a record in `--format json|toon` output.
pub struct FlatRecord {
    pub id: String,
    pub r#type: String,
    pub scope: String,
    pub confidence: String,
    pub created: String,
    pub last_verified: String,
    pub source: String,
    pub supersedes: String,
    pub file: String,
    pub body: String,
}

pub fn to_flat_record(r: &StoreRecord) -> FlatRecord {
    FlatRecord {
        id: r.frontmatter.id.clone(),
        r#type: r.frontmatter.r#type.clone(),
        scope: r.frontmatter.scope.join(","),
        confidence: r.frontmatter.confidence.clone(),
        created: r.frontmatter.created.clone(),
        last_verified: r.frontmatter.last_verified.clone(),
        source: r.frontmatter.source.clone(),
        supersedes: r.frontmatter.supersedes.join(","),
        file: r.file.clone(),
        body: r.body.clone(),
    }
}

fn confidence_rank(c: &str) -> i32 {
    match c {
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

/// Confidence desc, last_verified desc, id asc - the shared record ordering
/// (`show`, `pack`).
pub fn compare_records(a: &StoreRecord, b: &StoreRecord) -> std::cmp::Ordering {
    let ca = confidence_rank(&a.frontmatter.confidence);
    let cb = confidence_rank(&b.frontmatter.confidence);
    if ca != cb {
        return cb.cmp(&ca);
    }
    if a.frontmatter.last_verified != b.frontmatter.last_verified {
        return if a.frontmatter.last_verified < b.frontmatter.last_verified {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Less
        };
    }
    a.frontmatter.id.cmp(&b.frontmatter.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // `src/core/records.test.ts` scaffolds its fixture store via `runInit`
    // (a full command). `commands::init` is still a wave-1 stub, so these
    // ports build the same minimal store shape by hand instead - same
    // `load_records`/`all_scopes` coverage, no dependency on a later wave.
    fn tmp_store(name: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("agnos-records-rs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".agnosgram/lessons")).unwrap();
        root
    }

    fn write_pitfalls(root: &Path, body: &str) {
        fs::write(
            root.join(".agnosgram/lessons/pitfalls.md"),
            format!("# Pitfalls\n\n{body}"),
        )
        .unwrap();
    }

    #[test]
    fn load_records_returns_valid_records_with_frontmatter_and_location() {
        let root = tmp_store("valid");
        write_pitfalls(
            &root,
            "---\nid: LES-001\ntype: pitfall\nscope: [core, tooling]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nDo not do X.\n",
        );
        let records = load_records(&root);
        let found = records
            .iter()
            .find(|r| r.frontmatter.id == "LES-001")
            .expect("expected LES-001 to be indexed");
        assert_eq!(found.frontmatter.r#type, "pitfall");
        assert_eq!(
            found.frontmatter.scope,
            vec!["core".to_string(), "tooling".to_string()]
        );
        assert_eq!(found.store_rel, "lessons/pitfalls.md");
        assert_eq!(found.body, "Do not do X.");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn load_records_silently_skips_schema_invalid_records() {
        let root = tmp_store("invalid");
        write_pitfalls(&root, "---\nid: bad\ntype: rumor\n---\nbroken record\n");
        let records = load_records(&root);
        assert!(!records.iter().any(|r| r.body == "broken record"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn all_scopes_collects_distinct_scope_tags_across_the_store_sorted() {
        let root = tmp_store("scopes");
        write_pitfalls(
            &root,
            "---\nid: LES-001\ntype: pitfall\nscope: [backend, tooling]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nA.\n\n\
             ---\nid: LES-002\ntype: pitfall\nscope: [auth]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nB.\n",
        );
        assert_eq!(
            all_scopes(&root),
            vec![
                "auth".to_string(),
                "backend".to_string(),
                "tooling".to_string()
            ]
        );
        fs::remove_dir_all(&root).unwrap();
    }
}
