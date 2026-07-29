import { chmodSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

/**
 * Shared write-if-changed helper: every agnosgram-owned or managed-block target
 * (adapters, Claude Code hooks + skill) writes new content only when it actually
 * differs from what's on disk, and reports which of the three happened.
 */
export type WriteAction = "created" | "updated" | "unchanged";

export interface WriteResult {
  path: string;
  action: WriteAction;
}

export function writeIfChanged(
  root: string,
  relPath: string,
  content: string,
  opts: { executable?: boolean } = {},
): WriteResult {
  const target = join(root, relPath);
  const existedBefore = existsSync(target);
  const before = existedBefore ? readFileSync(target, "utf8") : null;
  if (before !== content) {
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, content);
  }
  if (opts.executable) chmodSync(target, 0o755);
  const action: WriteAction = before === content ? "unchanged" : existedBefore ? "updated" : "created";
  return { path: relPath, action };
}
