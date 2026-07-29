import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { parseArgs } from "node:util";
import { ADAPTER_KEYS, ADAPTERS, buildPointerBody, type Adapter, type SddHint } from "../adapters/index.js";
import { installClaudeHooks } from "../core/claudeHooks.js";
import { loadConfig, saveConfig, type AgnosgramConfig, type Toggle } from "../core/config.js";
import { AGENT_TARGETS, SDD_FRAMEWORKS, detectAgents, detectSdd } from "../core/detect.js";
import { upsertManagedBlock } from "../core/markers.js";
import { info, printJson, UserError } from "../core/output.js";
import { findProjectRoot, hasStore } from "../core/paths.js";

export type AdaptAction = "created" | "updated" | "unchanged";

export interface AdaptResult {
  adapter: string;
  path: string;
  action: AdaptAction;
}

/**
 * Which SDD frameworks are active for hint lines, given config + detection, each
 * paired with the specific directory that was actually found (falling back to the
 * framework's primary marker when forced "on" without a detected directory).
 */
export function resolveSddHints(root: string, config: AgnosgramConfig): SddHint[] {
  const detected = new Map(detectSdd(root).map((f) => [f.key, f.matchedPath]));
  return Object.entries(config.sdd)
    .filter(([key, toggle]) => toggle === "on" || (toggle === "auto" && detected.has(key)))
    .map(([key]) => {
      const framework = SDD_FRAMEWORKS.find((f) => f.key === key);
      const matchedPath = detected.get(key) ?? (framework ? `${framework.markers[0]}/` : "");
      return { key, matchedPath };
    });
}

/** Inject or refresh one adapter's managed block. Idempotent. */
export function applyAdapter(root: string, adapter: Adapter, sddHints: SddHint[]): AdaptResult {
  const target = join(root, adapter.targetPath);
  const body = buildPointerBody(sddHints);

  let existing = "";
  if (existsSync(target)) {
    existing = readFileSync(target, "utf8");
  } else if (adapter.dedicatedFile && adapter.preamble) {
    existing = adapter.preamble;
  }

  const next = upsertManagedBlock(existing, body);
  const existedBefore = existsSync(target);

  if (existedBefore && next === existing) {
    return { adapter: adapter.key, path: adapter.targetPath, action: "unchanged" };
  }

  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, next);
  return {
    adapter: adapter.key,
    path: adapter.targetPath,
    action: existedBefore ? "updated" : "created",
  };
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
  const { values, positionals } = parseArgs({
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

  const claudeHooks = values["claude-hooks"];

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
    info(`\nSDD hints included: ${sddHints.map((h) => `${h.key} (${h.matchedPath})`).join(", ")}`);
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
