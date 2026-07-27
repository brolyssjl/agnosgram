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
