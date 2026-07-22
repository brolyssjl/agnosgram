import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

export const MEMORY_DIR = ".agnosgram";

/**
 * Find the project root: the nearest ancestor containing `.agnosgram/`, else the
 * nearest containing `.git/`, else the starting directory. Keeps the tool usable
 * from any subdirectory of a project, like git itself.
 */
export function findProjectRoot(start: string = process.cwd()): string {
  let dir = resolve(start);
  const seen: string[] = [];
  while (true) {
    if (existsSync(join(dir, MEMORY_DIR))) return dir;
    seen.push(dir);
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  // No existing store: fall back to the git root if there is one.
  dir = resolve(start);
  while (true) {
    if (existsSync(join(dir, ".git"))) return dir;
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return resolve(start);
}

export function memoryDir(root: string): string {
  return join(root, MEMORY_DIR);
}

export function configPath(root: string): string {
  return join(memoryDir(root), "config.yml");
}

export function hasStore(root: string): boolean {
  return existsSync(memoryDir(root));
}
