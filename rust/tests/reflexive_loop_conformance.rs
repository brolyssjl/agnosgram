//! Regression test for the human-in-the-loop guarantee in
//! `.agnosgram/decisions/0003-human-in-the-loop-reflexive-loop.md`: `reflect`
//! leaves the entire working tree untouched, and `feedback` writes only
//! under `.agnosgram/meta/`. Ported from
//! `src/commands/reflexive-loop.conformance.test.ts` - deliberately its own
//! file so the guarantee has one obvious place to check, separate from each
//! command's own behavioral tests. See `tests/CONFORMANCE_MAP.md`.
mod common;
use common::{init_store, run_cli, snapshot, TempDir};
use std::fs;
use std::path::PathBuf;

fn setup() -> TempDir {
    let root = TempDir::new("agnos-reflexive");
    init_store(root.path());
    // A host-project file alongside .agnosgram/, so "the working tree" means
    // more than just the store.
    fs::write(
        root.path().join("ROADMAP.md"),
        "# Roadmap\n- [ ] Milestone 5\n",
    )
    .unwrap();
    root
}

#[test]
fn reflect_leaves_the_entire_working_tree_untouched() {
    let root = setup();
    run_cli(
        &[
            "feedback",
            "seed friction so reflect has something to digest",
        ],
        root.path(),
    );
    let before = snapshot(root.path());
    run_cli(&["reflect"], root.path());
    run_cli(&["reflect", "--json", "--months", "12"], root.path());
    let after = snapshot(root.path());
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

#[test]
fn feedback_writes_only_under_agnosgram_meta() {
    let root = setup();
    let before = snapshot(root.path());
    run_cli(&["feedback", "only meta/ should change"], root.path());
    run_cli(
        &["feedback", "a second entry, still only meta/"],
        root.path(),
    );
    let after = snapshot(root.path());

    let changed: Vec<&PathBuf> = after
        .iter()
        .filter(|(k, v)| before.get(*k) != Some(*v))
        .map(|(k, _)| k)
        .collect();
    let removed: Vec<&PathBuf> = before.keys().filter(|k| !after.contains_key(*k)).collect();

    assert!(removed.is_empty(), "feedback must never delete a file");
    assert!(
        !changed.is_empty(),
        "expected feedback to write at least meta/friction.md"
    );
    let expected = PathBuf::from(".agnosgram").join("meta").join("friction.md");
    for rel in changed {
        assert_eq!(
            rel, &expected,
            "feedback wrote outside .agnosgram/meta/: {rel:?}"
        );
    }
}
