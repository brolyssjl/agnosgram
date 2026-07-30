/**
 * Read-side helpers for walking a `.agnosgram/` store: enumerate its Markdown
 * files, and split them into the two kinds `doctor` cares about - record-bearing
 * files (lessons, decisions) whose frontmatter is validated, and everything else,
 * which is still scanned for links and safety lints.
 */

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { memoryDir } from "./paths.js";

export interface StoreFile {
  /** Absolute path on disk. */
  path: string;
  /** Path relative to the project root, e.g. `.agnosgram/lessons/pitfalls.md`. */
  rel: string;
  /** Path relative to the store dir, e.g. `lessons/pitfalls.md`. */
  storeRel: string;
  text: string;
  /** True when records should be extracted (lessons/*.md and decisions NNNN-*.md). */
  recordBearing: boolean;
}

const DECISION_FILE_RE = /^\d{4}-.*\.md$/;

function walk(dir: string, acc: string[]): void {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) walk(full, acc);
    else if (entry.isFile()) acc.push(full);
  }
}

/** Whether a store-relative Markdown path holds schema records. */
function isRecordBearing(storeRel: string, base: string): boolean {
  const parts = storeRel.split("/");
  if (parts[0] === "lessons" && storeRel.endsWith(".md")) return true;
  if (parts[0] === "decisions" && DECISION_FILE_RE.test(base)) return true;
  // meta/ (tool-friction feedback, see core/meta.ts) validates against its
  // own type enum, not the frozen lessons/decisions one - doctor picks the
  // right validator by path. `records.ts` explicitly excludes meta/ from the
  // shared retrieval index regardless of this flag.
  if (parts[0] === "meta" && storeRel.endsWith(".md")) return true;
  return false;
}

export function readStore(root: string): StoreFile[] {
  const dir = memoryDir(root);
  const files: string[] = [];
  walk(dir, files);

  const out: StoreFile[] = [];
  for (const path of files) {
    if (!path.endsWith(".md")) continue;
    const storeRel = relative(dir, path).split("\\").join("/");
    // Skip archived journals; they are frozen history, not live memory.
    if (storeRel.startsWith("journal/archive/")) continue;
    const base = storeRel.split("/").pop()!;
    out.push({
      path,
      rel: relative(root, path).split("\\").join("/"),
      storeRel,
      text: readFileSync(path, "utf8"),
      recordBearing: isRecordBearing(storeRel, base),
    });
  }
  out.sort((a, b) => a.storeRel.localeCompare(b.storeRel));
  return out;
}

/** True if a filesystem path exists (file or dir). */
export function pathExists(path: string): boolean {
  try {
    statSync(path);
    return true;
  } catch {
    return false;
  }
}

/** Journal months present in the store (`journal/YYYY-MM.md`), oldest first. */
export function journalMonths(root: string): string[] {
  const dir = join(memoryDir(root), "journal");
  if (!pathExists(dir)) return [];
  return readdirSync(dir)
    .filter((f) => /^\d{4}-\d{2}\.md$/.test(f))
    .map((f) => f.slice(0, 7))
    .sort();
}
