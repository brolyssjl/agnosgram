//! Port of `src/core/toon.ts`: a minimal TOON (Token-Oriented Object
//! Notation) encoder. Encoder only - Agnosgram never parses TOON back.

use super::json::Value;

const INDENT: &str = "  ";

pub fn encode_toon(value: &Value) -> String {
    if is_primitive(value) {
        return encode_primitive(value);
    }
    match value {
        Value::Array(items) => encode_array("", items, 0).trim_start().to_string(),
        Value::Object(entries) => encode_object(entries, 0),
        _ => unreachable!("is_primitive already handled scalars"),
    }
}

fn encode_object(entries: &[(String, Value)], depth: usize) -> String {
    let pad = INDENT.repeat(depth);
    let mut lines: Vec<String> = Vec::new();
    for (key, val) in entries {
        if is_primitive(val) {
            lines.push(format!(
                "{pad}{}: {}",
                encode_key(key),
                encode_primitive(val)
            ));
        } else if let Value::Array(arr) = val {
            lines.push(encode_array(key, arr, depth));
        } else if let Value::Object(obj) = val {
            lines.push(format!("{pad}{}:", encode_key(key)));
            lines.push(encode_object(obj, depth + 1));
        }
    }
    lines.join("\n")
}

fn encode_array(key: &str, arr: &[Value], depth: usize) -> String {
    let pad = INDENT.repeat(depth);
    let label = if key.is_empty() {
        String::new()
    } else {
        encode_key(key)
    };
    if arr.is_empty() {
        return format!("{pad}{label}[0]:");
    }

    if arr.iter().all(is_primitive) {
        let cells: Vec<String> = arr.iter().map(encode_primitive).collect();
        return format!("{pad}{label}[{}]: {}", arr.len(), cells.join(","));
    }

    if let Some(table) = as_uniform_table(arr) {
        let header = format!(
            "{pad}{label}[{}]{{{}}}:",
            arr.len(),
            table
                .fields
                .iter()
                .map(|f| encode_key(f))
                .collect::<Vec<_>>()
                .join(",")
        );
        let row_pad = INDENT.repeat(depth + 1);
        let rows: Vec<String> = table
            .rows
            .iter()
            .map(|row| {
                format!(
                    "{row_pad}{}",
                    row.iter()
                        .map(|v| encode_primitive(v))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect();
        let mut out = vec![header];
        out.extend(rows);
        return out.join("\n");
    }

    let header = format!("{pad}{label}[{}]:", arr.len());
    let item_pad = INDENT.repeat(depth + 1);
    let items: Vec<String> = arr
        .iter()
        .map(|item| {
            let body = if is_primitive(item) {
                encode_primitive(item)
            } else if let Value::Array(nested) = item {
                encode_array("", nested, depth + 2).trim_start().to_string()
            } else if let Value::Object(obj) = item {
                format!("\n{}", encode_object(obj, depth + 2))
            } else {
                unreachable!()
            };
            let body = if let Some(rest) = body.strip_prefix('\n') {
                rest.to_string()
            } else {
                body
            };
            format!("{item_pad}- {body}")
        })
        .collect();
    let mut out = vec![header];
    out.extend(items);
    out.join("\n")
}

struct Table<'a> {
    fields: Vec<&'a str>,
    rows: Vec<Vec<&'a Value>>,
}

fn as_uniform_table(arr: &[Value]) -> Option<Table<'_>> {
    let first = arr.first()?;
    let Value::Object(first_entries) = first else {
        return None;
    };
    let fields: Vec<&str> = first_entries.iter().map(|(k, _)| k.as_str()).collect();
    if fields.is_empty() {
        return None;
    }
    let mut rows: Vec<Vec<&Value>> = Vec::new();
    for item in arr {
        let Value::Object(entries) = item else {
            return None;
        };
        if entries.len() != fields.len() {
            return None;
        }
        let mut row = Vec::with_capacity(fields.len());
        for f in &fields {
            let cell = entries.iter().find(|(k, _)| k == f).map(|(_, v)| v)?;
            if !is_primitive(cell) {
                return None;
            }
            row.push(cell);
        }
        rows.push(row);
    }
    Some(Table { fields, rows })
}

fn is_primitive(v: &Value) -> bool {
    !matches!(v, Value::Array(_) | Value::Object(_))
}

fn encode_primitive(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => {
            if f.is_finite() {
                let s = format!("{f}");
                if s == "-0" {
                    "0".to_string()
                } else {
                    s
                }
            } else {
                "null".to_string()
            }
        }
        Value::String(s) => {
            if needs_quote(s) {
                quote(s)
            } else {
                s.clone()
            }
        }
        Value::Array(_) | Value::Object(_) => "null".to_string(),
    }
}

fn encode_key(key: &str) -> String {
    if needs_quote(key) {
        quote(key)
    } else {
        key.to_string()
    }
}

fn needs_quote(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    if s.chars()
        .any(|c| matches!(c, ',' | ':' | '{' | '}' | '[' | ']' | '"' | '\n'))
    {
        return true;
    }
    if s.trim() != s {
        return true;
    }
    if s == "null" || s == "true" || s == "false" {
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

fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(pairs: Vec<(&str, Value)>) -> Value {
        Value::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    #[test]
    fn encodes_uniform_array_of_flat_objects_as_table() {
        let arr = Value::Array(vec![
            obj(vec![("id", "LES-001".into()), ("type", "pitfall".into())]),
            obj(vec![
                ("id", "LES-002".into()),
                ("type", "convention".into()),
            ]),
        ]);
        assert_eq!(
            encode_toon(&arr),
            "[2]{id,type}:\n  LES-001,pitfall\n  LES-002,convention"
        );
    }

    #[test]
    fn encodes_small_non_uniform_object_as_indented_lines() {
        let value = obj(vec![("focus", "M3".into()), ("inFlight", Value::Null)]);
        assert_eq!(encode_toon(&value), "focus: M3\ninFlight: null");
    }

    #[test]
    fn encodes_primitive_array_inline() {
        let arr: Value = vec!["a", "b", "c"].into();
        assert_eq!(encode_toon(&arr), "[3]: a,b,c");
    }

    #[test]
    fn encodes_empty_array() {
        assert_eq!(encode_toon(&Value::array()), "[0]:");
    }

    #[test]
    fn quotes_values_and_keys_that_need_it() {
        let value = obj(vec![
            ("weird key", "has, a comma".into()),
            ("n", "123abc".into()),
        ]);
        let out = encode_toon(&value);
        assert!(out.starts_with("weird key: "));
        assert!(out.contains("\"has, a comma\""));
        assert!(out.contains("\"123abc\""));
    }

    #[test]
    fn quotes_a_key_with_reserved_characters() {
        let value = obj(vec![("a,b", 1i64.into())]);
        assert_eq!(encode_toon(&value), "\"a,b\": 1");
    }

    #[test]
    fn falls_back_to_list_form_for_non_uniform_array_of_objects() {
        let arr = Value::Array(vec![
            obj(vec![("a", 1i64.into())]),
            obj(vec![("a", 1i64.into()), ("b", 2i64.into())]),
        ]);
        let out = encode_toon(&arr);
        assert!(out.starts_with("[2]:"));
        assert!(out.contains("a: 1"));
        assert!(out.contains("b: 2"));
    }
}
