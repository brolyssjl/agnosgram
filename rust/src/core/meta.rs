//! Port of `src/core/meta.ts`: `.agnosgram/meta/` - tool-friction feedback,
//! strictly separate from host-project memory (lessons/, decisions/,
//! context/). This is an additive extension to the frozen format (DEC-0002):
//! a store without `meta/` is fully valid, and nothing here changes the
//! meaning of anything else. See
//! `.agnosgram/decisions/0003-human-in-the-loop-reflexive-loop.md`.
//!
//! Friction records reuse the record shape (id, type, scope, confidence,
//! created, last_verified, source, supersedes?) and `frontmatter.rs`'s
//! field-by-field validation, just against a different type enum - so the
//! frozen lessons/decisions validator (`KNOWN_TYPES`) never has to know
//! `meta/` exists.

use super::frontmatter::{extract_records, validate_record, RawRecord, ValidatedRecord};
use super::records::StoreRecord;
use super::store::read_store;
use std::path::Path;

/// The only friction record type today; kept as a list so `validate_record`
/// can share its `allowed_types` shape with the frozen lessons/decisions enum.
pub const KNOWN_META_TYPES: [&str; 1] = ["friction"];
pub const FRICTION_TYPE: &str = "friction";

/// Store-relative path of the single friction-entries file.
pub const FRICTION_FILE: &str = "meta/friction.md";

/// Validate one friction record's frontmatter (id/type/scope/confidence/dates/source).
pub fn validate_friction_record(raw: &RawRecord) -> ValidatedRecord {
    validate_record(raw, &KNOWN_META_TYPES)
}

/// Every schema-valid friction record in `meta/friction.md`, if it exists.
pub fn load_friction_records(root: &Path) -> Vec<StoreRecord> {
    let mut out = Vec::new();
    for file in read_store(root) {
        if file.store_rel != FRICTION_FILE {
            continue;
        }
        for raw in extract_records(&file.text) {
            let validated = validate_friction_record(&raw);
            let Some(frontmatter) = validated.frontmatter else {
                continue;
            };
            out.push(StoreRecord {
                frontmatter,
                body: raw.body,
                file: file.rel.clone(),
                store_rel: file.store_rel.clone(),
                line: raw.line,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_root(name: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("agnos-meta-rs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".agnosgram/meta")).unwrap();
        root
    }

    #[test]
    fn known_meta_types_only_allows_friction_today() {
        assert_eq!(KNOWN_META_TYPES, ["friction"]);
    }

    #[test]
    fn validate_friction_record_accepts_a_well_formed_friction_entry() {
        let text = "---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: meta/friction.md\n---\nThe doctor error message was confusing.\n";
        let recs = extract_records(text);
        let validated = validate_friction_record(&recs[0]);
        assert!(validated.issues.is_empty());
        let fm = validated.frontmatter.expect("expected valid frontmatter");
        assert_eq!(fm.id, "FRI-001");
        assert_eq!(fm.r#type, "friction");
    }

    #[test]
    fn validate_friction_record_rejects_a_lessons_decisions_type() {
        let text = "---\nid: FRI-001\ntype: pitfall\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: meta/friction.md\n---\nWrong type for this namespace.\n";
        let recs = extract_records(text);
        let validated = validate_friction_record(&recs[0]);
        assert!(validated.frontmatter.is_none());
        assert!(validated.issues.iter().any(|i| i.code == "type.unknown"));
    }

    #[test]
    fn load_friction_records_reads_only_meta_friction_md_and_skips_invalid_entries() {
        let root = tmp_root("load");
        fs::write(
            root.join(".agnosgram/meta/friction.md"),
            format!(
                "# Friction\n\n---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: {FRICTION_FILE}\n---\nConfusing error message.\n\n---\nid: bad\ntype: friction\n---\nbroken\n"
            ),
        )
        .unwrap();
        let records = load_friction_records(&root);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].frontmatter.id, "FRI-001");
        fs::remove_dir_all(&root).unwrap();
    }
}
