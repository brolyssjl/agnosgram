/**
 * Shared record index: every valid frontmatter record in the store, in one
 * place, for the retrieval commands (`show`, `pack`, `advise`) to filter and
 * sort. Built on `store.ts` (file enumeration) + `frontmatter.ts` (parsing and
 * schema validation). Invalid records are silently skipped here - `doctor` owns
 * diagnosing them, so retrieval never surfaces a half-parsed record.
 */
import { extractRecords, validateRecord, type Frontmatter, type RawRecord } from "./frontmatter.js";
import { readStore, type StoreFile } from "./store.js";

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

/**
 * Low-level walk shared by every command that needs raw (unvalidated)
 * records: the record-bearing files, split into their raw frontmatter
 * blocks. `loadRecords` (below) and `distill.ts`'s `existingIds` both
 * delegate to this instead of repeating `readStore -> recordBearing ->
 * extractRecords`. `doctor` keeps its own walk since it also needs to
 * report on invalid records, which this intentionally does not surface.
 */
export function* iterRawRecords(root: string): Generator<{ file: StoreFile; raw: RawRecord }> {
  for (const file of readStore(root)) {
    if (!file.recordBearing) continue;
    // meta/ is tool-friction feedback, not host-project memory - it must
    // never surface through the shared record index that pack/show/advise
    // and distill's id-collision check read from. `doctor` (and `reflect`,
    // via core/meta.ts) validate/read meta/ separately, off readStore()
    // directly. This exclusion is explicit here rather than relying on
    // schema validation to reject it, so it holds regardless of how the
    // default validator's type enum evolves.
    if (file.storeRel === "meta" || file.storeRel.startsWith("meta/")) continue;
    for (const raw of extractRecords(file.text)) {
      yield { file, raw };
    }
  }
}

/** Every schema-valid record across the store's record-bearing files. */
export function loadRecords(root: string): StoreRecord[] {
  const out: StoreRecord[] = [];
  for (const { file, raw } of iterRawRecords(root)) {
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
  return out;
}

/** Every distinct scope tag used across a set of records, sorted. */
export function allScopesFrom(records: StoreRecord[]): string[] {
  const scopes = new Set<string>();
  for (const rec of records) {
    for (const s of rec.frontmatter.scope) scopes.add(s);
  }
  return [...scopes].sort();
}

/** Every distinct scope tag used across the store's valid records, sorted. */
export function allScopes(root: string): string[] {
  return allScopesFrom(loadRecords(root));
}

/** Case-insensitive: does this record carry `tag` among its scope list? */
export function matchesScope(rec: StoreRecord, tag: string): boolean {
  const lower = tag.toLowerCase();
  return rec.frontmatter.scope.some((s) => s.toLowerCase() === lower);
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
