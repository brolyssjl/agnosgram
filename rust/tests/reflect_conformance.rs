//! Ported from `src/commands/reflect.conformance.test.ts`. See
//! `tests/CONFORMANCE_MAP.md`.
mod common;
use common::{init_store, run_cli, snapshot, write_store_file, Json, TempDir};
use std::fs;

fn setup() -> TempDir {
    let root = TempDir::new("agnos-reflect");
    init_store(root.path());
    root
}

fn write_friction(root: &TempDir, body: &str) {
    write_store_file(
        root.path(),
        "meta/friction.md",
        &format!("# Friction\n\n{body}"),
    );
}

#[test]
fn reflect_emits_a_prompt_naming_friction_journal_months_and_the_roadmap_rule() {
    let root = setup();
    let res = run_cli(&["reflect"], root.path());
    assert_eq!(res.status, 0);
    assert!(res.stdout.contains("reflect task"));
    assert!(res.stdout.contains("meta/friction.md"));
    assert!(
        res.stdout.contains("ROADMAP.md is owner-edited") || res.stdout.contains("owner-edited")
    );
    assert!(
        res.stdout.contains("never to `ROADMAP.md` directly")
            || res.stdout.contains("never applied automatically")
            || res.stdout.contains("never disposes")
    );
}

#[test]
fn reflect_includes_captured_friction_entries_in_its_digest() {
    let root = setup();
    write_friction(
        &root,
        "---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: meta/friction.md\n---\nThe doctor error message was confusing.\n",
    );
    let res = run_cli(&["reflect"], root.path());
    assert!(res.stdout.contains("FRI-001"));
    assert!(res.stdout.contains("doctor error message was confusing"));
}

#[test]
fn reflect_json_returns_a_versioned_envelope_with_friction_and_journal_coverage() {
    let root = setup();
    write_friction(
        &root,
        "---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: meta/friction.md\n---\nSomething was confusing.\n",
    );
    let res = run_cli(&["reflect", "--json"], root.path());
    let parsed = Json::parse(&res.stdout);
    assert_eq!(
        parsed.get("agnosgram_reflect").and_then(Json::as_f64),
        Some(1.0)
    );
    let ids = parsed.get("friction_ids").and_then(Json::as_array).unwrap();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0].as_str(), Some("FRI-001"));
    assert_eq!(
        parsed.get("friction_count").and_then(Json::as_f64),
        Some(1.0)
    );
    assert!(parsed
        .get("journal_months")
        .and_then(Json::as_array)
        .is_some());
    assert!(parsed.get("prompt").and_then(Json::as_str).is_some());
}

#[test]
fn reflect_months_limits_how_many_recent_journal_months_are_listed() {
    let root = setup();
    let journal_dir = root.path().join(".agnosgram/journal");
    for m in ["2026-01", "2026-02", "2026-03", "2026-04"] {
        fs::write(
            journal_dir.join(format!("{m}.md")),
            format!("# Journal - {m}\n"),
        )
        .unwrap();
    }
    let mut all_months: Vec<String> = fs::read_dir(&journal_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.len() == 10 && n.ends_with(".md"))
        .map(|n| n[..7].to_string())
        .collect();
    all_months.sort();
    let expected = &all_months[all_months.len() - 2..];

    let res = run_cli(&["reflect", "--months", "2", "--json"], root.path());
    let parsed = Json::parse(&res.stdout);
    let mut got: Vec<String> = parsed
        .get("journal_months")
        .and_then(Json::as_array)
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    got.sort();
    assert_eq!(got, expected);
}

#[test]
fn reflect_rejects_a_non_positive_months() {
    let root = setup();
    let res = run_cli(&["reflect", "--months", "0"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("--months must be a positive integer"));
}

#[test]
fn reflect_rejects_a_non_integer_months_instead_of_silently_truncating() {
    let root = setup();
    let res = run_cli(&["reflect", "--months", "2.5"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("--months must be a positive integer"));
}

#[test]
fn reflect_rejects_a_months_with_trailing_junk_instead_of_silently_parsing_a_prefix() {
    let root = setup();
    let res = run_cli(&["reflect", "--months", "3abc"], root.path());
    assert_ne!(res.status, 0);
    assert!(res.stderr.contains("--months must be a positive integer"));
}

#[test]
fn reflect_months_dash_1_gives_the_existing_validation_error_not_a_raw_parseargs_crash() {
    let root = setup();
    let res = run_cli(&["reflect", "--months", "-1"], root.path());
    assert_ne!(res.status, 0);
    assert!(res
        .stderr
        .contains("--months must be a positive integer, got \"-1\""));
}

#[test]
fn reflect_performs_no_writes_to_the_repo_read_only() {
    let root = setup();
    write_friction(
        &root,
        "---\nid: FRI-001\ntype: friction\nscope: [cli]\nconfidence: medium\ncreated: 2026-07-30\nlast_verified: 2026-07-30\nsource: meta/friction.md\n---\nSome friction.\n",
    );
    let before = snapshot(&root.path().join(".agnosgram"));
    run_cli(&["reflect"], root.path());
    run_cli(&["reflect", "--json"], root.path());
    let after = snapshot(&root.path().join(".agnosgram"));
    assert_eq!(
        after.keys().collect::<Vec<_>>(),
        before.keys().collect::<Vec<_>>(),
        "reflect must not create or delete any file"
    );
    for (file, mtime) in &before {
        assert_eq!(
            after.get(file),
            Some(mtime),
            "reflect must not modify {file:?}"
        );
    }
}
