/**
 * The freshness table in `MEMORY.md` is the file-level counterpart to per-record
 * `last_verified`: it tracks whole-document staleness and each file's token budget.
 * `doctor` reads it to flag documents that have not been re-verified in a while.
 *
 * A row looks like: `| state/status.md | 2026-07-21 | 400 tokens |`.
 */

export interface FreshnessRow {
  file: string;
  lastVerified: string;
  budget: number;
}

const ROW_RE =
  /^\|\s*([^|]+?)\s*\|\s*(\d{4}-\d{2}-\d{2})\s*\|\s*(\d+)\s*tokens?\s*\|/;

export function parseFreshnessTable(memoryMd: string): FreshnessRow[] {
  const rows: FreshnessRow[] = [];
  for (const line of memoryMd.split(/\r?\n/)) {
    const m = ROW_RE.exec(line);
    if (m) rows.push({ file: m[1]!, lastVerified: m[2]!, budget: Number(m[3]) });
  }
  return rows;
}
