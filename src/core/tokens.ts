/**
 * Dependency-free token estimate for budget checks.
 *
 * A faithful BPE tokenizer is a runtime dependency we refuse to take (see
 * decisions/0001). Budgets only need a *stable, monotonic* proxy to catch files
 * growing past their limit, not exact GPT counts. We blend a character-based and
 * a word-based estimate: prose lands near 4 chars/token, while punctuation- and
 * symbol-heavy Markdown tokenizes denser, which the word term nudges upward.
 *
 * This intentionally over- rather than under-estimates so `doctor` warns early.
 */
export function estimateTokens(text: string): number {
  if (text.trim() === "") return 0;
  const chars = text.length;
  const words = text.match(/\S+/g)?.length ?? 0;
  const byChars = chars / 4;
  const byWords = words / 0.75; // ~0.75 words per token for English
  return Math.ceil((byChars + byWords) / 2);
}
