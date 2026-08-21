//! Port of `src/core/serialize.ts`: the pluggable structured-output
//! serializer. JSON is the default; TOON is opt-in.

use super::json::{self, Value};
use super::output::UserError;
use super::toon::encode_toon;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Toon,
}

pub fn is_format(value: &str) -> bool {
    value == "json" || value == "toon"
}

fn parse_format(value: &str) -> Option<Format> {
    match value {
        "json" => Some(Format::Json),
        "toon" => Some(Format::Toon),
        _ => None,
    }
}

/// Render a structured value in the requested format.
pub fn serialize(value: &Value, format: Format) -> String {
    match format {
        Format::Toon => encode_toon(value),
        Format::Json => json::stringify_pretty(value),
    }
}

/// The subset of a command's parsed flags that decide output format.
pub struct FormatFlags {
    pub format: Option<String>,
    pub json: bool,
}

/// Resolved format, or `human` for the default human-readable text path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedFormat {
    Structured(Format),
    Human,
}

/// Resolve the output format from `--format <fmt>` / `--json` flags.
/// `--format` wins when both are given; `--json` is the shorthand for
/// `--format json`.
pub fn resolve_format(flags: &FormatFlags) -> Result<ResolvedFormat, UserError> {
    if let Some(fmt) = &flags.format {
        return match parse_format(fmt) {
            Some(f) => Ok(ResolvedFormat::Structured(f)),
            None => Err(UserError::new(format!(
                "unknown --format \"{fmt}\" (use json or toon)"
            ))),
        };
    }
    if flags.json {
        return Ok(ResolvedFormat::Structured(Format::Json));
    }
    Ok(ResolvedFormat::Human)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_format_accepts_only_json_and_toon() {
        assert!(is_format("json"));
        assert!(is_format("toon"));
        assert!(!is_format("yaml"));
    }

    #[test]
    fn serialize_defaults_to_pretty_json() {
        let mut v = Value::object();
        v.insert("a", 1i64);
        assert_eq!(serialize(&v, Format::Json), "{\n  \"a\": 1\n}");
    }

    #[test]
    fn serialize_routes_toon_through_the_toon_encoder() {
        let v: Value = vec!["a", "b"].into();
        assert_eq!(serialize(&v, Format::Toon), "[2]: a,b");
    }

    #[test]
    fn resolve_format_no_flags_means_human() {
        let flags = FormatFlags {
            format: None,
            json: false,
        };
        assert_eq!(resolve_format(&flags).unwrap(), ResolvedFormat::Human);
    }

    #[test]
    fn resolve_format_json_flag_means_json() {
        let flags = FormatFlags {
            format: None,
            json: true,
        };
        assert_eq!(
            resolve_format(&flags).unwrap(),
            ResolvedFormat::Structured(Format::Json)
        );
    }

    #[test]
    fn resolve_format_format_takes_precedence_over_json() {
        let flags = FormatFlags {
            format: Some("toon".to_string()),
            json: true,
        };
        assert_eq!(
            resolve_format(&flags).unwrap(),
            ResolvedFormat::Structured(Format::Toon)
        );
    }

    #[test]
    fn resolve_format_unknown_format_is_a_user_error() {
        let flags = FormatFlags {
            format: Some("yaml".to_string()),
            json: false,
        };
        let err = resolve_format(&flags).unwrap_err();
        assert!(err.0.contains("unknown --format"));
    }
}
