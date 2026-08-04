import { existsSync, lstatSync, readFileSync, readlinkSync, realpathSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { ADAPTER_KEYS, ADAPTERS, buildPointerBody, type Adapter, type SddHint } from "../adapters/index.js";
import { parseCliArgs } from "../core/args.js";
import { hooksInstalled, installClaudeHooks } from "../core/claudeHooks.js";
import { loadConfig, saveConfig, type AgnosgramConfig, type Toggle } from "../core/config.js";
import { AGENT_TARGETS, detectAgents, detectSdd } from "../core/detect.js";
import { upsertManagedBlock } from "../core/markers.js";
import { info, printJson, UserError } from "../core/output.js";
import { findProjectRoot, hasStore } from "../core/paths.js";
import { writeIfChanged, type WriteAction } from "../core/writeFile.js";

export type AdaptAction = WriteAction;

export interface AdaptResult {
  adapter: string;
  path: string;
  action: AdaptAction;
  /**
   * Other adapter keys collapsed into this result because their target path
   * is a symlink resolving to the same real file (FRI-003, e.g. CLAUDE.md ->
   * AGENTS.md) - the managed block was written once, through `path`, not
   * once per aliased adapter.
   */
  symlinkAliases?: { adapter: string; path: string }[];
}

/**
 * Which SDD frameworks are active for hint lines, given config + detection, each
 * paired with the specific directory that was actually found on disk. A
 * framework forced "on" in config without ever being detected gets `matchedPath:
 * null` - there's nothing real to point at, so the hint text says so instead of
 * fabricating a path.
 */
export function resolveSddHints(root: string, config: AgnosgramConfig): SddHint[] {
  const detected = new Map(detectSdd(root).map((f) => [f.key, f.matchedPath]));
  return Object.entries(config.sdd)
    .filter(([key, toggle]) => toggle === "on" || (toggle === "auto" && detected.has(key)))
    .map(([key]) => ({ key, matchedPath: detected.get(key) ?? null }));
}

/**
 * Resolve which path an adapter actually writes to for this project. Normally
 * `targetPath`, but a legacy single-file convention (e.g. Cline's `.clinerules`
 * file, predating the `.clinerules/` directory) takes over when that path already
 * exists as a plain file - never `mkdir` a directory over an existing file.
 */
function resolveAdapterPath(root: string, adapter: Adapter): string {
  if (adapter.legacyTargetPath) {
    const legacyAbs = join(root, adapter.legacyTargetPath);
    if (existsSync(legacyAbs) && statSync(legacyAbs).isFile()) {
      return adapter.legacyTargetPath;
    }
  }
  return adapter.targetPath;
}

/** Inject or refresh one adapter's managed block. Idempotent. */
export function applyAdapter(root: string, adapter: Adapter, sddHints: SddHint[]): AdaptResult {
  const relPath = resolveAdapterPath(root, adapter);
  const target = join(root, relPath);
  const body = buildPointerBody(sddHints);

  let existing = "";
  if (existsSync(target)) {
    existing = readFileSync(target, "utf8");
  } else if (adapter.dedicatedFile && adapter.preamble) {
    existing = adapter.preamble;
  }

  const next = upsertManagedBlock(existing, body);

  try {
    const result = writeIfChanged(root, relPath, next);
    return { adapter: adapter.key, path: result.path, action: result.action };
  } catch (err) {
    const detail = err instanceof Error ? err.message : String(err);
    throw new UserError(
      `Could not write ${adapter.name}'s adapter file at ${relPath}: ${detail}. ` +
        `Resolve the conflict (e.g. a file where a directory is expected) and re-run ` +
        `\`agnosgram adapt ${adapter.key}\`.`,
    );
  }
}

function isSymlink(abs: string): boolean {
  try {
    return lstatSync(abs).isSymbolicLink();
  } catch {
    return false;
  }
}

/**
 * The real file a target path ultimately refers to, resolving through a
 * symlink even when its target doesn't exist yet (e.g. CLAUDE.md is a
 * symlink to a not-yet-created AGENTS.md) - `realpathSync` alone would throw
 * in that case. Two adapters whose target paths resolve to the same real
 * path are the same file on disk (FRI-003), whichever names they're
 * requested under.
 */
function resolveRealPath(root: string, relPath: string): string {
  const abs = join(root, relPath);
  try {
    return realpathSync(abs);
  } catch {
    if (isSymlink(abs)) return resolve(dirname(abs), readlinkSync(abs));
    return abs;
  }
}

/**
 * Group requested adapter keys by the real file they resolve to, so a
 * symlinked pair (CLAUDE.md -> AGENTS.md, or the reverse) is written and
 * reported once instead of twice. Only collapses a group when at least one
 * member's path is an actual on-disk symlink - a coincidental real-path
 * match with no symlink involved is left as independent targets.
 */
function groupBySymlink(root: string, keys: string[]): string[][] {
  const byRealPath = new Map<string, string[]>();
  for (const key of keys) {
    const relPath = resolveAdapterPath(root, ADAPTERS[key]!);
    const real = resolveRealPath(root, relPath);
    const group = byRealPath.get(real) ?? [];
    group.push(key);
    byRealPath.set(real, group);
  }

  const groups: string[][] = [];
  for (const group of byRealPath.values()) {
    const isAlias = group.length > 1 && group.some((key) => isSymlink(join(root, resolveAdapterPath(root, ADAPTERS[key]!))));
    if (isAlias) {
      groups.push(group);
    } else {
      for (const key of group) groups.push([key]);
    }
  }
  return groups;
}

/** Apply one adapter, or a symlink-aliased group of adapters (FRI-003) that all
 * resolve to the same real file - every adapter injects the same generic
 * pointer body, so writing once through the non-symlink member is equivalent
 * to writing through every alias. */
function applyAdapterGroup(root: string, keys: string[], sddHints: SddHint[]): AdaptResult {
  if (keys.length === 1) return applyAdapter(root, ADAPTERS[keys[0]!]!, sddHints);

  const withPaths = keys.map((key) => ({ key, relPath: resolveAdapterPath(root, ADAPTERS[key]!) }));
  const primary = withPaths.find((t) => !isSymlink(join(root, t.relPath))) ?? withPaths[0]!;
  const aliases = withPaths.filter((t) => t.key !== primary.key);

  const result = applyAdapter(root, ADAPTERS[primary.key]!, sddHints);
  return { ...result, symlinkAliases: aliases.map((a) => ({ adapter: a.key, path: a.relPath })) };
}

/** Adapters that should be written when no explicit targets are given. */
export function resolveEnabledAdapters(root: string, config: AgnosgramConfig): string[] {
  const detected = new Set(detectAgents(root).map((a) => a.key));
  return ADAPTER_KEYS.filter((key) => {
    const toggle: Toggle = config.adapters[key] ?? "off";
    return toggle === "on" || (toggle === "auto" && detected.has(key));
  });
}

function validateTargets(targets: string[]): void {
  const unknown = targets.filter((t) => !ADAPTERS[t]);
  if (unknown.length > 0) {
    throw new UserError(
      `Unknown adapter(s): ${unknown.join(", ")}. Known: ${ADAPTER_KEYS.join(", ")}.`,
    );
  }
}

export function runAdapt(argv: string[]): void {
  const { values, positionals } = parseCliArgs({
    args: argv,
    allowPositionals: true,
    options: {
      all: { type: "boolean", default: false },
      refresh: { type: "boolean", default: false },
      json: { type: "boolean", default: false },
      "claude-hooks": { type: "boolean", default: false },
    },
  });

  const root = findProjectRoot();
  if (!hasStore(root)) {
    throw new UserError("No .agnosgram/ store found. Run `agnosgram init` first.");
  }

  const config = loadConfig(root);
  validateTargets(positionals);

  let targets: string[];
  if (positionals.length > 0) {
    targets = positionals;
    // An explicit `adapt <target>` turns that adapter on for future refreshes.
    for (const t of targets) config.adapters[t] = "on";
    saveConfig(root, config);
  } else if (values.all) {
    targets = ADAPTER_KEYS;
  } else {
    targets = resolveEnabledAdapters(root, config);
  }

  // --refresh also picks up hooks a prior run already installed, so an upgrade
  // (which regenerates the hook scripts' content) doesn't require remembering
  // to pass --claude-hooks again.
  const claudeHooks = values["claude-hooks"] || (values.refresh && hooksInstalled(root));

  if (targets.length === 0 && !claudeHooks) {
    const detectedHint = detectAgents(root)
      .map((a) => a.name)
      .join(", ");
    throw new UserError(
      "No adapters to write. Name one explicitly (e.g. `agnosgram adapt claude`), " +
        "use `--all`, enable adapters in config.yml, or pass `--claude-hooks`." +
        (detectedHint ? `\nDetected agents in this repo: ${detectedHint}.` : ""),
    );
  }

  const sddHints = resolveSddHints(root, config);
  const results = groupBySymlink(root, targets).map((group) => applyAdapterGroup(root, group, sddHints));
  const hooksResult = claudeHooks ? installClaudeHooks(root) : null;

  if (values.json) {
    printJson({
      adapters: results,
      sdd: sddHints,
      ...(hooksResult ? { claudeHooks: hooksResult.written } : {}),
    });
    return;
  }

  for (const r of results) {
    const verb = r.action === "unchanged" ? "unchanged" : r.action;
    if (r.symlinkAliases && r.symlinkAliases.length > 0) {
      const aliasPaths = r.symlinkAliases.map((a) => a.path).join(", ");
      info(`  ${verb.padEnd(9)} ${aliasPaths} -> ${r.path} (symlink), managed block written once`);
    } else {
      info(`  ${verb.padEnd(9)} ${r.path}  (${ADAPTERS[r.adapter]!.name})`);
    }
  }
  if (sddHints.length > 0 && results.length > 0) {
    const summary = sddHints
      .map((h) => (h.matchedPath ? `${h.key} (${h.matchedPath})` : `${h.key} (no directory detected)`))
      .join(", ");
    info(`\nSDD hints included: ${summary}`);
  }
  if (hooksResult) {
    info("");
    info("Claude Code hooks + skill (SessionStart runs `agnosgram pack`, Stop reminds `agnosgram log`):");
    for (const w of hooksResult.written) {
      info(`  ${w.action.padEnd(9)} ${w.path}`);
    }
  }
}

export { AGENT_TARGETS };
