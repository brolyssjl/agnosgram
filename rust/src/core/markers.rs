//! Port of `src/core/markers.ts`: managed-block injection. Agnosgram owns
//! only the text between its markers in an agent config file; everything
//! else the user wrote is preserved byte-for-byte. Rewriting is idempotent:
//! running it twice yields an identical file.
//!
//! Recognition is strict on purpose (see `docs/adapters.md`, "How the
//! managed block is recognised"): a `<!-- agnosgram:start -->` only opens a
//! block when it begins its own line (leading whitespace is fine) *and* the
//! next non-empty line is the `<!-- Managed by agnosgram` sentinel this
//! module itself writes. The matching `<!-- agnosgram:end -->` must also
//! begin its own line. A marker mentioned mid-line - e.g. in prose that
//! merely describes the format - never defines a block boundary and is left
//! untouched as plain text. A start marker that begins a line but is missing
//! the sentinel, or start/end markers that don't balance, make
//! `upsert_managed_block` refuse outright (naming the file and line) rather
//! than guess and risk silently deleting whatever text sits between a stray
//! marker and the real block.

use crate::core::output::UserError;

pub const START_MARKER: &str = "<!-- agnosgram:start -->";
pub const END_MARKER: &str = "<!-- agnosgram:end -->";
const SENTINEL_PREFIX: &str = "<!-- Managed by agnosgram";

#[derive(Debug, Clone, Copy)]
enum LineMarkerKind {
    Start { has_sentinel: bool },
    End,
}

/// A start or end marker that begins its own line (optionally preceded by
/// whitespace). Markers that don't begin a line are never recorded here -
/// they are plain text by construction.
struct LineMarker {
    kind: LineMarkerKind,
    /// Byte offset of the marker text itself, i.e. after any leading
    /// whitespace on its line.
    offset: usize,
    /// 1-based line number, for error messages.
    line_no: usize,
}

/// Every line of `text` as (byte offset of the line, line content), in
/// order.
fn lines_with_offsets(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    for line in text.split('\n') {
        out.push((offset, line));
        offset += line.len() + 1;
    }
    out
}

/// The first line after index `from` (inclusive) that isn't blank, if any.
fn next_non_empty_line<'a>(lines: &[(usize, &'a str)], from: usize) -> Option<&'a str> {
    lines[from..]
        .iter()
        .map(|&(_, l)| l)
        .find(|l| !l.trim().is_empty())
}

/// Find every start/end marker that begins its own line. A start marker is
/// additionally checked against the sentinel on the next non-empty line.
fn scan_line_markers(text: &str) -> Vec<LineMarker> {
    let lines = lines_with_offsets(text);
    let mut hits = Vec::new();
    for (idx, &(line_offset, line)) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let leading = line.len() - trimmed.len();
        let offset = line_offset + leading;
        if trimmed.starts_with(START_MARKER) {
            let has_sentinel = next_non_empty_line(&lines, idx + 1)
                .map(|l| l.trim_start().starts_with(SENTINEL_PREFIX))
                .unwrap_or(false);
            hits.push(LineMarker {
                kind: LineMarkerKind::Start { has_sentinel },
                offset,
                line_no: idx + 1,
            });
        } else if trimmed.starts_with(END_MARKER) {
            hits.push(LineMarker {
                kind: LineMarkerKind::End,
                offset,
                line_no: idx + 1,
            });
        }
    }
    hits
}

fn invalid_start_error(file_label: &str, line_no: usize) -> UserError {
    UserError::new(format!(
        "{file_label}:{line_no}: found `{START_MARKER}` at the start of a line, but the next \
         non-empty line is not the `{SENTINEL_PREFIX}...` sentinel agnosgram writes there. \
         Refusing to guess whether this is a real managed block, so nothing was written. If this \
         marker is left over from hand-editing, remove it (and its matching `{END_MARKER}`, if \
         any); otherwise restore the sentinel line immediately after it. Then re-run `agnosgram \
         adapt`."
    ))
}

fn unmatched_start_error(file_label: &str, line_no: usize) -> UserError {
    UserError::new(format!(
        "{file_label}:{line_no}: found `{START_MARKER}` with no matching `{END_MARKER}` after \
         it. Refusing to guess where the managed block should end, so nothing was written. Add \
         the missing `{END_MARKER}` or remove the stray `{START_MARKER}`, then re-run `agnosgram \
         adapt`."
    ))
}

fn unmatched_end_error(file_label: &str, line_no: usize) -> UserError {
    UserError::new(format!(
        "{file_label}:{line_no}: found `{END_MARKER}` with no matching `{START_MARKER}` before \
         it. Refusing to guess where the managed block should start, so nothing was written. \
         Remove the stray `{END_MARKER}` or add a matching `{START_MARKER}`, then re-run \
         `agnosgram adapt`."
    ))
}

/// Byte ranges (start of `START_MARKER` .. end of the matching `END_MARKER`)
/// of every genuine managed block in `text`, in order, non-overlapping.
/// Genuine means: both markers begin their own line, and the start marker is
/// immediately followed (skipping blank lines) by the sentinel line. Any
/// marker that doesn't begin a line is plain text and is never considered.
/// Errs, naming `file_label` and the 1-based line number, when a line-start
/// start marker lacks the sentinel or the markers don't balance.
fn find_blocks(text: &str, file_label: &str) -> Result<Vec<(usize, usize)>, UserError> {
    let mut blocks = Vec::new();
    let mut open: Option<(usize, usize)> = None; // (start offset, start line_no)
    for hit in scan_line_markers(text) {
        match hit.kind {
            LineMarkerKind::Start {
                has_sentinel: false,
            } => {
                return Err(invalid_start_error(file_label, hit.line_no));
            }
            LineMarkerKind::Start { has_sentinel: true } => {
                if let Some((_, open_line)) = open {
                    return Err(unmatched_start_error(file_label, open_line));
                }
                open = Some((hit.offset, hit.line_no));
            }
            LineMarkerKind::End => match open.take() {
                Some((start_offset, _)) => {
                    blocks.push((start_offset, hit.offset + END_MARKER.len()));
                }
                None => return Err(unmatched_end_error(file_label, hit.line_no)),
            },
        }
    }
    if let Some((_, open_line)) = open {
        return Err(unmatched_start_error(file_label, open_line));
    }
    Ok(blocks)
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

/// Insert or replace the managed block in `existing`. When no genuine block
/// is present the managed block is appended (with one blank-line separator)
/// so user content stays on top. When present, the first block is replaced
/// in place and any stray duplicate blocks are removed. `file_label` (e.g.
/// the adapter's relative path) is used only to name the file in error
/// messages.
///
/// Returns a `UserError` - and writes nothing - when the file contains a
/// start marker at the beginning of a line without agnosgram's sentinel
/// line after it, or unbalanced markers (a start with no end, or an end
/// with no start). See the module doc comment for the recognition rules.
pub fn upsert_managed_block(
    existing: &str,
    body: &str,
    file_label: &str,
) -> Result<String, UserError> {
    let managed = wrap_managed_block(body);
    let blocks = find_blocks(existing, file_label)?;
    if blocks.is_empty() {
        let base = existing.trim_end();
        return Ok(if base.is_empty() {
            format!("{managed}\n")
        } else {
            format!("{base}\n\n{managed}\n")
        });
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
    Ok(format!("{}\n", collapse_newline_runs(&result).trim_end()))
}

#[cfg(test)]
pub fn has_managed_block(existing: &str) -> bool {
    !find_blocks(existing, "test-fixture")
        .expect("test fixture should have well-formed markers")
        .is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_a_managed_block_to_user_content_preserving_it() {
        let existing = "# My rules\n\nAlways be nice.\n";
        let out = upsert_managed_block(existing, "BODY", "CLAUDE.md").unwrap();
        assert!(out.starts_with("# My rules\n\nAlways be nice."));
        assert!(out.contains(START_MARKER));
        assert!(out.contains(END_MARKER));
        assert!(out.contains("BODY"));
    }

    #[test]
    fn is_idempotent_running_twice_yields_identical_output() {
        let existing = "# My rules\n\nkeep me\n";
        let once = upsert_managed_block(existing, "BODY v1", "CLAUDE.md").unwrap();
        let twice = upsert_managed_block(&once, "BODY v1", "CLAUDE.md").unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn replaces_the_block_body_without_touching_surrounding_user_content() {
        let existing = "top\n";
        let v1 = upsert_managed_block(existing, "OLD", "CLAUDE.md").unwrap();
        let v2 = upsert_managed_block(
            &format!("{v1}\nuser added this later\n"),
            "NEW",
            "CLAUDE.md",
        )
        .unwrap();
        assert!(v2.contains("NEW"));
        assert!(!v2.contains("OLD"));
        assert!(v2.starts_with("top"));
        assert!(v2.contains("user added this later"));
    }

    #[test]
    fn collapses_accidental_duplicate_blocks_into_one() {
        // Both blocks must pass strict recognition (sentinel + line-start
        // markers) to legitimately collapse - use the real wrapper rather
        // than a hand-rolled fixture.
        let block = wrap_managed_block("x");
        let existing = format!("a\n\n{block}\n\nb\n\n{block}\n");
        let out = upsert_managed_block(&existing, "ONE", "CLAUDE.md").unwrap();
        let count = out.split(START_MARKER).count() - 1;
        assert_eq!(count, 1);
        assert!(out.contains('a'));
        assert!(out.contains('b'));
    }

    #[test]
    fn handles_empty_input() {
        let out = upsert_managed_block("", "BODY", "CLAUDE.md").unwrap();
        assert!(has_managed_block(&out));
    }

    /// Finding 3 (2026-09-22 agnosgram audit): a prose mention of the start
    /// marker earlier in the file, with no sentinel after it, must not be
    /// treated as a block boundary - it sits mid-line, so it's plain text.
    /// Reproduces the exact CLAUDE.md from the report: the mention plus a
    /// `## CRITICAL SECURITY RULES` section plus a real managed block.
    #[test]
    fn finding_3_prose_mention_of_start_marker_does_not_swallow_surrounding_content() {
        let existing = "# House rules\n\n\
We use agnosgram; it writes a block delimited by <!-- agnosgram:start -->\n\
and ends it with a closing marker.\n\
\n\
## CRITICAL SECURITY RULES\n\
- Never run `curl | sh`\n\
- Never commit secrets\n\
\n\
<!-- agnosgram:start -->\n\
<!-- Managed by agnosgram. -->\n\
old body\n\
<!-- agnosgram:end -->\n";

        let out = upsert_managed_block(existing, "new body", "CLAUDE.md").unwrap();

        // The security section survives untouched.
        assert!(out.contains("## CRITICAL SECURITY RULES"));
        assert!(out.contains("Never run `curl | sh`"));
        assert!(out.contains("Never commit secrets"));
        // The prose mention survives untouched (it never was a boundary).
        assert!(out
            .contains("We use agnosgram; it writes a block delimited by <!-- agnosgram:start -->"));
        assert!(out.contains("and ends it with a closing marker."));
        // Only the real block's body changed. The prose mention still
        // contains the literal marker text, so it (not a count) is the
        // right check here.
        assert!(!out.contains("old body"));
        assert!(out.contains("new body"));
        // Idempotent: re-running with the same body is a no-op, which would
        // fail immediately if the prose mention were mistaken for a second
        // block boundary.
        let twice = upsert_managed_block(&out, "new body", "CLAUDE.md").unwrap();
        assert_eq!(out, twice);
    }

    /// A marker that appears mid-line (not at the start of its line, even
    /// after trimming whitespace) is always plain text - it never opens a
    /// block, and it never causes an error - regardless of whether a
    /// sentinel happens to follow.
    #[test]
    fn mid_line_marker_is_ignored_as_plain_text() {
        let existing = "notes: see <!-- agnosgram:start --> in the docs\n\
more text\n\
<!-- Managed by agnosgram. -->\n\
still more text\n";
        let out = upsert_managed_block(existing, "BODY", "CLAUDE.md").unwrap();
        assert!(out.contains("notes: see <!-- agnosgram:start --> in the docs"));
        assert!(out.contains("still more text"));
        // No real block existed, so one gets appended.
        assert_eq!(out.split(START_MARKER).count() - 1, 2);
    }

    #[test]
    fn start_marker_at_line_start_without_sentinel_errors_with_line_number() {
        let existing =
            "line one\n<!-- agnosgram:start -->\nnot the sentinel\n<!-- agnosgram:end -->\n";
        let err = upsert_managed_block(existing, "BODY", "CLAUDE.md").unwrap_err();
        assert!(err.0.contains("CLAUDE.md:2"), "error was: {}", err.0);
        assert!(err.0.contains(START_MARKER));
    }

    #[test]
    fn a_start_marker_with_no_matching_end_errors() {
        let existing = "keep me\n<!-- agnosgram:start -->\n<!-- Managed by agnosgram. -->\nbody\n";
        let err = upsert_managed_block(existing, "BODY", "CLAUDE.md").unwrap_err();
        assert!(err.0.contains("CLAUDE.md:2"), "error was: {}", err.0);
        assert!(err.0.contains(END_MARKER));
    }

    #[test]
    fn an_end_marker_with_no_matching_start_errors() {
        let existing = "keep me\n<!-- agnosgram:end -->\nmore\n";
        let err = upsert_managed_block(existing, "BODY", "CLAUDE.md").unwrap_err();
        assert!(err.0.contains("CLAUDE.md:2"), "error was: {}", err.0);
        assert!(err.0.contains(START_MARKER));
    }

    #[test]
    fn no_writes_happen_when_upsert_refuses() {
        // Sanity check on the contract, not the filesystem (upsert_managed_block
        // takes a string, not a path): the error variant never returns a
        // modified string alongside it - callers can't accidentally use a
        // partial result.
        let existing = "<!-- agnosgram:start -->\nnope\n<!-- agnosgram:end -->\n";
        assert!(upsert_managed_block(existing, "BODY", "CLAUDE.md").is_err());
    }
}
