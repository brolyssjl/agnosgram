//! Port of `src/core/freshness.ts`: the freshness table in `MEMORY.md` is the
//! file-level counterpart to per-record `last_verified` - it tracks
//! whole-document staleness and each file's token budget. `doctor` reads it
//! to flag documents that have not been re-verified in a while.
//!
//! A row looks like: `| state/status.md | 2026-07-21 | 400 tokens |`.

pub struct FreshnessRow {
    pub file: String,
    pub last_verified: String,
    #[allow(dead_code)] // parsed for the on-disk shape; doctor never reads it (same in TS)
    pub budget: i64,
}

/// Every freshness-table row found in `memory_md`, in document order.
/// Non-matching lines (headers, separators, prose) are skipped.
pub fn parse_freshness_table(memory_md: &str) -> Vec<FreshnessRow> {
    let mut rows = Vec::new();
    for line in memory_md.lines() {
        if let Some(row) = parse_row(line) {
            rows.push(row);
        }
    }
    rows
}

/// Match `^\|\s*([^|]+?)\s*\|\s*(\d{4}-\d{2}-\d{2})\s*\|\s*(\d+)\s*tokens?\s*\|`
/// against one line.
fn parse_row(line: &str) -> Option<FreshnessRow> {
    // `^\|` then three more `|`-delimited fields: `splitn(5, '|')` isolates
    // them (`parts[0]` is whatever precedes the first `|`, which must be
    // empty for the line to start with it; `parts[4]` is unparsed trailing
    // content, since the regex is not end-anchored).
    let parts: Vec<&str> = line.splitn(5, '|').collect();
    if parts.len() < 5 || !parts[0].is_empty() {
        return None;
    }

    let field1 = parts[1];
    if field1.is_empty() {
        return None;
    }
    let file = field1.trim().to_string();

    let last_verified = parts[2].trim();
    if !looks_like_iso_date(last_verified) {
        return None;
    }

    let budget = parse_budget_field(parts[3])?;

    Some(FreshnessRow {
        file,
        last_verified: last_verified.to_string(),
        budget,
    })
}

fn looks_like_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[0..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit)
}

/// Match `\s*(\d+)\s*tokens?\s*` against one field, returning the digits.
fn parse_budget_field(field: &str) -> Option<i64> {
    let mut chars = field.chars().peekable();
    while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
        chars.next();
    }
    let mut digits = String::new();
    while matches!(chars.peek(), Some(c) if c.is_ascii_digit()) {
        digits.push(chars.next().unwrap());
    }
    if digits.is_empty() {
        return None;
    }
    while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
        chars.next();
    }
    for expected in "token".chars() {
        if chars.next() != Some(expected) {
            return None;
        }
    }
    if chars.peek() == Some(&'s') {
        chars.next();
    }
    if chars.any(|c| !c.is_whitespace()) {
        return None;
    }
    digits.parse::<i64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rows_from_a_memory_md_freshness_table() {
        let md = "## Freshness\n\
            | File | Last verified | Budget |\n\
            |------|---------------|--------|\n\
            | state/status.md | 2026-07-21 | 400 tokens |\n\
            | context/architecture.md | 2026-07-20 | 1500 tokens |\n";
        let rows = parse_freshness_table(md);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].file, "state/status.md");
        assert_eq!(rows[0].last_verified, "2026-07-21");
        assert_eq!(rows[0].budget, 400);
        assert_eq!(rows[1].budget, 1500);
    }

    #[test]
    fn ignores_the_header_and_separator_rows() {
        let md = "| File | Last verified | Budget |\n|---|---|---|\n";
        assert!(parse_freshness_table(md).is_empty());
    }
}
