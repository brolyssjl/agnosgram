import { existsSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
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
  const results = targets.map((key) => applyAdapter(root, ADAPTERS[key]!, sddHints));
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
    info(`  ${verb.padEnd(9)} ${r.path}  (${ADAPTERS[r.adapter]!.name})`);
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
