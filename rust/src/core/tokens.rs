//! Port of `src/core/tokens.ts`: dependency-free token estimate for budget
//! checks.
//!
//! A faithful BPE tokenizer is a runtime dependency we refuse to take (see
//! `decisions/0001`). Budgets only need a *stable, monotonic* proxy to catch
//! files growing past their limit, not exact GPT counts. We blend a
//! character-based and a word-based estimate: prose lands near 4 chars/token,
//! while punctuation- and symbol-heavy Markdown tokenizes denser, which the
//! word term nudges upward.
//!
//! This intentionally over- rather than under-estimates so `doctor` warns
//! early.

/// Estimate a token count for `text`. `chars` counts UTF-16 code units, the
/// same unit as JS `string.length`, so multi-byte content estimates
/// identically to the TS implementation.
pub fn estimate_tokens(text: &str) -> i64 {
    if text.trim().is_empty() {
        return 0;
    }
    let chars = text.encode_utf16().count() as f64;
    let words = text.split_whitespace().count() as f64;
    let by_chars = chars / 4.0;
    let by_words = words / 0.75; // ~0.75 words per token for English
    ((by_chars + by_words) / 2.0).ceil() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_is_zero_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("   \n  "), 0);
    }

    #[test]
    fn estimate_grows_monotonically_with_content() {
        let small = estimate_tokens("hello world");
        let big = estimate_tokens(&"hello world ".repeat(50));
        assert!(big > small);
    }

    #[test]
    fn estimate_is_in_a_sane_range_for_a_paragraph() {
        // ~40 words; a real tokenizer lands near 50-60 tokens. Heuristic must
        // be close-ish.
        let text = "The quick brown fox jumps over the lazy dog. ".repeat(5);
        let t = estimate_tokens(&text);
        assert!(t > 40 && t < 120, "unexpected estimate {t}");
    }
}
