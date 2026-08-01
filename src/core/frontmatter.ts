/**
 * Record frontmatter: the schema every curated memory entry (lessons, decisions)
 * carries so `doctor` can validate it and `distill` can produce it mechanically.
 *
 * A record is a YAML frontmatter block (`--- ... ---`) followed by a Markdown
 * body. A single file may hold many records (lessons) or exactly one (a decision).
 * This module extracts records from a file and validates their frontmatter against
 * the frozen schema. It only ever *reads* frontmatter - records are authored by
 * humans or by agents following the `distill` prompt, never serialized here.
 */

import { isValidIsoDate } from "./dates.js";
import { parseYaml, type YamlValue } from "./yaml.js";

export const KNOWN_TYPES = ["pitfall", "convention", "decision"] as const;
export const KNOWN_CONFIDENCE = ["low", "medium", "high"] as const;

/** Ids look like `LES-001`, `CON-002`, `DEC-0001`: uppercase prefix + number. */
export const ID_PATTERN = /^[A-Z]{2,}-\d{2,}$/;

export interface Frontmatter {
  id: string;
  type: string;
  scope: string[];
  confidence: string;
  created: string;
  last_verified: string;
  source: string;
  supersedes?: string[];
}

export interface RawRecord {
  /** Parsed YAML frontmatter map (empty if the block failed to parse). */
  data: Record<string, YamlValue>;
  /** Markdown body after the closing fence, trimmed. */
  body: string;
  /** 1-based line of the opening `---` fence, for diagnostics. */
  line: number;
  /** Set when the frontmatter YAML could not be parsed at all. */
  parseError?: string;
}

export interface RecordIssue {
  level: "error" | "warn";
  code: string;
  message: string;
  line?: number;
}

export interface ValidatedRecord {
  raw: RawRecord;
  frontmatter?: Frontmatter;
  issues: RecordIssue[];
}

/**
 * Replace HTML-comment and fenced-code spans with blanks, preserving line
 * numbers. This keeps template example-entries (which live inside `<!-- -->`) and
 * documentation snippets (inside ``` fences) from being mistaken for real records.
 */
function maskNonRecordSpans(text: string): string {
  const blank = (m: string) => m.replace(/[^\n]/g, " ");
  return text
    .replace(/<!--[\s\S]*?-->/g, blank)
    .replace(/```[\s\S]*?```/g, blank);
}

/**
 * A line that can appear inside a frontmatter block: a `key:` line, a block-list
 * item, an indented continuation, or nothing. Used to tell a record's opening
 * fence apart from a Markdown thematic break (`---`) inside a body.
 */
const YAMLISH_LINE = /^(\s+\S.*|[A-Za-z_][A-Za-z0-9_-]*:(\s.*)?|-\s.*|-)$/;

/** Extract every frontmatter record from a Markdown file's text. */
export function extractRecords(text: string): RawRecord[] {
  // Fences are detected on the masked text (so examples inside comments and
  // code fences are invisible), but YAML and bodies are read from the original
  // lines so real body content - including fenced code - is preserved.
  const original = text.split(/\r?\n/);
  const masked = maskNonRecordSpans(text).split(/\r?\n/);

  // A `---` opens a record only when the lines up to the next `---` are all
  // YAML-shaped (and there is at least one). A thematic break followed by prose
  // is body content, not a fence - the frozen format allows `---` in bodies.
  const bounds: Array<{ open: number; close: number }> = [];
  let i = 0;
  while (i < masked.length) {
    if (masked[i]!.trim() !== "---") {
      i++;
      continue;
    }
    let j = i + 1;
    while (j < masked.length && masked[j]!.trim() !== "---") j++;
    const block = masked.slice(i + 1, j).filter((l) => l.trim() !== "");
    const isRecord =
      j < masked.length && block.length > 0 && block.every((l) => YAMLISH_LINE.test(l.trimEnd()));
    if (!isRecord) {
      i++;
      continue;
    }
    bounds.push({ open: i, close: j });
    i = j + 1;
  }

  const records: RawRecord[] = [];
  for (let k = 0; k < bounds.length; k++) {
    const { open, close } = bounds[k]!;
    const yamlText = original.slice(open + 1, close).join("\n");
    const nextOpen = bounds[k + 1]?.open ?? original.length;
    const body = original.slice(close + 1, nextOpen).join("\n").trim();

    let data: Record<string, YamlValue> = {};
    let parseError: string | undefined;
    try {
      const parsed = parseYaml(yamlText);
      if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
        data = parsed as Record<string, YamlValue>;
      } else {
        parseError = "frontmatter is not a key/value map";
      }
    } catch (err) {
      parseError = err instanceof Error ? err.message : String(err);
    }

    records.push({ data, body, line: open + 1, ...(parseError ? { parseError } : {}) });
  }
  return records;
}

function asStringList(value: YamlValue | undefined): string[] | undefined {
  if (typeof value === "string") return value.trim() === "" ? [] : [value];
  if (Array.isArray(value)) {
    return value.filter((v) => v !== null && v !== undefined).map((v) => String(v));
  }
  return undefined;
}

/**
 * Validate one raw record's frontmatter against the schema. `allowedTypes`
 * defaults to the frozen lessons/decisions enum (`KNOWN_TYPES`); passing a
 * different list lets other additive namespaces (e.g. `meta/`) reuse this
 * same field-by-field validation against their own type enum, without
 * touching the frozen lessons/decisions contract itself.
 */
export function validateRecord(
  raw: RawRecord,
  allowedTypes: readonly string[] = KNOWN_TYPES,
): ValidatedRecord {
  const issues: RecordIssue[] = [];
  const err = (code: string, message: string) =>
    issues.push({ level: "error", code, message, line: raw.line });
  const warn = (code: string, message: string) =>
    issues.push({ level: "warn", code, message, line: raw.line });

  if (raw.parseError) {
    // Field-level checks against an empty map would add seven "missing field"
    // errors that are pure noise next to the actual problem; report only the
    // parse failure so the fix is obvious.
    err("frontmatter.parse", `frontmatter did not parse: ${raw.parseError}`);
    return { raw, issues };
  }

  const d = raw.data;

  const id = typeof d.id === "string" ? d.id : undefined;
  if (id === undefined) err("id.missing", "missing required field `id`");
  else if (!ID_PATTERN.test(id)) err("id.format", `id "${id}" must look like ABC-123`);

  const type = typeof d.type === "string" ? d.type : undefined;
  if (type === undefined) err("type.missing", "missing required field `type`");
  else if (!allowedTypes.includes(type)) {
    err("type.unknown", `type "${type}" is not one of ${allowedTypes.join(", ")}`);
  }

  const scope = asStringList(d.scope);
  if (scope === undefined || scope.length === 0) {
    err("scope.missing", "missing or empty `scope` (expected a non-empty list)");
  }

  const confidence = typeof d.confidence === "string" ? d.confidence : undefined;
  if (confidence === undefined) err("confidence.missing", "missing required field `confidence`");
  else if (!(KNOWN_CONFIDENCE as readonly string[]).includes(confidence)) {
    err("confidence.unknown", `confidence "${confidence}" is not one of ${KNOWN_CONFIDENCE.join(", ")}`);
  }

  const created = typeof d.created === "string" ? d.created : undefined;
  if (created === undefined) err("created.missing", "missing required field `created`");
  else if (!isValidIsoDate(created)) err("created.format", `created "${created}" is not a valid YYYY-MM-DD date`);

  const lastVerified = typeof d.last_verified === "string" ? d.last_verified : undefined;
  if (lastVerified === undefined) err("last_verified.missing", "missing required field `last_verified`");
  else if (!isValidIsoDate(lastVerified)) {
    err("last_verified.format", `last_verified "${lastVerified}" is not a valid YYYY-MM-DD date`);
  }

  if (
    created !== undefined &&
    lastVerified !== undefined &&
    isValidIsoDate(created) &&
    isValidIsoDate(lastVerified) &&
    lastVerified < created
  ) {
    warn("date.order", "`last_verified` is earlier than `created`");
  }

  const source = typeof d.source === "string" ? d.source : undefined;
  if (source === undefined) err("source.missing", "missing required field `source`");

  let supersedes: string[] | undefined;
  if (d.supersedes !== undefined && d.supersedes !== null) {
    supersedes = asStringList(d.supersedes);
    if (supersedes === undefined) {
      err("supersedes.format", "`supersedes` must be an id or a list of ids");
    } else {
      for (const ref of supersedes) {
        if (!ID_PATTERN.test(ref)) err("supersedes.format", `supersedes "${ref}" must look like ABC-123`);
      }
    }
  }

  if (raw.body.trim() === "") warn("body.empty", "record has no body text");

  const hasError = issues.some((i) => i.level === "error");
  let frontmatter: Frontmatter | undefined;
  if (!hasError) {
    frontmatter = {
      id: id!,
      type: type!,
      scope: scope!,
      confidence: confidence!,
      created: created!,
      last_verified: lastVerified!,
      source: source!,
      ...(supersedes && supersedes.length > 0 ? { supersedes } : {}),
    };
  }

  return { raw, frontmatter, issues };
}
