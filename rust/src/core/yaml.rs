//! Port of `src/core/yaml.ts`: minimal, dependency-free YAML for the narrow
//! subset Agnosgram controls: nested maps (2-space indent), scalars (string /
//! number / boolean / null), and block sequences of scalars (`- item`). Not a
//! general YAML implementation - it covers `config.yml` and record
//! frontmatter, both of which we author from templates. Anything outside the
//! subset returns an error.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum YamlValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    Array(Vec<YamlValue>),
    Map(Vec<(String, YamlValue)>),
}

impl YamlValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            YamlValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn as_map(&self) -> Option<&[(String, YamlValue)]> {
        match self {
            YamlValue::Map(entries) => Some(entries.as_slice()),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn as_array(&self) -> Option<&[YamlValue]> {
        match self {
            YamlValue::Array(items) => Some(items.as_slice()),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            YamlValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            YamlValue::Int(n) => Some(*n),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn get(&self, key: &str) -> Option<&YamlValue> {
        self.as_map()?
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }
}

impl From<&str> for YamlValue {
    fn from(s: &str) -> Self {
        YamlValue::String(s.to_string())
    }
}
impl From<String> for YamlValue {
    fn from(s: String) -> Self {
        YamlValue::String(s)
    }
}
impl From<bool> for YamlValue {
    fn from(b: bool) -> Self {
        YamlValue::Bool(b)
    }
}
impl From<i64> for YamlValue {
    fn from(n: i64) -> Self {
        YamlValue::Int(n)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlError(pub String);

impl fmt::Display for YamlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for YamlError {}

struct Line {
    indent: usize,
    content: String,
    line_no: usize,
}

/// Remove a trailing ` # comment`, respecting single/double quotes.
fn strip_comment(raw: &str) -> &str {
    let chars: Vec<char> = raw.chars().collect();
    let mut in_single = false;
    let mut in_double = false;
    for i in 0..chars.len() {
        let ch = chars[i];
        let prev = if i == 0 { '\0' } else { chars[i - 1] };
        let can_open = i == 0 || prev.is_whitespace() || prev == '[' || prev == ',' || prev == ':';
        if ch == '\'' && !in_double {
            if in_single {
                in_single = false;
            } else if can_open {
                in_single = true;
            }
        } else if ch == '"' && !in_single {
            if in_double {
                in_double = false;
            } else if can_open {
                in_double = true;
            }
        } else if ch == '#' && !in_single && !in_double && (i == 0 || prev == ' ' || prev == '\t') {
            let byte_idx: usize = chars[..i].iter().map(|c| c.len_utf8()).sum();
            return &raw[..byte_idx];
        }
    }
    raw
}

fn tokenize(text: &str) -> Vec<Line> {
    let mut out = Vec::new();
    for (idx, row) in text.split(['\n']).enumerate() {
        let row = row.strip_suffix('\r').unwrap_or(row);
        let raw = strip_comment(row);
        if raw.trim().is_empty() {
            continue;
        }
        if raw.trim() == "---" {
            continue;
        }
        let trimmed_start = raw.trim_start();
        let indent = raw.chars().count() - trimmed_start.chars().count();
        out.push(Line {
            indent,
            content: trimmed_start.trim_end().to_string(),
            line_no: idx + 1,
        });
    }
    out
}

fn parse_scalar(token: &str) -> YamlValue {
    let t = token.trim();
    if t.is_empty() || t == "~" || t == "null" {
        return YamlValue::Null;
    }
    if t == "true" {
        return YamlValue::Bool(true);
    }
    if t == "false" {
        return YamlValue::Bool(false);
    }
    if is_int_literal(t) {
        if let Ok(n) = t.parse::<i64>() {
            return YamlValue::Int(n);
        }
    }
    if is_float_literal(t) {
        if let Ok(f) = t.parse::<f64>() {
            return YamlValue::Float(f);
        }
    }
    if t == "{}" {
        return YamlValue::Map(Vec::new());
    }
    if t.starts_with('[') && t.ends_with(']') {
        let inner = t[1..t.len() - 1].trim();
        if inner.is_empty() {
            return YamlValue::Array(Vec::new());
        }
        return YamlValue::Array(split_flow(inner).iter().map(|s| parse_scalar(s)).collect());
    }
    if (t.starts_with('"') && t.ends_with('"') && t.len() >= 2)
        || (t.starts_with('\'') && t.ends_with('\'') && t.len() >= 2)
    {
        return YamlValue::String(t[1..t.len() - 1].to_string());
    }
    YamlValue::String(t.to_string())
}

fn is_int_literal(t: &str) -> bool {
    let t = t.strip_prefix('-').unwrap_or(t);
    !t.is_empty() && t.chars().all(|c| c.is_ascii_digit())
}

fn is_float_literal(t: &str) -> bool {
    let t2 = t.strip_prefix('-').unwrap_or(t);
    match t2.split_once('.') {
        Some((int_part, frac_part)) => {
            !int_part.is_empty()
                && !frac_part.is_empty()
                && int_part.chars().all(|c| c.is_ascii_digit())
                && frac_part.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

/// Split a flow-sequence body on top-level commas, respecting quotes.
fn split_flow(inner: &str) -> Vec<String> {
    let chars: Vec<char> = inner.chars().collect();
    let mut parts = Vec::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut start = 0usize;
    for i in 0..chars.len() {
        let ch = chars[i];
        if ch == '\'' && !in_double {
            in_single = !in_single;
        } else if ch == '"' && !in_single {
            in_double = !in_double;
        } else if ch == ',' && !in_single && !in_double {
            parts.push(
                chars[start..i]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_string(),
            );
            start = i + 1;
        }
    }
    parts.push(chars[start..].iter().collect::<String>().trim().to_string());
    parts.into_iter().filter(|p| !p.is_empty()).collect()
}

fn find_colon(content: &str) -> Option<usize> {
    let chars: Vec<char> = content.chars().collect();
    let mut in_single = false;
    let mut in_double = false;
    for i in 0..chars.len() {
        let ch = chars[i];
        if ch == '\'' && !in_double {
            in_single = !in_single;
        } else if ch == '"' && !in_single {
            in_double = !in_double;
        } else if ch == ':'
            && !in_single
            && !in_double
            && (i + 1 >= chars.len() || chars[i + 1] == ' ')
        {
            let byte_idx: usize = chars[..i].iter().map(|c| c.len_utf8()).sum();
            return Some(byte_idx);
        }
    }
    None
}

fn is_block_scalar_marker(rest: &str) -> bool {
    // `^[|>][+-]?$`
    let mut chars = rest.chars();
    match chars.next() {
        Some('|') | Some('>') => {}
        _ => return false,
    }
    match chars.next() {
        None => true,
        Some('+') | Some('-') => chars.next().is_none(),
        _ => false,
    }
}

/// Parse a block of lines whose indentation is >= `indent`, starting at `i`.
fn parse_block(
    lines: &[Line],
    mut i: usize,
    indent: usize,
) -> Result<(YamlValue, usize), YamlError> {
    let Some(first) = lines.get(i) else {
        return Ok((YamlValue::Null, i));
    };

    if first.content.starts_with("- ") || first.content == "-" {
        let mut arr = Vec::new();
        while i < lines.len() && lines[i].indent == indent && lines[i].content.starts_with('-') {
            let item = lines[i].content[1..].trim();
            arr.push(parse_scalar(item));
            i += 1;
        }
        return Ok((YamlValue::Array(arr), i));
    }

    let mut map: Vec<(String, YamlValue)> = Vec::new();
    while i < lines.len() && lines[i].indent == indent {
        let line = &lines[i];
        let Some(colon) = find_colon(&line.content) else {
            return Err(YamlError(format!(
                "Invalid YAML at line {}: expected \"key: value\"",
                line.line_no
            )));
        };
        let key = line.content[..colon].trim().to_string();
        let rest = line.content[colon + 1..].trim();
        if is_block_scalar_marker(rest) {
            return Err(YamlError(format!(
                "Invalid YAML at line {}: block scalars (| and >) are outside the supported subset",
                line.line_no
            )));
        }
        let rest_owned = rest.to_string();
        i += 1;
        if !rest_owned.is_empty() {
            set_map(&mut map, key, parse_scalar(&rest_owned));
        } else if i < lines.len() && lines[i].indent > indent {
            let next_indent = lines[i].indent;
            let (value, next) = parse_block(lines, i, next_indent)?;
            set_map(&mut map, key, value);
            i = next;
        } else {
            set_map(&mut map, key, YamlValue::Null);
        }
    }
    Ok((YamlValue::Map(map), i))
}

fn set_map(map: &mut Vec<(String, YamlValue)>, key: String, value: YamlValue) {
    if let Some(slot) = map.iter_mut().find(|(k, _)| *k == key) {
        slot.1 = value;
    } else {
        map.push((key, value));
    }
}

pub fn parse_yaml(text: &str) -> Result<YamlValue, YamlError> {
    let lines = tokenize(text);
    if lines.is_empty() {
        return Ok(YamlValue::Map(Vec::new()));
    }
    let (value, next) = parse_block(&lines, 0, lines[0].indent)?;
    if next < lines.len() {
        return Err(YamlError(format!(
            "Invalid YAML at line {}: unexpected indentation or content outside the supported subset",
            lines[next].line_no
        )));
    }
    Ok(value)
}

fn needs_quote(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    if s.chars().next().map(|c| c.is_whitespace()).unwrap_or(false)
        || s.chars().last().map(|c| c.is_whitespace()).unwrap_or(false)
    {
        return true;
    }
    if s.contains(':') || s.contains('#') {
        return true;
    }
    if matches!(s, "true" | "false" | "null" | "~") {
        return true;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if first == '-' {
        if matches!(chars.next(), Some(c) if c.is_ascii_digit()) {
            return true;
        }
    } else if first.is_ascii_digit() {
        return true;
    }
    false
}

/// `JSON.stringify(value)` for a plain string, used the same way `yaml.ts`
/// reuses `JSON.stringify` to quote a scalar - double-quoted with the same
/// escaping rules the JSON emitter uses.
fn json_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn format_scalar_str(value: &str) -> String {
    if needs_quote(value) {
        json_quote(value)
    } else {
        value.to_string()
    }
}

fn format_scalar(value: &YamlValue) -> String {
    match value {
        YamlValue::Null => String::new(),
        YamlValue::Bool(b) => b.to_string(),
        YamlValue::Int(n) => n.to_string(),
        YamlValue::Float(f) => f.to_string(),
        YamlValue::String(s) => format_scalar_str(s),
        YamlValue::Array(_) | YamlValue::Map(_) => {
            unreachable!("format_scalar called on a non-scalar")
        }
    }
}

pub fn stringify_yaml(value: &YamlValue, indent: usize) -> String {
    let pad = "  ".repeat(indent);

    match value {
        YamlValue::Array(items) => {
            if items.is_empty() {
                return format!("{pad}[]\n");
            }
            let mut out = String::new();
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                out.push_str(&format!("{pad}- {}", format_scalar(item)));
            }
            out.push('\n');
            out
        }
        YamlValue::Map(entries) => {
            if entries.is_empty() {
                return format!("{pad}{{}}\n");
            }
            let mut out = String::new();
            for (key, v) in entries {
                match v {
                    YamlValue::Array(items) => {
                        if items.is_empty() {
                            out.push_str(&format!("{pad}{key}: []\n"));
                        } else {
                            out.push_str(&format!("{pad}{key}:\n"));
                            out.push_str(&stringify_yaml(v, indent));
                        }
                    }
                    YamlValue::Map(_) => {
                        out.push_str(&format!("{pad}{key}:\n"));
                        out.push_str(&stringify_yaml(v, indent + 1));
                    }
                    scalar => {
                        out.push_str(&format!("{pad}{key}: {}\n", format_scalar(scalar)));
                    }
                }
            }
            out
        }
        scalar => format!("{pad}{}\n", format_scalar(scalar)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scalars_with_correct_types() {
        let v = parse_yaml("version: 1\nname: hello\nflag: true\noff: false\nempty:\n").unwrap();
        assert_eq!(v.get("version").unwrap().as_i64(), Some(1));
        assert_eq!(v.get("name").unwrap().as_str(), Some("hello"));
        assert_eq!(v.get("flag").unwrap().as_bool(), Some(true));
        assert_eq!(v.get("off").unwrap().as_bool(), Some(false));
        assert_eq!(v.get("empty").unwrap(), &YamlValue::Null);
    }

    #[test]
    fn parses_nested_maps() {
        let v = parse_yaml("journal:\n  committed: true\nbudgets:\n  a.md: 100\n").unwrap();
        assert_eq!(
            v.get("journal")
                .unwrap()
                .get("committed")
                .unwrap()
                .as_bool(),
            Some(true)
        );
        assert_eq!(
            v.get("budgets").unwrap().get("a.md").unwrap().as_i64(),
            Some(100)
        );
    }

    #[test]
    fn parses_block_sequences_of_scalars() {
        let v = parse_yaml("scope:\n  - backend\n  - auth\n").unwrap();
        let scope = v.get("scope").unwrap().as_array().unwrap();
        assert_eq!(
            scope,
            &[
                YamlValue::String("backend".into()),
                YamlValue::String("auth".into())
            ]
        );
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        let v = parse_yaml("# a comment\n\nversion: 1  # inline\n").unwrap();
        assert_eq!(v.get("version").unwrap().as_i64(), Some(1));
    }

    #[test]
    fn round_trips_a_config_shaped_object() {
        let obj = YamlValue::Map(vec![
            ("version".into(), YamlValue::Int(1)),
            (
                "journal".into(),
                YamlValue::Map(vec![("committed".into(), YamlValue::Bool(true))]),
            ),
            (
                "budgets".into(),
                YamlValue::Map(vec![("state/status.md".into(), YamlValue::Int(400))]),
            ),
            (
                "adapters".into(),
                YamlValue::Map(vec![
                    ("claude".into(), "on".into()),
                    ("cursor".into(), "off".into()),
                ]),
            ),
            (
                "sdd".into(),
                YamlValue::Map(vec![("openspec".into(), "auto".into())]),
            ),
        ]);
        let round = parse_yaml(&stringify_yaml(&obj, 0)).unwrap();
        assert_eq!(round, obj);
    }

    #[test]
    fn parses_inline_flow_sequences_of_scalars() {
        let v = parse_yaml("scope: [core, tooling]\nsupersedes: [DEC-0001, DEC-0002]\n").unwrap();
        assert_eq!(
            v.get("scope").unwrap().as_array().unwrap(),
            &[
                YamlValue::String("core".into()),
                YamlValue::String("tooling".into())
            ]
        );
        assert_eq!(
            v.get("supersedes").unwrap().as_array().unwrap(),
            &[
                YamlValue::String("DEC-0001".into()),
                YamlValue::String("DEC-0002".into())
            ]
        );
    }

    #[test]
    fn parses_empty_flow_sequence_and_empty_flow_map() {
        let v = parse_yaml("a: []\nb: {}\n").unwrap();
        assert_eq!(v.get("a").unwrap(), &YamlValue::Array(vec![]));
        assert_eq!(v.get("b").unwrap(), &YamlValue::Map(vec![]));
    }

    #[test]
    fn flow_sequence_respects_quoted_commas() {
        let v = parse_yaml("tags: [\"a, b\", c]\n").unwrap();
        assert_eq!(
            v.get("tags").unwrap().as_array().unwrap(),
            &[
                YamlValue::String("a, b".into()),
                YamlValue::String("c".into())
            ]
        );
    }

    #[test]
    fn quotes_values_that_would_otherwise_reparse_wrong() {
        let obj = YamlValue::Map(vec![
            ("a".into(), YamlValue::String("true".into())),
            ("b".into(), YamlValue::String("123".into())),
            ("c".into(), YamlValue::String("x: y".into())),
        ]);
        let out = stringify_yaml(&obj, 0);
        let round = parse_yaml(&out).unwrap();
        assert_eq!(round.get("a").unwrap().as_str(), Some("true"));
        assert_eq!(round.get("b").unwrap().as_str(), Some("123"));
        assert_eq!(round.get("c").unwrap().as_str(), Some("x: y"));
    }

    #[test]
    fn rejects_block_scalars_instead_of_silently_mis_parsing_them() {
        let err = parse_yaml("source: |\n  journal/2026-07.md\n  trailing junk\n").unwrap_err();
        assert!(err.0.contains("block scalars"));
        let err2 = parse_yaml("note: >-\n  folded\n").unwrap_err();
        assert!(err2.0.contains("block scalars"));
    }

    #[test]
    fn rejects_leftover_lines_it_cannot_represent() {
        let err = parse_yaml("a: 1\n  b: 2\n").unwrap_err();
        assert!(err.0.contains("line 2"));
    }

    #[test]
    fn strips_a_comment_after_an_unquoted_value_containing_an_apostrophe() {
        let v = parse_yaml("note: don't repeat this # see LES-002\n").unwrap();
        assert_eq!(v.get("note").unwrap().as_str(), Some("don't repeat this"));
    }
}
