//! Ported from `bench/bench.mjs` (Tier-1) and `bench/tier2.mjs` (Tier-2).
//! Both retired `.mjs` scripts measured real GPT tokens via the
//! `gpt-tokenizer` npm devDependency and, for Tier-1, additionally compared
//! our hand-rolled TOON encoder against the "faithful" `@toon-format/toon`
//! reference encoder (also an npm devDependency). Neither is available
//! without npm, and this crate's integration tests are std-only (no
//! dev-dependencies) - see `rust/tests/CONFORMANCE_MAP.md` for the adaptation
//! this required: both real-BPE-token gates now use `estimate_tokens`
//! (`core::tokens`), the same dependency-free proxy the shipped binary
//! itself budgets against, and the drift-vs-faithful-encoder gate is dropped
//! since there is no longer a second, independent TOON implementation to
//! drift from - the "shipped encoder must still win" gate below replaces it
//! (it does not merely restate it: it is the whole guarantee that remains
//! representable).
mod common;
use agnosgram::core::json::{stringify_compact, Value};
use agnosgram::core::tokens::estimate_tokens;
use agnosgram::core::toon::encode_toon;
use common::run_cli;
use std::path::Path;

fn round1(n: f64) -> f64 {
    (n * 10.0).round() / 10.0
}

fn delta_pct(candidate: i64, baseline: i64) -> f64 {
    round1(((candidate - baseline) as f64 / baseline as f64) * 100.0)
}

// ---- Tier 1: payload shapes in isolation --------------------------------

fn payload_record() -> Value {
    let mut v = Value::object();
    v.insert("id", "LES-001");
    v.insert("type", "pitfall");
    v.insert("scope", "core,tooling");
    v.insert("confidence", "high");
    v.insert("last_verified", "2026-07-21");
    v.insert(
        "body",
        "Do not use CommonJS require() in this package - it is ESM (`\"type\": \"module\"`) with verbatimModuleSyntax. `require` fails at runtime.",
    );
    v
}

fn payload_lessons() -> Value {
    let items = (0..12)
        .map(|i| {
            let mut r = Value::object();
            r.insert("id", format!("LES-{:03}", i + 1));
            r.insert("type", if i % 2 == 1 { "pitfall" } else { "convention" });
            r.insert("scope", "backend");
            r.insert("confidence", "high");
            r.insert("last_verified", "2026-07-21");
            r
        })
        .collect();
    Value::Array(items)
}

struct TierOneRow {
    json: i64,
    toon: i64,
    delta_pct: f64,
}

fn measure_tier1(payload: &Value) -> TierOneRow {
    let json = estimate_tokens(&stringify_compact(payload));
    let toon = estimate_tokens(&encode_toon(payload));
    TierOneRow { json, toon, delta_pct: delta_pct(toon, json) }
}

/// Gate (adapted from bench.mjs Gates 1+4, which both asserted a >=15% win
/// on the uniform `lessons` array - Gate 1 against the faithful encoder,
/// Gate 4 against the shipped one; with the faithful encoder gone, only the
/// shipped-encoder guarantee is representable). Measured against
/// `estimate_tokens` (2026-08-22, this fixture set): -46.5% on the uniform
/// `lessons` array (json=155, toon=83). The threshold below (-15%) matches
/// the original bar and leaves wide headroom under the measured value while
/// still gating the core TOON value proposition.
#[test]
fn toon_saves_tokens_on_the_uniform_lessons_array() {
    let row = measure_tier1(&payload_lessons());
    assert!(
        row.delta_pct <= -15.0,
        "expected TOON to save >=15% on the uniform `lessons` array, got {}% (json={}, toon={})",
        row.delta_pct,
        row.json,
        row.toon
    );
}

/// Gate (adapted from bench.mjs Gate 2 - small-object honesty, restated
/// against our own shipped encoder since the faithful reference encoder is
/// gone). A small, non-uniform, punctuation-heavy `record` object must not
/// show a net TOON win - if it ever does, the "TOON is opt-in for tabular
/// data only, never a blanket default" decision needs revisiting. Measured
/// (2026-08-22): +13.3% (json=45, toon=51) - TOON costs more tokens than
/// compact JSON here under the proxy estimator too, same direction as the
/// original faithful-encoder finding. The threshold is exactly 0, matching
/// the original's zero-headroom bar.
#[test]
fn toon_does_not_win_on_a_small_non_uniform_record() {
    let row = measure_tier1(&payload_record());
    assert!(
        row.delta_pct >= 0.0,
        "expected TOON to not show a net win on the small `record` object, got {}% (json={}, toon={})",
        row.delta_pct,
        row.json,
        row.toon
    );
}

// ---- Tier 2: real CLI output against fixture stores ----------------------

// The original tier2.mjs also measured `advise-prep` (`["advise", "plan.md"]`)
// across all three formats, alongside these three - but never gated it: it
// is a prose prompt wrapped in a single JSON/TOON string field, where format
// cannot help, by design (see the savings-gate doc comment below). Omitted
// here since nothing in this file reads that measurement.
const TABULAR_TASKS: &[(&str, &[&str])] = &[
    ("session-start-pack", &["pack"]),
    ("scoped-pack", &["pack", "--scope", "core"]),
    ("show", &["show", "core"]),
];

fn fixture_root(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("bench").join("fixtures").join(name)
}

/// Runs one CLI task against a fixture store, in the given format. None of
/// tier2's tasks are expected to exit 1 against these fixtures (`show core`
/// matches in both stores) - a non-zero exit or empty stdout is a
/// measurement failure, not a "0 tokens" row.
fn measure(store_root: &Path, args: &[&str], format: &str) -> String {
    let full_args: Vec<&str> = if format == "human" {
        args.to_vec()
    } else {
        let mut a = args.to_vec();
        a.push("--format");
        a.push(format);
        a
    };
    let res = run_cli(&full_args, store_root);
    assert_eq!(
        res.status, 0,
        "CLI exited {} for {full_args:?} in {store_root:?}: {}",
        res.status, res.stderr
    );
    assert!(
        !res.stdout.trim().is_empty(),
        "CLI produced empty stdout for {full_args:?} in {store_root:?} - a task that isn't expected to fail must not measure as \"0 tokens\""
    );
    res.stdout
}

/// Gate (adapted from tier2.mjs Gate 1, real-BPE savings vs compact JSON,
/// now measured with `estimate_tokens`). Excludes `advise-prep`, a prose
/// prompt wrapped in a single string field where format cannot help, by
/// design. Measured (2026-08-22, this fixture set) against the compact-JSON
/// baseline: store-small -15.8%/-10.3%/-14.6% (session-start-pack/
/// scoped-pack/show), store-large -23.3%/-18.1%/-20.7%. The tightest is
/// store-small/scoped-pack at -10.3%; the threshold below (-5%) keeps ~5
/// points of headroom under it while still gating a real, meaningful win.
#[test]
fn toon_saves_tokens_vs_compact_json_for_tabular_cli_output() {
    for store in ["store-small", "store-large"] {
        let root = fixture_root(store);
        for &(task, args) in TABULAR_TASKS {
            let json_stdout = measure(&root, args, "json");
            let toon_stdout = measure(&root, args, "toon");

            let parsed = agnosgram::core::json::parse(&json_stdout)
                .unwrap_or_else(|e| panic!("CLI's own --format json output failed to parse for {store}/{task}: {e}"));
            let compact_json_tokens = estimate_tokens(&stringify_compact(&parsed));
            let toon_tokens = estimate_tokens(&toon_stdout);
            assert!(compact_json_tokens > 0, "measured 0 tokens for {store}/{task}/json - treating as a measurement failure");
            assert!(toon_tokens > 0, "measured 0 tokens for {store}/{task}/toon - treating as a measurement failure");

            let pct = delta_pct(toon_tokens, compact_json_tokens);
            assert!(
                pct <= -5.0,
                "expected TOON to save >=5% vs compact JSON on {store}/{task}, got {pct}% (json={compact_json_tokens}, toon={toon_tokens})"
            );
        }
    }
}

/// Gate (adapted from tier2.mjs Gate 2 - pack's whole-record greedy budget
/// must actually cap real output growth). The original compared the
/// Markdown output's real BPE token count against `DEFAULT_PACK_BUDGET`
/// with a 1.4x tolerance, because `estimate_tokens` under-counts real GPT
/// tokenization by ~35-40% on Markdown-heavy content - a real-vs-proxy gap.
/// With no real tokenizer available, this now measures the Markdown output
/// with the *same* proxy `pack` budgets against, so the gap that motivated
/// 1.4x is gone; the tolerance is tightened to 1.1x (ceiling 2200 against a
/// 2000 budget). Measured (2026-08-22): store-small 331/253 tokens
/// (session-start-pack/scoped-pack), store-large 1874/518 - the tightest,
/// store-large/session-start-pack at 1874, still leaves ~326 tokens under
/// the 1.1x ceiling. The remaining headroom still catches a real
/// budget-accounting-vs-emission mismatch (the actual thing this gate
/// guards against), it just no longer needs to absorb a real-vs-proxy
/// tokenizer gap that doesn't exist once both sides use the same estimator.
#[test]
fn packs_budget_caps_real_output_growth() {
    const BUDGET_TOLERANCE: f64 = 1.1;
    let ceiling = (agnosgram::commands::pack::DEFAULT_PACK_BUDGET as f64 * BUDGET_TOLERANCE).round() as i64;
    for store in ["store-small", "store-large"] {
        let root = fixture_root(store);
        for &(task, args) in TABULAR_TASKS.iter().filter(|(t, _)| *t != "show") {
            let human = measure(&root, args, "human");
            let tokens = estimate_tokens(&human);
            assert!(
                tokens <= ceiling,
                "expected {store}/{task} to stay under ~{ceiling} tokens (budget {} x {BUDGET_TOLERANCE}), got {tokens}",
                agnosgram::commands::pack::DEFAULT_PACK_BUDGET
            );
        }
    }
}
