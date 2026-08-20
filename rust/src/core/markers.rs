//! Port of `src/core/markers.ts`: managed-block injection. Agnosgram owns
//! only the text between its markers in an agent config file; everything
//! else the user wrote is preserved byte-for-byte. Rewriting is idempotent:
//! running it twice yields an identical file.

pub const START_MARKER: &str = "<!-- agnosgram:start -->";
pub const END_MARKER: &str = "<!-- agnosgram:end -->";

/// Byte ranges (start of `START_MARKER` .. end of the matching `END_MARKER`)
/// of every managed block in `text`, in order, non-overlapping. Mirrors the
/// TS lazy/global regex: each match's `END_MARKER` is the first one found
/// after its `START_MARKER`, and the next search resumes after that match.
fn find_blocks(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut search_from = 0usize;
    while let Some(start_rel) = text[search_from..].find(START_MARKER) {
        let start = search_from + start_rel;
        let after_start = start + START_MARKER.len();
        let Some(end_rel) = text[after_start..].find(END_MARKER) else {
            break;
        };
        let end = after_start + end_rel + END_MARKER.len();
        out.push((start, end));
        search_from = end;
    }
    out
}

/// Collapse runs of 3+ consecutive newlines down to exactly 2.
fn collapse_newline_runs(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\n' {
            let mut j = i;
            while j < chars.len() && chars[j] == '\n' {
                j += 1;
            }
            let run = j - i;
            for _ in 0..run.min(2) {
                out.push('\n');
            }
            i = j;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Wrap block body in markers with a note that the region is tool-managed.
pub fn wrap_managed_block(body: &str) -> String {
    format!(
        "{START_MARKER}\n<!-- Managed by agnosgram. Edits inside this block are overwritten on `agnosgram adapt`. -->\n{}\n{END_MARKER}",
        body.trim_end()
    )
}

/// Insert or replace the managed block in `existing`. When no block is
/// present the managed block is appended (with one blank-line separator) so
/// user content stays on top. When present, the first block is replaced in
/// place and any stray duplicate blocks are removed.
pub fn upsert_managed_block(existing: &str, body: &str) -> String {
    let managed = wrap_managed_block(body);
    let blocks = find_blocks(existing);
    if blocks.is_empty() {
        let base = existing.trim_end();
        return if base.is_empty() {
            format!("{managed}\n")
        } else {
            format!("{base}\n\n{managed}\n")
        };
    }

    let mut result = String::new();
    let mut last = 0usize;
    let mut replaced = false;
    for &(start, end) in &blocks {
        result.push_str(&existing[last..start]);
        if !replaced {
            result.push_str(&managed);
            replaced = true;
        }
        // else: collapse accidental duplicate blocks (push nothing)
        last = end;
    }
    result.push_str(&existing[last..]);

    // Clean up any blank runs left by removed duplicates, keep a trailing newline.
    format!("{}\n", collapse_newline_runs(&result).trim_end())
}

pub fn has_managed_block(existing: &str) -> bool {
    !find_blocks(existing).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_a_managed_block_to_user_content_preserving_it() {
        let existing = "# My rules\n\nAlways be nice.\n";
        let out = upsert_managed_block(existing, "BODY");
        assert!(out.starts_with("# My rules\n\nAlways be nice."));
        assert!(out.contains(START_MARKER));
        assert!(out.contains(END_MARKER));
        assert!(out.contains("BODY"));
    }

    #[test]
    fn is_idempotent_running_twice_yields_identical_output() {
        let existing = "# My rules\n\nkeep me\n";
        let once = upsert_managed_block(existing, "BODY v1");
        let twice = upsert_managed_block(&once, "BODY v1");
        assert_eq!(once, twice);
    }

    #[test]
    fn replaces_the_block_body_without_touching_surrounding_user_content() {
        let existing = "top\n";
        let v1 = upsert_managed_block(existing, "OLD");
        let v2 = upsert_managed_block(&format!("{v1}\nuser added this later\n"), "NEW");
        assert!(v2.contains("NEW"));
        assert!(!v2.contains("OLD"));
        assert!(v2.starts_with("top"));
        assert!(v2.contains("user added this later"));
    }

    #[test]
    fn collapses_accidental_duplicate_blocks_into_one() {
        let block = format!("{START_MARKER}\nx\n{END_MARKER}");
        let existing = format!("a\n\n{block}\n\nb\n\n{block}\n");
        let out = upsert_managed_block(&existing, "ONE");
        let count = out.split(START_MARKER).count() - 1;
        assert_eq!(count, 1);
        assert!(out.contains('a'));
        assert!(out.contains('b'));
    }

    #[test]
    fn handles_empty_input() {
        let out = upsert_managed_block("", "BODY");
        assert!(has_managed_block(&out));
    }
}
