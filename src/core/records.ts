/**
 * Shared record index: every valid frontmatter record in the store, in one
 * place, for the retrieval commands (`show`, `pack`, `advise`) to filter and
 * sort. Built on `store.ts` (file enumeration) + `frontmatter.ts` (parsing and
 * schema validation). Invalid records are silently skipped here - `doctor` owns
 * diagnosing them, so retrieval never surfaces a half-parsed record.
 */
import { extractRecords, validateRecord, type Frontmatter } from "./frontmatter.js";
import { readStore } from "./store.js";

export interface StoreRecord {
  frontmatter: Frontmatter;
  body: string;
  /** Path relative to the project root, e.g. `.agnosgram/lessons/pitfalls.md`. */
  file: string;
  /** Path relative to the store dir, e.g. `lessons/pitfalls.md`. */
  storeRel: string;
  /** 1-based line of the record's opening `---` fence, for diagnostics. */
  line: number;
}

/** Every schema-valid record across the store's record-bearing files. */
export function loadRecords(root: string): StoreRecord[] {
  const out: StoreRecord[] = [];
  for (const file of readStore(root)) {
    if (!file.recordBearing) continue;
    for (const raw of extractRecords(file.text)) {
      const { frontmatter } = validateRecord(raw);
      if (!frontmatter) continue;
      out.push({
        frontmatter,
        body: raw.body,
        file: file.rel,
        storeRel: file.storeRel,
        line: raw.line,
      });
    }
  }
  return out;
}

/** Every distinct scope tag used across the store's valid records, sorted. */
export function allScopes(root: string): string[] {
  const scopes = new Set<string>();
  for (const rec of loadRecords(root)) {
    for (const s of rec.frontmatter.scope) scopes.add(s);
  }
  return [...scopes].sort();
}

/** Render a record's frontmatter + body as it looks on disk (canonical form). */
export function renderRecordBlock(r: StoreRecord): string {
  const fm = r.frontmatter;
  const lines = [
    `id: ${fm.id}`,
    `type: ${fm.type}`,
    `scope: [${fm.scope.join(", ")}]`,
    `confidence: ${fm.confidence}`,
    `created: ${fm.created}`,
    `last_verified: ${fm.last_verified}`,
    `source: ${fm.source}`,
  ];
  if (fm.supersedes && fm.supersedes.length > 0) {
    lines.push(`supersedes: [${fm.supersedes.join(", ")}]`);
  }
  return `---\n${lines.join("\n")}\n---\n${r.body}`;
}

/**
 * Structured shape for a record in `--format json|toon` output: a flat object
 * so an array of these stays TOON-tabular (a nested `scope`/`supersedes` array
 * would disqualify the tabular encoding) - list fields join with commas.
 */
export interface FlatRecord {
  id: string;
  type: string;
  scope: string;
  confidence: string;
  created: string;
  last_verified: string;
  source: string;
  supersedes: string;
  file: string;
  body: string;
}

export function toFlatRecord(r: StoreRecord): FlatRecord {
  return {
    id: r.frontmatter.id,
    type: r.frontmatter.type,
    scope: r.frontmatter.scope.join(","),
    confidence: r.frontmatter.confidence,
    created: r.frontmatter.created,
    last_verified: r.frontmatter.last_verified,
    source: r.frontmatter.source,
    supersedes: (r.frontmatter.supersedes ?? []).join(","),
    file: r.file,
    body: r.body,
  };
}

/** Confidence desc, last_verified desc, id asc - the shared record ordering (`show`, `pack`). */
export function compareRecords(a: StoreRecord, b: StoreRecord): number {
  const rank: Record<string, number> = { high: 3, medium: 2, low: 1 };
  const ca = rank[a.frontmatter.confidence] ?? 0;
  const cb = rank[b.frontmatter.confidence] ?? 0;
  if (ca !== cb) return cb - ca;
  if (a.frontmatter.last_verified !== b.frontmatter.last_verified) {
    return a.frontmatter.last_verified < b.frontmatter.last_verified ? 1 : -1;
  }
  return a.frontmatter.id.localeCompare(b.frontmatter.id);
}
