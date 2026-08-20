//! Port of `src/commands/pack.ts`: a token-budgeted context bundle for the
//! start of an agent session: status + lessons (+ decisions when scoped),
//! most important first, dropped whole-record when the budget runs out. This
//! is the agent hot path, so the default output is the human-readable
//! Markdown bundle itself, not JSON.
//!
//! Priority order (also the greedy-drop order): header, `state/status.md`
//! verbatim (always kept - never dropped), lessons (pitfalls then
//! conventions, each sorted confidence desc / last_verified desc / id asc),
//! decisions (only included when `--scope` is given). Context files are
//! never packed - they are meant to be read directly by an agent working in
//! that area, not bundled into every session start.

use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::commands::show::flat_record_to_value;
use crate::core::args::{parse_cli_args, ArgsConfig, OptionDef};
use crate::core::config::load_config;
use crate::core::json::Value;
use crate::core::output::{print_structured, UserError};
use crate::core::paths::{find_project_root, has_store, memory_dir};
use crate::core::records::{
    compare_records, load_records, matches_scope, render_record_block, to_flat_record, StoreRecord,
};
use crate::core::serialize::{resolve_format, FormatFlags, ResolvedFormat};
use crate::core::tokens::estimate_tokens;

/// Output-schema version for `pack --json` (independent of the store format version).
const PACK_SCHEMA_VERSION: i64 = 1;
/// Default token budget `pack` targets when neither `--budget` nor config's
/// `pack_budget` is given.
pub const DEFAULT_PACK_BUDGET: i64 = 2000;

/// At most this many omitted records are named in the "Omitted (budget)"
/// footer; the rest are summarized as "...and N more" so the footer itself
/// cannot grow without bound on a large store.
const FOOTER_SHOW: usize = 5;

fn type_heading(t: &str) -> String {
    match t {
        "pitfall" => "## Pitfalls".to_string(),
        "convention" => "## Conventions".to_string(),
        "decision" => "## Decisions".to_string(),
        other => format!("## {other}"),
    }
}

/// Records of one type, scope-filtered (when `scope` is given) and sorted by priority.
fn by_type<'a>(records: &'a [StoreRecord], ty: &str, scope: Option<&str>) -> Vec<&'a StoreRecord> {
    let mut pool: Vec<&StoreRecord> = records
        .iter()
        .filter(|r| r.frontmatter.r#type == ty)
        .collect();
    if let Some(s) = scope {
        pool.retain(|r| matches_scope(r, s));
    }
    pool.sort_by(|a, b| compare_records(a, b));
    pool
}

fn render_omitted_footer(omitted: &[&StoreRecord]) -> String {
    if omitted.is_empty() {
        return String::new();
    }
    let mut shown: Vec<String> = omitted
        .iter()
        .take(FOOTER_SHOW)
        .map(|r| format!("- {} ({})", r.frontmatter.id, r.frontmatter.r#type))
        .collect();
    let rest = omitted.len() - FOOTER_SHOW.min(omitted.len());
    if rest > 0 {
        shown.push(format!("- ...and {rest} more"));
    }
    format!("## Omitted (budget)\n\n{}", shown.join("\n"))
}

/// Worst-case token cost of the omitted footer over any subset of
/// `candidates`. The footer is capped (`FOOTER_SHOW` entries + an "...and N
/// more" tail), but its exact size still depends on which records end up
/// omitted - which is what the greedy loop below is deciding. Reserving this
/// upper bound up front (built from the single heaviest "id (type)" line,
/// repeated, with the full candidate count for the tail) means the loop
/// never has to overshoot to make room for the footer later: any real
/// omitted subset is shorter-or-equal in both id/type text and count, and
/// `estimate_tokens` is monotonic in chars and words, so its real cost never
/// exceeds this reserve.
fn max_footer_reserve(candidates: &[&StoreRecord]) -> i64 {
    if candidates.is_empty() {
        return 0;
    }
    let heaviest: &StoreRecord = candidates
        .iter()
        .copied()
        .reduce(|a, b| {
            let la = a.frontmatter.id.len() + a.frontmatter.r#type.len();
            let lb = b.frontmatter.id.len() + b.frontmatter.r#type.len();
            if la >= lb {
                a
            } else {
                b
            }
        })
        .expect("non-empty candidates");
    let worst_case: Vec<&StoreRecord> = std::iter::repeat_n(heaviest, candidates.len()).collect();
    estimate_tokens(&render_omitted_footer(&worst_case))
}

/// Greedily admit candidates in priority order, simulating the *actual*
/// section assembly (type headings inserted on change, "\n\n" joiners) so
/// the budget check reflects what will really be rendered - not just the
/// sum of bare record bodies. `footer_reserve` is subtracted from the
/// ceiling so there is always room left for the omitted-footer this loop's
/// own output may require.
fn simulate_pack<'a>(
    candidates: &[&'a StoreRecord],
    block_text: &[String],
    header: &str,
    status_block: &str,
    budget: i64,
    footer_reserve: i64,
) -> (Vec<&'a StoreRecord>, Vec<&'a StoreRecord>, Vec<String>) {
    let mut included: Vec<&StoreRecord> = Vec::new();
    let mut omitted: Vec<&StoreRecord> = Vec::new();
    let mut sections: Vec<String> = vec![header.to_string(), status_block.to_string()];
    let mut current_type = String::new();

    for (i, rec) in candidates.iter().enumerate() {
        let mut trial = sections.clone();
        if rec.frontmatter.r#type != current_type {
            trial.push(type_heading(&rec.frontmatter.r#type));
        }
        trial.push(block_text[i].clone());
        let cost = estimate_tokens(&trial.join("\n\n"));
        if cost + footer_reserve <= budget {
            sections = trial;
            current_type = rec.frontmatter.r#type.clone();
            included.push(rec);
        } else {
            omitted.push(rec);
        }
    }

    (included, omitted, sections)
}

struct PackResult<'a> {
    budget: i64,
    tokens: i64,
    status: String,
    markdown: String,
    included: Vec<&'a StoreRecord>,
    omitted: Vec<&'a StoreRecord>,
}

fn build_pack<'a>(
    root: &Path,
    budget: i64,
    scope: Option<&str>,
    records: &'a [StoreRecord],
) -> Result<PackResult<'a>, UserError> {
    let status_path = memory_dir(root).join("state").join("status.md");
    if !status_path.exists() {
        return Err(UserError::new(
            "Missing .agnosgram/state/status.md. Run `agnosgram doctor` to see what else is missing, \
             or `agnosgram init` to scaffold a fresh store.",
        ));
    }
    let status = fs::read_to_string(&status_path)
        .map_err(|e| UserError::new(e.to_string()))?
        .trim()
        .to_string();

    let pitfalls = by_type(records, "pitfall", scope);
    let conventions = by_type(records, "convention", scope);
    // Decisions are only ever candidates when the pack is scoped - an
    // unscoped pack stays lean for every session; decisions are
    // architectural detail an agent needs only when it is about to work in
    // that scoped area.
    let decisions = if scope.is_some() {
        by_type(records, "decision", scope)
    } else {
        Vec::new()
    };

    let mut candidates: Vec<&StoreRecord> = Vec::new();
    candidates.extend(pitfalls);
    candidates.extend(conventions);
    candidates.extend(decisions);

    let header = match scope {
        Some(s) => format!("# Agnosgram pack (scope: {s})\n"),
        None => "# Agnosgram pack\n".to_string(),
    };
    let status_block = format!("## state/status.md\n\n{status}\n");

    // Render each record block exactly once; the cached string funds both
    // the budget simulation and the final assembly below.
    let block_text: Vec<String> = candidates.iter().map(|r| render_record_block(r)).collect();

    // First pass: no footer reserve. If everything fits, there is no footer
    // to account for, so this is also the final answer - keeps the common
    // case (a store that fits within budget) from losing headroom to a
    // footer it will never render.
    let (mut included, mut omitted, mut sections) =
        simulate_pack(&candidates, &block_text, &header, &status_block, budget, 0);
    if !omitted.is_empty() {
        let reserve = max_footer_reserve(&candidates);
        let sim = simulate_pack(
            &candidates,
            &block_text,
            &header,
            &status_block,
            budget,
            reserve,
        );
        included = sim.0;
        omitted = sim.1;
        sections = sim.2;
    }

    if !omitted.is_empty() {
        sections.push(render_omitted_footer(&omitted));
    }

    let markdown = format!("{}\n", sections.join("\n\n").trim_end());
    let tokens = estimate_tokens(&markdown);

    Ok(PackResult {
        budget,
        tokens,
        status,
        markdown,
        included,
        omitted,
    })
}

/// Mimics JS `Number.parseInt(s, 10)`: skip leading whitespace, an optional
/// sign, then as many leading digits as are present (ignoring any trailing
/// non-digit content). `None` when no digits are found (JS `NaN`).
fn parse_int_like_js(s: &str) -> Option<i64> {
    let mut chars = s.chars().peekable();
    while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
        chars.next();
    }
    let mut negative = false;
    match chars.peek() {
        Some('+') => {
            chars.next();
        }
        Some('-') => {
            negative = true;
            chars.next();
        }
        _ => {}
    }
    let mut digits = String::new();
    while matches!(chars.peek(), Some(c) if c.is_ascii_digit()) {
        digits.push(chars.next().unwrap());
    }
    if digits.is_empty() {
        return None;
    }
    digits
        .parse::<i64>()
        .ok()
        .map(|n| if negative { -n } else { n })
}

pub fn run(argv: Vec<String>) -> Result<(), UserError> {
    let cfg = ArgsConfig::new(false)
        .option("scope", OptionDef::string())
        .option("budget", OptionDef::string())
        .option("json", OptionDef::boolean(false))
        .option("format", OptionDef::string());
    let parsed = parse_cli_args(&argv, &cfg)?;

    let format = resolve_format(&FormatFlags {
        format: parsed.str("format").map(|s| s.to_string()),
        json: parsed.bool("json"),
    })?;

    let cwd = env::current_dir().map_err(|e| UserError::new(e.to_string()))?;
    let root = find_project_root(&cwd);
    if !has_store(&root) {
        return Err(UserError::new(
            "No .agnosgram/ store found. Run `agnosgram init` first.",
        ));
    }

    let mut budget = DEFAULT_PACK_BUDGET;
    let config = load_config(&root).map_err(|e| UserError::new(e.to_string()))?;
    if let Some(pb) = config.pack_budget {
        budget = pb;
    }
    if let Some(raw) = parsed.str("budget") {
        match parse_int_like_js(raw) {
            Some(n) if n > 0 => budget = n,
            _ => {
                return Err(UserError::new(format!(
                    "--budget must be a positive integer, got \"{raw}\""
                )));
            }
        }
    }

    let scope = parsed.str("scope").map(str::trim).filter(|s| !s.is_empty());

    let records = load_records(&root);
    let result = build_pack(&root, budget, scope, &records)?;

    match format {
        ResolvedFormat::Human => {
            std::io::stdout()
                .write_all(result.markdown.as_bytes())
                .map_err(|e| UserError::new(e.to_string()))?;
        }
        ResolvedFormat::Structured(f) => {
            let mut v = Value::object();
            v.insert("version", PACK_SCHEMA_VERSION);
            v.insert("budget", result.budget);
            v.insert("tokens", result.tokens);
            v.insert("status", result.status.clone());
            v.insert(
                "records",
                Value::Array(
                    result
                        .included
                        .iter()
                        .map(|r| flat_record_to_value(&to_flat_record(r)))
                        .collect(),
                ),
            );
            v.insert(
                "omitted",
                Value::Array(
                    result
                        .omitted
                        .iter()
                        .map(|r| Value::from(r.frontmatter.id.clone()))
                        .collect(),
                ),
            );
            print_structured(&v, f);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_int_like_js_parses_leading_digits_and_ignores_trailing_garbage() {
        assert_eq!(parse_int_like_js("60"), Some(60));
        assert_eq!(parse_int_like_js("-1"), Some(-1));
        assert_eq!(parse_int_like_js("  42  "), Some(42));
        assert_eq!(parse_int_like_js("10abc"), Some(10));
        assert_eq!(parse_int_like_js("abc"), None);
        assert_eq!(parse_int_like_js(""), None);
    }

    #[test]
    fn render_omitted_footer_caps_the_listed_entries_and_summarizes_the_rest() {
        use crate::core::frontmatter::{extract_records, validate_record, KNOWN_TYPES};

        let mut owned: Vec<StoreRecord> = Vec::new();
        for i in 0..7 {
            let text = format!(
                "---\nid: LES-{i:03}\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nBody {i}.\n"
            );
            let raw = extract_records(&text).remove(0);
            let validated = validate_record(&raw, &KNOWN_TYPES);
            owned.push(StoreRecord {
                frontmatter: validated.frontmatter.expect("valid"),
                body: raw.body,
                file: "lessons/pitfalls.md".to_string(),
                store_rel: "lessons/pitfalls.md".to_string(),
                line: raw.line,
            });
        }
        let refs: Vec<&StoreRecord> = owned.iter().collect();
        let footer = render_omitted_footer(&refs);
        assert!(footer.starts_with("## Omitted (budget)"));
        assert!(footer.contains("...and 2 more"));
    }

    #[test]
    fn render_omitted_footer_is_empty_for_no_omissions() {
        assert_eq!(render_omitted_footer(&[]), "");
    }
}
