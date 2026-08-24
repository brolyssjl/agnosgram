//! Port of `src/core/freshness.ts`: the freshness table in `MEMORY.md` is the
//! file-level counterpart to per-record `last_verified` - it tracks
//! whole-document staleness and each file's token budget. `doctor` reads it
//! to flag documents that have not been re-verified in a while.
//!
//! A row looks like: `| state/status.md | 2026-07-21 | 400 tokens |`.
//!
//! This module also owns the recall-freshness signals `doctor` and `pack`
//! both need: the newest dated entry in the journal, and status.md's own
//! recorded freshness date. Both read *content* dates only - never file
//! mtimes, which git does not preserve across a clone/checkout.

use super::dates::is_valid_iso_date;
use super::store::StoreFile;

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

/// The newest `YYYY-MM-DD` date named in a journal entry heading (`## YYYY-MM-DD
/// HH:MM \u{b7} agent \u{b7} branch`) across every non-archived journal month
/// file in the store, or `None` when the journal has no real entries yet -
/// only the scaffold's commented-out example heading, which never matches
/// (its date field reads literally `DD`, not two digits).
pub fn newest_journal_entry_date(store: &[StoreFile]) -> Option<String> {
    let mut newest: Option<String> = None;
    for file in store {
        if !file.store_rel.starts_with("journal/") {
            continue;
        }
        for line in file.text.lines() {
            let Some(date) = entry_heading_date(line) else {
                continue;
            };
            if newest.as_deref().is_none_or(|n| date.as_str() > n) {
                newest = Some(date);
            }
        }
    }
    newest
}

fn entry_heading_date(line: &str) -> Option<String> {
    let rest = line.strip_prefix("## ")?;
    let candidate = rest.get(0..10)?;
    is_valid_iso_date(candidate).then(|| candidate.to_string())
}

/// `state/status.md`'s recorded freshness date: the `MEMORY.md` freshness
/// table's row for it when present (the same signal the `file.stale` check
/// already trusts), falling back to a dated `Last updated: YYYY-MM-DD` line
/// inside `status.md` itself when the table carries no row for it.
pub fn status_freshness_date(store: &[StoreFile]) -> Option<String> {
    if let Some(memory_file) = store.iter().find(|f| f.store_rel == "MEMORY.md") {
        if let Some(row) = parse_freshness_table(&memory_file.text)
            .into_iter()
            .find(|r| r.file == "state/status.md")
        {
            return Some(row.last_verified);
        }
    }
    let status_file = store.iter().find(|f| f.store_rel == "state/status.md")?;
    last_updated_date(&status_file.text)
}

fn last_updated_date(text: &str) -> Option<String> {
    let marker = "Last updated:";
    let idx = text.find(marker)?;
    let candidate: String = text[idx + marker.len()..]
        .trim_start()
        .chars()
        .take(10)
        .collect();
    is_valid_iso_date(&candidate).then_some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_file(store_rel: &str, text: &str) -> StoreFile {
        StoreFile {
            path: std::path::PathBuf::new(),
            rel: format!(".agnosgram/{store_rel}"),
            store_rel: store_rel.to_string(),
            text: text.to_string(),
            record_bearing: false,
        }
    }

    #[test]
    fn newest_journal_entry_date_finds_the_latest_real_heading() {
        let store = vec![
            store_file(
                "journal/2026-07.md",
                "# Journal - 2026-07\n\n## 2026-07-05 10:00 \u{b7} claude\n- **Did:** thing\n",
            ),
            store_file(
                "journal/2026-08.md",
                "# Journal - 2026-08\n\n## 2026-08-24 19:34 \u{b7} gate \u{b7} feature/x\n- **Did:** other\n",
            ),
        ];
        assert_eq!(
            newest_journal_entry_date(&store),
            Some("2026-08-24".to_string())
        );
    }

    #[test]
    fn newest_journal_entry_date_ignores_the_scaffold_placeholder_heading() {
        let store = vec![store_file(
            "journal/2026-08.md",
            "# Journal - 2026-08\n\n<!-- Entry format (newest at the bottom; `agnosgram log` appends here):\n\n## 2026-08-DD HH:MM \u{b7} <agent> \u{b7} <branch>\n- **Did:** ...\n\n-->\n",
        )];
        assert_eq!(newest_journal_entry_date(&store), None);
    }

    #[test]
    fn status_freshness_date_prefers_the_memory_md_freshness_table() {
        let store = vec![
            store_file(
                "MEMORY.md",
                "## Freshness\n| File | Last verified | Budget |\n|---|---|---|\n| state/status.md | 2026-07-30 | 400 tokens |\n",
            ),
            store_file("state/status.md", "# Status\n\n_Last updated: 2026-08-20_\n"),
        ];
        assert_eq!(
            status_freshness_date(&store),
            Some("2026-07-30".to_string())
        );
    }

    #[test]
    fn status_freshness_date_falls_back_to_the_dated_line_in_status_md() {
        let store = vec![
            store_file(
                "MEMORY.md",
                "## Freshness\n| File | Last verified | Budget |\n|---|---|---|\n",
            ),
            store_file(
                "state/status.md",
                "# Status\n\n_Last updated: 2026-08-20_\n",
            ),
        ];
        assert_eq!(
            status_freshness_date(&store),
            Some("2026-08-20".to_string())
        );
    }

    #[test]
    fn status_freshness_date_is_none_when_neither_signal_is_present() {
        let store = vec![store_file("state/status.md", "# Status\n\nno date here\n")];
        assert_eq!(status_freshness_date(&store), None);
    }

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
