//! Port of `src/commands/advise.ts`.
//!
//! `agnosgram advise <plan-path>` - the landmine-catcher. Cross-checks a plan
//! (or spec) against everything the store has learned (`lessons/`,
//! `decisions/`) and flags contradictions before the plan becomes code.
//!
//! Two-step pattern, same shape as `distill` (see `commands/distill.rs`):
//! step 1 emits a prompt for whatever agent is present; step 2 (`--validate`)
//! mechanically checks the JSON report the agent wrote back, against the
//! pinned `agnosgram_advise` schema (a public contract shared with Gate - see
//! docs/advise.md). The CLI itself never calls an LLM and never judges
//! content, only shape and provenance.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::args::{parse_cli_args, ArgsConfig, OptionDef};
use crate::core::dates::is_valid_iso_date;
use crate::core::json::{self, Value};
use crate::core::lint::{
    distinct_untrusted_sources, render_untrusted_banner, scan_untrusted, warn_untrusted_hits,
    UntrustedHit, UNTRUSTED_DATA_NOTE,
};
use crate::core::output::{info, print_structured, UserError};
use crate::core::paths::{find_project_root, has_store};
use crate::core::records::{load_records, StoreRecord};
use crate::core::serialize::{resolve_format, FormatFlags, ResolvedFormat};

const KNOWN_KINDS: [&str; 2] = ["empirical", "normative"];
const KNOWN_SEVERITIES: [&str; 2] = ["blocker", "caution"];
const KNOWN_CONFIDENCE: [&str; 3] = ["low", "medium", "high"];

/// Current schema version of the pinned `agnosgram_advise` report contract.
pub const ADVISE_SCHEMA_VERSION: i64 = 1;

fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn digest_table(records: &[StoreRecord]) -> String {
    let header =
        "| id | type | scope | confidence | last_verified | excerpt |\n|---|---|---|---|---|---|";
    let mut sorted: Vec<&StoreRecord> = records.iter().collect();
    sorted.sort_by(|a, b| a.frontmatter.id.cmp(&b.frontmatter.id));
    if sorted.is_empty() {
        return format!("{header}\n_(store has no records yet)_");
    }
    let rows: Vec<String> = sorted
        .iter()
        .map(|r| {
            let normalized = normalize_ws(&r.body);
            // TS `.slice(0, 80)` counts UTF-16 code units; from_utf16_lossy
            // matches Node's U+FFFD for a slice-split surrogate pair.
            let units: Vec<u16> = normalized.encode_utf16().collect();
            let truncated = units.len() > 80;
            let snippet =
                String::from_utf16_lossy(&units[..units.len().min(80)]).replace('|', "\\|");
            format!(
                "| {} | {} | {} | {} | {} | {}{} |",
                r.frontmatter.id,
                r.frontmatter.r#type,
                r.frontmatter.scope.join(","),
                r.frontmatter.confidence,
                r.frontmatter.last_verified,
                snippet,
                if truncated { "..." } else { "" }
            )
        })
        .collect();
    format!("{header}\n{}", rows.join("\n"))
}

fn build_prompt(root: &Path, plan_path: &str, out_path: &str) -> String {
    let records = load_records(root);
    let digest = digest_table(&records);
    // Record bodies feed the digest and the plan is handed to the agent
    // wholesale - both are manipulable text, so scan and warn-and-mark,
    // never drop (agnosgram#39). The plan path is resolved the way the
    // agent will read it: as given, relative to the caller's cwd.
    let mut hits: Vec<UntrustedHit> = Vec::new();
    for r in &records {
        hits.extend(scan_untrusted(&r.body, &r.file, Some(&r.frontmatter.id)));
    }
    if let Ok(text) = std::fs::read_to_string(plan_path) {
        hits.extend(scan_untrusted(&text, plan_path, None));
    }
    warn_untrusted_hits("advise", &hits);
    let banner_block = if hits.is_empty() {
        String::new()
    } else {
        format!(
            "{}\n\n",
            render_untrusted_banner(&distinct_untrusted_sources(&hits))
        )
    };
    format!(
        "# Agnosgram advise task\n\
\n\
{banner_block}\
{trust_note}\n\
\n\
You are reviewing a plan for contradictions against this project's memory\n\
store at `.agnosgram/`. Do NOT invent facts; only flag a contradiction when a\n\
stored record actually conflicts with something the plan says or assumes.\n\
\n\
## Plan to review\n\
`{plan_path}`\n\
\n\
## Digest: every lesson and decision currently in the store\n\
{digest}\n\
\n\
## Precedence rule (verbatim - apply exactly, do not reinterpret)\n\
- Normative contradiction (the plan proposes a different convention/policy than\n\
\x20\x20a stored decision or convention): the spec (the plan) wins by default; record\n\
\x20\x20the exception only if a stored record explicitly permits deviating.\n\
- Empirical contradiction (the plan assumes a fact a stored pitfall/lesson\n\
\x20\x20directly contradicts): the contradiction wins - flag it; a human arbitrates.\n\
\n\
## Output rules\n\
1. Write a JSON report to `{out_path}` (this exact path, or the `--out` path\n\
\x20\x20\x20you were given) matching this schema exactly:\n\
\x20\x20\x20```json\n\
\x20\x20\x20{{\n\
\x20\x20\x20\x20\x20\"agnosgram_advise\": 1,\n\
\x20\x20\x20\x20\x20\"plan\": \"{plan_path}\",\n\
\x20\x20\x20\x20\x20\"generated\": \"YYYY-MM-DD\",\n\
\x20\x20\x20\x20\x20\"checked_ids\": [\"LES-001\", \"...\"],\n\
\x20\x20\x20\x20\x20\"contradictions\": [{{\n\
\x20\x20\x20\x20\x20\x20\x20\"record_id\": \"LES-002\",\n\
\x20\x20\x20\x20\x20\x20\x20\"kind\": \"empirical | normative\",\n\
\x20\x20\x20\x20\x20\x20\x20\"severity\": \"blocker | caution\",\n\
\x20\x20\x20\x20\x20\x20\x20\"plan_excerpt\": \"...\",\n\
\x20\x20\x20\x20\x20\x20\x20\"record_excerpt\": \"...\",\n\
\x20\x20\x20\x20\x20\x20\x20\"confidence\": \"high\",\n\
\x20\x20\x20\x20\x20\x20\x20\"last_verified\": \"YYYY-MM-DD\",\n\
\x20\x20\x20\x20\x20\x20\x20\"explanation\": \"one sentence\"\n\
\x20\x20\x20\x20\x20}}],\n\
\x20\x20\x20\x20\x20\"clear\": false\n\
\x20\x20\x20}}\n\
\x20\x20\x20```\n\
2. `checked_ids` lists every record id from the digest above that you actually\n\
\x20\x20\x20considered - check all of them, not a sample.\n\
3. For each contradiction, `confidence` and `last_verified` must copy the\n\
\x20\x20\x20cited record's actual frontmatter exactly (this is checked mechanically).\n\
4. `plan_excerpt` must be a real substring of the plan file; `record_excerpt`\n\
\x20\x20\x20must be a real substring of the cited record's body. Do not paraphrase them.\n\
5. `clear` is `true` only when `contradictions` is empty or contains no\n\
\x20\x20\x20`\"severity\": \"blocker\"` entries.\n\
6. Keep `explanation` to one tight sentence per contradiction.\n\
\n\
## Validate your result (mechanical, no LLM)\n\
Run this and fix anything it reports before finishing:\n\
`agnosgram advise --validate {out_path}`\n",
        trust_note = UNTRUSTED_DATA_NOTE,
    )
}

#[derive(Debug)]
struct ValidateIssue {
    level: &'static str,
    code: String,
    message: String,
}

#[derive(Debug)]
struct ValidateResult {
    file: String,
    ok: bool,
    errors: usize,
    warnings: usize,
    issues: Vec<ValidateIssue>,
    report: Value,
    /// Mechanically-derived clearness: true only when no blocker-severity
    /// contradiction was validated AND the report's own `clear` field says
    /// `true`. Never the report's self-declared `clear` alone.
    clear: bool,
}

fn validate_result_to_json(r: &ValidateResult) -> Value {
    let mut out = Value::object();
    out.insert("file", r.file.clone());
    out.insert("ok", r.ok);
    out.insert("errors", r.errors);
    out.insert("warnings", r.warnings);
    let mut issues = Value::array();
    for i in &r.issues {
        let mut o = Value::object();
        o.insert("level", i.level);
        o.insert("code", i.code.clone());
        o.insert("message", i.message.clone());
        issues.push(o);
    }
    out.insert("issues", issues);
    out.insert("report", r.report.clone());
    out.insert("clear", r.clear);
    out
}

/// Compact `JSON.stringify(value)` for error-message interpolation only -
/// not part of the byte-exact literal surface, since the value it renders is
/// whatever malformed content a report happens to contain.
fn json_repr(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Int(n)) => n.to_string(),
        Some(Value::Float(f)) => format!("{f}"),
        Some(Value::String(s)) => format!("{s:?}"),
        Some(Value::Array(items)) => {
            let inner: Vec<String> = items.iter().map(|i| json_repr(Some(i))).collect();
            format!("[{}]", inner.join(","))
        }
        Some(Value::Object(entries)) => {
            let inner: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("{k:?}:{}", json_repr(Some(v))))
                .collect();
            format!("{{{}}}", inner.join(","))
        }
    }
}

fn as_string(v: Option<&Value>) -> Option<&str> {
    v.and_then(Value::as_str)
}

/// Resolve a plan path the same way a person running the CLI would find it:
/// cwd-relative first, then falling back to root-relative so `--validate`
/// still works from a subdirectory of the project.
fn resolve_plan_file(root: &Path, plan_field: &str) -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let cwd_relative = cwd.join(plan_field);
    if cwd_relative.exists() {
        return Some(cwd_relative);
    }
    let root_relative = root.join(plan_field);
    if root_relative.exists() {
        return Some(root_relative);
    }
    None
}

fn validate_report(root: &Path, report_path: &str) -> Result<ValidateResult, UserError> {
    let mut issues: Vec<ValidateIssue> = Vec::new();
    macro_rules! err {
        ($code:expr, $message:expr) => {
            issues.push(ValidateIssue {
                level: "error",
                code: $code.to_string(),
                message: $message,
            })
        };
    }
    macro_rules! warn_issue {
        ($code:expr, $message:expr) => {
            issues.push(ValidateIssue {
                level: "warn",
                code: $code.to_string(),
                message: $message,
            })
        };
    }

    if !Path::new(report_path).exists() {
        return Err(UserError::new(format!("No such file: {report_path}")));
    }

    let text = fs::read_to_string(report_path).map_err(|e| UserError::new(e.to_string()))?;
    let report = match json::parse(&text) {
        Ok(v) => v,
        Err(e) => {
            err!("json.parse", format!("report is not valid JSON: {e}"));
            return Ok(ValidateResult {
                file: report_path.to_string(),
                ok: false,
                errors: 1,
                warnings: 0,
                issues,
                report: Value::Null,
                clear: false,
            });
        }
    };

    if !matches!(report, Value::Object(_)) {
        err!("schema.shape", "report must be a JSON object".to_string());
        return Ok(ValidateResult {
            file: report_path.to_string(),
            ok: false,
            errors: 1,
            warnings: 0,
            issues,
            report,
            clear: false,
        });
    }

    let advise_version = report.get("agnosgram_advise");
    if advise_version != Some(&Value::Int(ADVISE_SCHEMA_VERSION)) {
        err!(
            "schema.version",
            format!(
                "\"agnosgram_advise\" must be {ADVISE_SCHEMA_VERSION}, got {}",
                json_repr(advise_version)
            )
        );
    }

    let plan_field = as_string(report.get("plan")).map(|s| s.to_string());
    if plan_field.is_none() {
        err!("schema.plan", "missing or non-string \"plan\"".to_string());
    }

    let generated = as_string(report.get("generated"));
    if generated.map(is_valid_iso_date) != Some(true) {
        err!(
            "schema.generated",
            "\"generated\" must be a YYYY-MM-DD date".to_string()
        );
    }

    let mut checked_ids: Vec<String> = Vec::new();
    match report.get("checked_ids") {
        Some(Value::Array(items)) => {
            checked_ids = items
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if items.iter().any(|v| !matches!(v, Value::String(_))) {
                err!(
                    "schema.checked_ids",
                    "\"checked_ids\" must contain only strings; found a non-string entry"
                        .to_string()
                );
            }
        }
        _ => {
            err!(
                "schema.checked_ids",
                "\"checked_ids\" must be an array of record ids".to_string()
            );
        }
    }

    let records = load_records(root);
    let by_id: HashMap<&str, &StoreRecord> = records
        .iter()
        .map(|r| (r.frontmatter.id.as_str(), r))
        .collect();

    for id in &checked_ids {
        if !by_id.contains_key(id.as_str()) {
            err!(
                "provenance.checked_id.unknown",
                format!("checked_ids references \"{id}\", which no record defines")
            );
        }
    }

    let contradictions: Vec<Value> = match report.get("contradictions") {
        Some(Value::Array(items)) => items.clone(),
        _ => {
            err!(
                "schema.contradictions",
                "\"contradictions\" must be an array".to_string()
            );
            Vec::new()
        }
    };

    // Plan file, for excerpt substring checks (best-effort: excerpt mismatches
    // are warnings, never errors, since the plan may have moved or the CLI
    // may be run from a different cwd than the report's provenance).
    let mut plan_text: Option<String> = None;
    if let Some(plan_field) = &plan_field {
        match resolve_plan_file(root, plan_field) {
            Some(resolved) => {
                plan_text = fs::read_to_string(&resolved).ok();
            }
            None => {
                warn_issue!(
                    "coverage.plan_missing",
                    format!(
                        "plan file \"{plan_field}\" was not found on disk; excerpt checks skipped"
                    )
                );
            }
        }
    }

    let mut blocker_seen = false;
    for (i, c) in contradictions.iter().enumerate() {
        let where_ = format!("contradictions[{i}]");
        let record_id = as_string(c.get("record_id")).map(|s| s.to_string());
        let record: Option<&StoreRecord> =
            record_id.as_deref().and_then(|id| by_id.get(id).copied());
        if record_id.is_none() {
            err!(
                format!("schema.{where_}.record_id"),
                format!("{where_}: missing or non-string \"record_id\"")
            );
        }
        if let Some(rid) = &record_id {
            if record.is_none() {
                err!(
                    "provenance.record_id.unknown",
                    format!("{where_}: record_id \"{rid}\" does not exist in the store")
                );
            } else if !checked_ids.iter().any(|c| c == rid) {
                warn_issue!(
                    "coverage.uncovered",
                    format!("{where_}: record_id \"{rid}\" is cited but not listed in checked_ids")
                );
            }
        }

        let kind = as_string(c.get("kind"));
        if !kind.map(|k| KNOWN_KINDS.contains(&k)).unwrap_or(false) {
            err!(
                format!("schema.{where_}.kind"),
                format!(
                    "{where_}: \"kind\" must be one of {}",
                    KNOWN_KINDS.join(", ")
                )
            );
        }

        let severity = as_string(c.get("severity"));
        if !severity
            .map(|s| KNOWN_SEVERITIES.contains(&s))
            .unwrap_or(false)
        {
            err!(
                format!("schema.{where_}.severity"),
                format!(
                    "{where_}: \"severity\" must be one of {}",
                    KNOWN_SEVERITIES.join(", ")
                )
            );
        } else if severity == Some("blocker") {
            blocker_seen = true;
        }

        let plan_excerpt = as_string(c.get("plan_excerpt")).filter(|s| !s.is_empty());
        match plan_excerpt {
            None => err!(
                format!("schema.{where_}.plan_excerpt"),
                format!("{where_}: missing or non-string \"plan_excerpt\"")
            ),
            Some(pe) => {
                if let Some(pt) = &plan_text {
                    if !pt.contains(pe) {
                        warn_issue!(
                            "excerpt.plan_mismatch",
                            format!(
                                "{where_}: plan_excerpt is not a substring of \"{}\"",
                                plan_field.as_deref().unwrap_or("")
                            )
                        );
                    }
                }
            }
        }

        let record_excerpt = as_string(c.get("record_excerpt")).filter(|s| !s.is_empty());
        match record_excerpt {
            None => err!(
                format!("schema.{where_}.record_excerpt"),
                format!("{where_}: missing or non-string \"record_excerpt\"")
            ),
            Some(re) => {
                if let (Some(rid), Some(rec)) = (&record_id, record) {
                    if !rec.body.contains(re) {
                        warn_issue!(
                            "excerpt.record_mismatch",
                            format!("{where_}: record_excerpt is not a substring of {rid}'s body")
                        );
                    }
                }
            }
        }

        let confidence = as_string(c.get("confidence"));
        match confidence.filter(|cf| KNOWN_CONFIDENCE.contains(cf)) {
            None => err!(
                format!("schema.{where_}.confidence"),
                format!(
                    "{where_}: \"confidence\" must be one of {}",
                    KNOWN_CONFIDENCE.join(", ")
                )
            ),
            Some(cf) => {
                if let Some(rec) = record {
                    if cf != rec.frontmatter.confidence {
                        err!(
                            "provenance.confidence.mismatch",
                            format!(
                                "{where_}: confidence \"{cf}\" does not match {}'s actual confidence \"{}\"",
                                record_id.as_deref().unwrap_or(""),
                                rec.frontmatter.confidence
                            )
                        );
                    }
                }
            }
        }

        let last_verified = as_string(c.get("last_verified"));
        match last_verified.filter(|lv| is_valid_iso_date(lv)) {
            None => err!(
                format!("schema.{where_}.last_verified"),
                format!("{where_}: \"last_verified\" must be a YYYY-MM-DD date")
            ),
            Some(lv) => {
                if let Some(rec) = record {
                    if lv != rec.frontmatter.last_verified {
                        err!(
                            "provenance.last_verified.mismatch",
                            format!(
                                "{where_}: last_verified \"{lv}\" does not match {}'s actual last_verified \"{}\"",
                                record_id.as_deref().unwrap_or(""),
                                rec.frontmatter.last_verified
                            )
                        );
                    }
                }
            }
        }

        let explanation_ok = as_string(c.get("explanation"))
            .map(|e| !e.trim().is_empty())
            .unwrap_or(false);
        if !explanation_ok {
            err!(
                format!("schema.{where_}.explanation"),
                format!("{where_}: missing or empty \"explanation\"")
            );
        }
    }

    let declared_clear = match report.get("clear") {
        Some(Value::Bool(b)) => Some(*b),
        _ => None,
    };
    match declared_clear {
        None => err!("schema.clear", "\"clear\" must be a boolean".to_string()),
        Some(true) if blocker_seen => {
            // A blocker alongside a self-declared `clear: true` is not just
            // inconsistent, it is the exact shape a Gate consumer trusting
            // `clear` would be fooled by - promote it to an error so
            // `--validate` (without even `--strict`) already catches it.
            err!(
                "consistency.clear",
                "\"clear\" is true but a blocker contradiction is present".to_string()
            );
        }
        Some(false) if contradictions.is_empty() => {
            warn_issue!(
                "consistency.clear",
                "\"clear\" is false but no contradictions were reported".to_string()
            );
        }
        _ => {}
    }

    // Coverage: did the report actually look at (close to) everything on offer?
    let store_ids: Vec<&str> = records.iter().map(|r| r.frontmatter.id.as_str()).collect();
    let unchecked_count = store_ids
        .iter()
        .filter(|id| !checked_ids.iter().any(|c| c == *id))
        .count();
    if checked_ids.is_empty() && !store_ids.is_empty() {
        warn_issue!(
            "coverage.empty",
            "no records were checked; the report examined nothing".to_string()
        );
    } else if unchecked_count > 0 {
        warn_issue!(
            "coverage.partial",
            format!(
                "{unchecked_count} of {} store record(s) were not listed in checked_ids",
                store_ids.len()
            )
        );
    }

    let errors = issues.iter().filter(|i| i.level == "error").count();
    let warnings = issues.iter().filter(|i| i.level == "warn").count();
    // Mechanical clearness: never trust the report's self-declared `clear`
    // alone - a blocker-severity contradiction always means "not clear".
    let clear = declared_clear == Some(true) && !blocker_seen;
    Ok(ValidateResult {
        file: report_path.to_string(),
        ok: errors == 0,
        errors,
        warnings,
        issues,
        report,
        clear,
    })
}

pub fn run(argv: Vec<String>) -> Result<(), UserError> {
    let cfg = ArgsConfig::new(true)
        .option("validate", OptionDef::boolean(false))
        .option("out", OptionDef::string())
        .option("strict", OptionDef::boolean(false))
        .option("json", OptionDef::boolean(false))
        .option("format", OptionDef::string());
    let parsed = parse_cli_args(&argv, &cfg)?;

    let format = resolve_format(&FormatFlags {
        format: parsed.str("format").map(|s| s.to_string()),
        json: parsed.bool("json"),
    })?;

    let root =
        find_project_root(&std::env::current_dir().map_err(|e| UserError::new(e.to_string()))?);
    if !has_store(&root) {
        return Err(UserError::new(
            "No .agnosgram/ store found. Run `agnosgram init` first.",
        ));
    }

    if parsed.bool("validate") {
        let report_arg = parsed
            .positionals
            .first()
            .map(|s| s.as_str())
            .or_else(|| parsed.str("out"));
        let Some(report_arg) = report_arg else {
            return Err(UserError::new(
                "Usage: agnosgram advise --validate <report-file>",
            ));
        };
        let result = validate_report(&root, report_arg)?;

        if let ResolvedFormat::Structured(fmt) = format {
            print_structured(&validate_result_to_json(&result), fmt);
        } else if result.issues.is_empty() {
            info(&format!("{}: valid.", result.file));
        } else {
            for i in &result.issues {
                let level = if i.level == "error" { "error" } else { "warn " };
                info(&format!("  {level} {}  {}", i.code, i.message));
            }
            info(&format!(
                "\n{}: {} error(s), {} warning(s).",
                result.file, result.errors, result.warnings
            ));
        }

        if result.errors > 0 || (parsed.bool("strict") && !result.clear) {
            use std::io::Write;
            let _ = std::io::stdout().flush();
            std::process::exit(1);
        }
        return Ok(());
    }

    let plan_path = parsed.positionals.first().map(|s| s.trim());
    let Some(plan_path) = plan_path.filter(|s| !s.is_empty()) else {
        return Err(UserError::new(
            "Usage: agnosgram advise <plan-path> [--out <report-file>]",
        ));
    };
    let out_path = parsed
        .str("out")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{plan_path}.advise.json"));
    let prompt = build_prompt(&root, plan_path, &out_path);

    if let ResolvedFormat::Structured(fmt) = format {
        let mut out = Value::object();
        out.insert("prompt", prompt);
        print_structured(&out, fmt);
    } else {
        use std::io::Write;
        let _ = std::io::stdout().write_all(prompt.as_bytes());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::frontmatter::{extract_records, validate_record, KNOWN_TYPES};
    use std::fs;

    fn tmp_root(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("agnos-advise-rs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(".agnosgram/lessons")).unwrap();
        dir
    }

    fn one_record() -> StoreRecord {
        let text = "---\nid: LES-001\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nDo not use require() in this ESM package.\n";
        let raw = extract_records(text).remove(0);
        let validated = validate_record(&raw, &KNOWN_TYPES);
        StoreRecord {
            frontmatter: validated.frontmatter.unwrap(),
            body: raw.body,
            file: ".agnosgram/lessons/pitfalls.md".to_string(),
            store_rel: "lessons/pitfalls.md".to_string(),
            line: raw.line,
        }
    }

    #[test]
    fn digest_table_reports_no_records_placeholder_when_empty() {
        assert!(digest_table(&[]).contains("_(store has no records yet)_"));
    }

    #[test]
    fn digest_table_escapes_pipes_and_only_truncates_past_80_chars() {
        let rec = one_record();
        let table = digest_table(&[rec]);
        assert!(table.contains("LES-001"));
        assert!(!table.contains("..."));
    }

    #[test]
    fn digest_table_truncates_a_long_body_with_an_ellipsis() {
        let mut rec = one_record();
        rec.body = "x".repeat(120);
        let table = digest_table(&[rec]);
        assert!(table.contains("..."));
    }

    #[test]
    fn digest_table_escapes_a_literal_pipe_in_the_body() {
        let mut rec = one_record();
        rec.body = "Short body with a | pipe in it.".to_string();
        let table = digest_table(&[rec]);
        assert!(table.contains("a \\| pipe in it."));
        let row = table.lines().find(|l| l.contains("LES-001")).unwrap();
        assert!(!row.contains("..."));
    }

    #[test]
    fn validate_report_rejects_a_missing_file() {
        let root = tmp_root("missing");
        let err = validate_report(&root, "nope.json").unwrap_err();
        assert!(err.0.contains("No such file"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn validate_report_rejects_malformed_json() {
        let root = tmp_root("bad-json");
        let report_path = root.join("bad.json");
        fs::write(&report_path, "{not json").unwrap();
        let result = validate_report(&root, report_path.to_str().unwrap()).unwrap();
        assert!(!result.ok);
        assert!(result.issues.iter().any(|i| i.code == "json.parse"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn validate_report_rejects_a_non_object_report() {
        let root = tmp_root("non-object");
        let report_path = root.join("arr.json");
        fs::write(&report_path, "[1,2,3]").unwrap();
        let result = validate_report(&root, report_path.to_str().unwrap()).unwrap();
        assert!(!result.ok);
        assert!(result.issues.iter().any(|i| i.code == "schema.shape"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn validate_report_accepts_a_well_formed_provenance_correct_report() {
        let root = tmp_root("valid");
        fs::write(
            root.join(".agnosgram/lessons/pitfalls.md"),
            "# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nDo not use require() in this ESM package; it fails at runtime.\n",
        )
        .unwrap();
        fs::write(
            root.join("plan.md"),
            "We propose to use require() everywhere for simplicity.\n",
        )
        .unwrap();
        let report = r#"{
  "agnosgram_advise": 1,
  "plan": "plan.md",
  "generated": "2026-07-27",
  "checked_ids": ["LES-001"],
  "contradictions": [{
    "record_id": "LES-001",
    "kind": "empirical",
    "severity": "blocker",
    "plan_excerpt": "use require() everywhere",
    "record_excerpt": "Do not use require() in this ESM package",
    "confidence": "high",
    "last_verified": "2026-07-21",
    "explanation": "Plan proposes require() but LES-001 forbids it."
  }],
  "clear": false
}"#;
        let report_path = root.join("report.json");
        fs::write(&report_path, report).unwrap();
        let orig_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();
        let result = validate_report(&root, "report.json").unwrap();
        std::env::set_current_dir(orig_cwd).unwrap();
        assert!(
            result.ok,
            "issues: {:?}",
            result.issues.iter().map(|i| &i.message).collect::<Vec<_>>()
        );
        assert!(!result.clear);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn validate_report_flags_a_clear_true_blocker_inconsistency() {
        let root = tmp_root("clear-blocker");
        fs::write(
            root.join(".agnosgram/lessons/pitfalls.md"),
            "# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nDo not use require().\n",
        )
        .unwrap();
        let report = r#"{
  "agnosgram_advise": 1,
  "plan": "plan.md",
  "generated": "2026-07-27",
  "checked_ids": ["LES-001"],
  "contradictions": [{
    "record_id": "LES-001",
    "kind": "empirical",
    "severity": "blocker",
    "plan_excerpt": "x",
    "record_excerpt": "Do not use require()",
    "confidence": "high",
    "last_verified": "2026-07-21",
    "explanation": "conflict"
  }],
  "clear": true
}"#;
        let report_path = root.join("report.json");
        fs::write(&report_path, report).unwrap();
        let result = validate_report(&root, report_path.to_str().unwrap()).unwrap();
        assert!(!result.ok);
        assert!(!result.clear);
        assert!(result.issues.iter().any(|i| i.code == "consistency.clear"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn json_repr_matches_json_stringify_shapes() {
        assert_eq!(json_repr(None), "undefined");
        assert_eq!(json_repr(Some(&Value::Null)), "null");
        assert_eq!(json_repr(Some(&Value::Int(2))), "2");
    }
}
