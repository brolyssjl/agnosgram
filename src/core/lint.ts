/**
 * Safety lints for the memory store. Memory is read by agents at the start of
 * every session, so it is a supply-chain surface of its own: a leaked secret is a
 * real leak, and a planted imperative ("ignore prior instructions ...") is a
 * prompt-injection vector. `doctor` runs these patterns over every stored file.
 *
 * These are heuristics tuned to flag, not to prove. False positives are cheaper
 * than a missed secret, so patterns lean sensitive; the human reviews each hit.
 */

export interface LintPattern {
  code: string;
  label: string;
  re: RegExp;
}

/** High-signal secret shapes. Matching one is treated as an error. */
export const SECRET_PATTERNS: LintPattern[] = [
  { code: "aws-access-key", label: "AWS access key id", re: /\bAKIA[0-9A-Z]{16}\b/ },
  {
    code: "private-key",
    label: "PEM private key block",
    re: /-----BEGIN (?:RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY-----/,
  },
  { code: "github-token", label: "GitHub token", re: /\bgh[posru]_[A-Za-z0-9]{36,}\b/ },
  { code: "slack-token", label: "Slack token", re: /\bxox[baprs]-[A-Za-z0-9-]{10,}\b/ },
  { code: "google-api-key", label: "Google API key", re: /\bAIza[0-9A-Za-z_-]{35}\b/ },
  {
    code: "generic-secret",
    label: "hard-coded secret assignment",
    re: /\b(?:api[_-]?key|secret|password|passwd|access[_-]?token|auth[_-]?token)\b\s*[:=]\s*['"][^'"\s]{12,}['"]/i,
  },
];

/**
 * Imperative phrasings that could hijack an agent reading the store. Matching one
 * is a warning: memory should record facts and lessons, never issue commands to
 * override the agent or run destructive/exfiltrating actions.
 */
export const INJECTION_PATTERNS: LintPattern[] = [
  {
    code: "ignore-instructions",
    label: "instruction-override phrasing",
    re: /\bignore\s+(?:all\s+)?(?:previous|prior|above|earlier)\s+instructions?\b/i,
  },
  {
    code: "disregard-instructions",
    label: "instruction-override phrasing",
    re: /\bdisregard\s+(?:all\s+)?(?:previous|prior|the\s+above|earlier)\b/i,
  },
  {
    code: "role-override",
    label: "role/system override",
    re: /\byou\s+are\s+now\s+(?:a|an|the)\b|\bnew\s+system\s+prompt\b|\boverride\s+your\s+(?:instructions|rules|guidelines)\b/i,
  },
  {
    code: "exfiltration",
    label: "data-exfiltration imperative",
    re: /\b(?:exfiltrate|leak|upload|send|post)\b[^.\n]{0,50}\b(?:secret|token|password|credential|api[_-]?key|env(?:ironment)?\s+var|\.env)\b/i,
  },
  {
    code: "destructive-shell",
    label: "destructive shell command",
    re: /\brm\s+-rf\b|(?:\bcurl\b|\bwget\b)[^\n]*\|\s*(?:sudo\s+)?(?:sh|bash)\b/i,
  },
];

export interface LintHit {
  code: string;
  label: string;
  /** 1-based line number of the match. */
  line: number;
  /** The matched fragment (trimmed), for the report. */
  match: string;
}

/** Run a pattern set over text and return every match with its line number. */
export function scanPatterns(text: string, patterns: LintPattern[]): LintHit[] {
  const hits: LintHit[] = [];
  const lines = text.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!;
    for (const p of patterns) {
      const m = p.re.exec(line);
      if (m) hits.push({ code: p.code, label: p.label, line: i + 1, match: m[0].trim() });
    }
  }
  return hits;
}
