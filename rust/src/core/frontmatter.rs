//! Port of `src/core/frontmatter.ts`: record frontmatter extraction and
//! schema validation. Reads only - records are authored by humans or by
//! agents following the `distill` prompt, never serialized here.

use super::dates::is_valid_iso_date;
use super::yaml::{parse_yaml, YamlValue};

pub const KNOWN_TYPES: [&str; 3] = ["pitfall", "convention", "decision"];
pub const KNOWN_CONFIDENCE: [&str; 3] = ["low", "medium", "high"];

/// Ids look like `LES-001`, `CON-002`, `DEC-0001`: uppercase prefix (2+
/// letters) + number (2+ digits).
pub fn is_valid_id(s: &str) -> bool {
    let Some((prefix, num)) = s.split_once('-') else {
        return false;
    };
    prefix.len() >= 2
        && prefix.chars().all(|c| c.is_ascii_uppercase())
        && num.len() >= 2
        && num.chars().all(|c| c.is_ascii_digit())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Frontmatter {
    pub id: String,
    pub r#type: String,
    pub scope: Vec<String>,
    pub confidence: String,
    pub created: String,
    pub last_verified: String,
    pub source: String,
    pub supersedes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RawRecord {
    pub data: Vec<(String, YamlValue)>,
    pub body: String,
    /// 1-based line of the opening `---` fence.
    pub line: usize,
    pub parse_error: Option<String>,
}

impl RawRecord {
    fn get(&self, key: &str) -> Option<&YamlValue> {
        self.data.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueLevel {
    Error,
    Warn,
}

#[derive(Debug, Clone)]
pub struct RecordIssue {
    pub level: IssueLevel,
    pub code: String,
    pub message: String,
    pub line: usize,
}

pub struct ValidatedRecord {
    pub frontmatter: Option<Frontmatter>,
    pub issues: Vec<RecordIssue>,
}

/// Replace HTML-comment and fenced-code spans with blanks, preserving line
/// structure (each replaced char becomes a space, newlines survive). Mirrors
/// the TS implementation's two sequential `.replace(...)` passes: the second
/// pass (fenced code) scans the *already comment-masked* text, so a fence
/// entirely inside a comment is not matched twice (it is already blank).
fn mask_non_record_spans(text: &str) -> String {
    let mut buf: Vec<char> = text.chars().collect();
    mask_spans_in_place(&mut buf, "<!--", "-->");
    mask_spans_in_place(&mut buf, "```", "```");
    buf.into_iter().collect()
}

fn mask_spans_in_place(buf: &mut [char], open: &str, close: &str) {
    let snapshot: Vec<char> = buf.to_vec();
    let open_chars: Vec<char> = open.chars().collect();
    let close_chars: Vec<char> = close.chars().collect();
    let mut i = 0usize;
    while i < snapshot.len() {
        if matches_at(&snapshot, i, &open_chars) {
            let mut j = i + open_chars.len();
            let mut found_close = None;
            while j <= snapshot.len().saturating_sub(close_chars.len()) {
                if matches_at(&snapshot, j, &close_chars) {
                    found_close = Some(j + close_chars.len());
                    break;
                }
                j += 1;
            }
            let end = found_close.unwrap_or(snapshot.len());
            for slot in buf.iter_mut().take(end).skip(i) {
                if *slot != '\n' {
                    *slot = ' ';
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }
}

fn matches_at(chars: &[char], pos: usize, needle: &[char]) -> bool {
    if pos + needle.len() > chars.len() {
        return false;
    }
    chars[pos..pos + needle.len()] == *needle
}

/// A line that can appear inside a frontmatter block: a `key:` line, a
/// block-list item, an indented continuation, or nothing.
fn is_yamlish_line(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    // `\s+\S.*` : starts with whitespace, then a non-whitespace char, then anything.
    let chars = line.chars();
    let first = chars.clone().next().unwrap();
    if first.is_whitespace() {
        let rest: String = chars.skip(1).collect();
        if !rest.is_empty() && !rest.chars().next().unwrap().is_whitespace() {
            return true;
        }
        return false;
    }
    // `-\s.*` or `-`
    if line == "-" {
        return true;
    }
    if let Some(rest) = line.strip_prefix('-') {
        if rest.starts_with(char::is_whitespace) {
            return true;
        }
    }
    // `[A-Za-z_][A-Za-z0-9_-]*:(\s.*)?`
    let bytes: Vec<char> = line.chars().collect();
    if !(bytes[0].is_ascii_alphabetic() || bytes[0] == '_') {
        return false;
    }
    let mut idx = 1;
    while idx < bytes.len()
        && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == '_' || bytes[idx] == '-')
    {
        idx += 1;
    }
    if idx < bytes.len() && bytes[idx] == ':' {
        if idx + 1 == bytes.len() {
            return true;
        }
        return bytes[idx + 1].is_whitespace();
    }
    false
}

/// Extract every frontmatter record from a Markdown file's text.
pub fn extract_records(text: &str) -> Vec<RawRecord> {
    let original: Vec<&str> = text.split('\n').collect();
    let masked_text = mask_non_record_spans(text);
    let masked: Vec<&str> = masked_text.split('\n').collect();

    let mut bounds: Vec<(usize, usize)> = Vec::new();
    let mut i = 0usize;
    while i < masked.len() {
        if masked[i].trim() != "---" {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < masked.len() && masked[j].trim() != "---" {
            j += 1;
        }
        let block: Vec<&str> = masked[i + 1..j]
            .iter()
            .filter(|l| !l.trim().is_empty())
            .copied()
            .collect();
        let is_record = j < masked.len()
            && !block.is_empty()
            && block.iter().all(|l| is_yamlish_line(l.trim_end()));
        if !is_record {
            i += 1;
            continue;
        }
        bounds.push((i, j));
        i = j + 1;
    }

    let mut records = Vec::new();
    for k in 0..bounds.len() {
        let (open, close) = bounds[k];
        let yaml_text = original[open + 1..close].join("\n");
        let next_open = bounds.get(k + 1).map(|(o, _)| *o).unwrap_or(original.len());
        let body = original[close + 1..next_open].join("\n").trim().to_string();

        let mut data: Vec<(String, YamlValue)> = Vec::new();
        let mut parse_error = None;
        match parse_yaml(&yaml_text) {
            Ok(YamlValue::Map(entries)) => data = entries,
            Ok(_) => parse_error = Some("frontmatter is not a key/value map".to_string()),
            Err(e) => parse_error = Some(e.0),
        }

        records.push(RawRecord {
            data,
            body,
            line: open + 1,
            parse_error,
        });
    }
    records
}

fn as_string_list(value: Option<&YamlValue>) -> Option<Vec<String>> {
    match value? {
        YamlValue::String(s) => {
            if s.trim().is_empty() {
                Some(Vec::new())
            } else {
                Some(vec![s.clone()])
            }
        }
        YamlValue::Array(items) => Some(
            items
                .iter()
                .filter(|v| !matches!(v, YamlValue::Null))
                .map(yaml_to_display_string)
                .collect(),
        ),
        _ => None,
    }
}

fn yaml_to_display_string(v: &YamlValue) -> String {
    match v {
        YamlValue::String(s) => s.clone(),
        YamlValue::Int(n) => n.to_string(),
        YamlValue::Float(f) => f.to_string(),
        YamlValue::Bool(b) => b.to_string(),
        YamlValue::Null => "null".to_string(),
        _ => String::new(),
    }
}

/// Validate one raw record's frontmatter against the schema. `allowed_types`
/// defaults to the frozen lessons/decisions enum (`KNOWN_TYPES`).
pub fn validate_record(raw: &RawRecord, allowed_types: &[&str]) -> ValidatedRecord {
    let mut issues: Vec<RecordIssue> = Vec::new();
    macro_rules! err {
        ($code:expr, $message:expr) => {
            issues.push(RecordIssue {
                level: IssueLevel::Error,
                code: $code.to_string(),
                message: $message,
                line: raw.line,
            })
        };
    }

    if let Some(parse_error) = &raw.parse_error {
        err!(
            "frontmatter.parse",
            format!("frontmatter did not parse: {parse_error}")
        );
        return ValidatedRecord {
            frontmatter: None,
            issues,
        };
    }

    let id = raw.get("id").and_then(YamlValue::as_str);
    match id {
        None => err!("id.missing", "missing required field `id`".to_string()),
        Some(id) if !is_valid_id(id) => {
            err!("id.format", format!("id \"{id}\" must look like ABC-123"))
        }
        _ => {}
    }

    let type_ = raw.get("type").and_then(YamlValue::as_str);
    match type_ {
        None => err!("type.missing", "missing required field `type`".to_string()),
        Some(t) if !allowed_types.contains(&t) => {
            err!(
                "type.unknown",
                format!("type \"{t}\" is not one of {}", allowed_types.join(", "))
            );
        }
        _ => {}
    }

    let scope = as_string_list(raw.get("scope"));
    if scope.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        err!(
            "scope.missing",
            "missing or empty `scope` (expected a non-empty list)".to_string()
        );
    }

    let confidence = raw.get("confidence").and_then(YamlValue::as_str);
    match confidence {
        None => err!(
            "confidence.missing",
            "missing required field `confidence`".to_string()
        ),
        Some(c) if !KNOWN_CONFIDENCE.contains(&c) => {
            err!(
                "confidence.unknown",
                format!(
                    "confidence \"{c}\" is not one of {}",
                    KNOWN_CONFIDENCE.join(", ")
                )
            );
        }
        _ => {}
    }

    let created = raw.get("created").and_then(YamlValue::as_str);
    match created {
        None => err!(
            "created.missing",
            "missing required field `created`".to_string()
        ),
        Some(c) if !is_valid_iso_date(c) => {
            err!(
                "created.format",
                format!("created \"{c}\" is not a valid YYYY-MM-DD date")
            );
        }
        _ => {}
    }

    let last_verified = raw.get("last_verified").and_then(YamlValue::as_str);
    match last_verified {
        None => err!(
            "last_verified.missing",
            "missing required field `last_verified`".to_string()
        ),
        Some(lv) if !is_valid_iso_date(lv) => {
            err!(
                "last_verified.format",
                format!("last_verified \"{lv}\" is not a valid YYYY-MM-DD date")
            );
        }
        _ => {}
    }

    if let (Some(c), Some(lv)) = (created, last_verified) {
        if is_valid_iso_date(c) && is_valid_iso_date(lv) && lv < c {
            issues.push(RecordIssue {
                level: IssueLevel::Warn,
                code: "date.order".to_string(),
                message: "`last_verified` is earlier than `created`".to_string(),
                line: raw.line,
            });
        }
    }

    let source = raw.get("source").and_then(YamlValue::as_str);
    if source.is_none() {
        err!(
            "source.missing",
            "missing required field `source`".to_string()
        );
    }

    let mut supersedes: Option<Vec<String>> = None;
    if let Some(v) = raw.get("supersedes") {
        if !matches!(v, YamlValue::Null) {
            match as_string_list(Some(v)) {
                None => err!(
                    "supersedes.format",
                    "`supersedes` must be an id or a list of ids".to_string()
                ),
                Some(list) => {
                    for r in &list {
                        if !is_valid_id(r) {
                            err!(
                                "supersedes.format",
                                format!("supersedes \"{r}\" must look like ABC-123")
                            );
                        }
                    }
                    supersedes = Some(list);
                }
            }
        }
    }

    if raw.body.trim().is_empty() {
        issues.push(RecordIssue {
            level: IssueLevel::Warn,
            code: "body.empty".to_string(),
            message: "record has no body text".to_string(),
            line: raw.line,
        });
    }

    let has_error = issues.iter().any(|i| i.level == IssueLevel::Error);
    let frontmatter = if has_error {
        None
    } else {
        Some(Frontmatter {
            id: id.unwrap().to_string(),
            r#type: type_.unwrap().to_string(),
            scope: scope.unwrap(),
            confidence: confidence.unwrap().to_string(),
            created: created.unwrap().to_string(),
            last_verified: last_verified.unwrap().to_string(),
            source: source.unwrap().to_string(),
            supersedes: supersedes.unwrap_or_default(),
        })
    };

    ValidatedRecord {
        frontmatter,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "# Pitfalls\n\nintro text that is not a record\n\n---\nid: LES-001\ntype: pitfall\nscope: [core, tooling]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nDo not use require() in this ESM package.\n\n---\nid: LES-002\ntype: convention\nscope: [tooling]\nconfidence: medium\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\nsupersedes: LES-000\n---\nPrefer node built-ins.\n";

    #[test]
    fn extract_records_finds_every_record_and_skips_intro_text() {
        let recs = extract_records(VALID);
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].get("id").unwrap().as_str(), Some("LES-001"));
        assert!(recs[0].body.contains("require()"));
        assert_eq!(recs[1].get("id").unwrap().as_str(), Some("LES-002"));
    }

    #[test]
    fn extract_records_ignores_frontmatter_inside_html_comments() {
        let templated = "# Pitfalls\n\n<!-- Example:\n\n---\nid: LES-001\ntype: pitfall\n---\nexample body\n\n-->\n";
        assert_eq!(extract_records(templated).len(), 0);
    }

    #[test]
    fn extract_records_ignores_frontmatter_inside_fenced_code_blocks() {
        let doc = "# Decisions\n\n```yaml\n---\nid: DEC-0001\n---\n```\n";
        assert_eq!(extract_records(doc).len(), 0);
    }

    #[test]
    fn validate_record_accepts_a_well_formed_record() {
        let recs = extract_records(VALID);
        let v = validate_record(&recs[0], &KNOWN_TYPES);
        assert_eq!(
            v.issues
                .iter()
                .filter(|i| i.level == IssueLevel::Error)
                .count(),
            0
        );
        let fm = v.frontmatter.unwrap();
        assert_eq!(fm.scope, vec!["core".to_string(), "tooling".to_string()]);
    }

    #[test]
    fn validate_record_normalizes_a_single_supersedes_id_to_a_list() {
        let recs = extract_records(VALID);
        let v = validate_record(&recs[1], &KNOWN_TYPES);
        assert_eq!(
            v.frontmatter.unwrap().supersedes,
            vec!["LES-000".to_string()]
        );
    }

    #[test]
    fn validate_record_flags_missing_required_fields() {
        let recs = extract_records("---\ntype: pitfall\n---\nbody\n");
        let v = validate_record(&recs[0], &KNOWN_TYPES);
        assert!(v.frontmatter.is_none());
        let codes: Vec<&str> = v.issues.iter().map(|i| i.code.as_str()).collect();
        assert!(codes.contains(&"id.missing"));
        assert!(codes.contains(&"scope.missing"));
        assert!(codes.contains(&"confidence.missing"));
        assert!(codes.contains(&"created.missing"));
    }

    #[test]
    fn validate_record_rejects_bad_id_type_confidence_and_date() {
        let recs = extract_records(
            "---\nid: bad_id\ntype: rumor\nscope: [x]\nconfidence: maybe\ncreated: 2026-13-40\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nbody\n",
        );
        let v = validate_record(&recs[0], &KNOWN_TYPES);
        let codes: Vec<&str> = v.issues.iter().map(|i| i.code.as_str()).collect();
        assert!(codes.contains(&"id.format"));
        assert!(codes.contains(&"type.unknown"));
        assert!(codes.contains(&"confidence.unknown"));
        assert!(codes.contains(&"created.format"));
    }

    #[test]
    fn validate_record_warns_when_last_verified_precedes_created() {
        let recs = extract_records(
            "---\nid: LES-009\ntype: pitfall\nscope: [x]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-01\nsource: journal/2026-07.md\n---\nbody\n",
        );
        let v = validate_record(&recs[0], &KNOWN_TYPES);
        let codes: Vec<&str> = v.issues.iter().map(|i| i.code.as_str()).collect();
        assert!(codes.contains(&"date.order"));
    }

    #[test]
    fn a_thematic_break_inside_a_body_does_not_derail_later_records() {
        let doc = "# Pitfalls\n\n---\nid: LES-001\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nFirst half of the lesson.\n\n---\n\nSecond half after a horizontal rule.\n\n---\nid: LES-002\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\nA completely separate second lesson.\n";
        let recs = extract_records(doc);
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].get("id").unwrap().as_str(), Some("LES-001"));
        assert!(recs[0]
            .body
            .contains("Second half after a horizontal rule."));
        assert_eq!(recs[1].get("id").unwrap().as_str(), Some("LES-002"));
        for rec in &recs {
            let v = validate_record(rec, &KNOWN_TYPES);
            assert_eq!(
                v.issues
                    .iter()
                    .filter(|i| i.level == IssueLevel::Error)
                    .count(),
                0
            );
        }
    }

    #[test]
    fn a_body_consisting_of_a_fenced_code_block_is_preserved_not_blanked() {
        let doc = "---\nid: LES-003\ntype: pitfall\nscope: [core]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\n```bash\nexport NODE_OPTIONS=--max-old-space-size=4096\n```\n";
        let recs = extract_records(doc);
        assert!(recs[0].body.contains("NODE_OPTIONS"));
        let v = validate_record(&recs[0], &KNOWN_TYPES);
        let codes: Vec<&str> = v.issues.iter().map(|i| i.code.as_str()).collect();
        assert!(!codes.contains(&"body.empty"));
    }
}
