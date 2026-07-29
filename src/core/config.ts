import { readFileSync, writeFileSync } from "node:fs";
import { configPath } from "./paths.js";
import { parseYaml, stringifyYaml, type YamlValue } from "./yaml.js";

export type Toggle = "auto" | "on" | "off";

export interface AgnosgramConfig {
  version: number;
  journal: { committed: boolean };
  /** A record whose `last_verified` is older than this many days is stale. */
  staleness_days: number;
  budgets: Record<string, number>;
  adapters: Record<string, Toggle>;
  sdd: Record<string, Toggle>;
  /** Default token budget `pack` uses when `--budget` is not given (additive, optional). */
  pack_budget?: number;
}

export const DEFAULT_STALENESS_DAYS = 120;

export const DEFAULT_BUDGETS: Record<string, number> = {
  "state/status.md": 400,
  "context/architecture.md": 1500,
  "context/stack.md": 800,
  "context/domain.md": 1000,
  "lessons/pitfalls.md": 1000,
  "lessons/conventions.md": 1000,
};

export function defaultConfig(): AgnosgramConfig {
  return {
    version: 1,
    journal: { committed: true },
    staleness_days: DEFAULT_STALENESS_DAYS,
    budgets: { ...DEFAULT_BUDGETS },
    adapters: { claude: "off", cursor: "off", windsurf: "off", cline: "off", roo: "off", agents: "off" },
    sdd: { openspec: "auto", speckit: "auto", bmad: "auto", agentos: "auto" },
  };
}

export function loadConfig(root: string): AgnosgramConfig {
  const raw = parseYaml(readFileSync(configPath(root), "utf8"));
  return normalizeConfig(raw);
}

export function saveConfig(root: string, config: AgnosgramConfig): void {
  writeFileSync(configPath(root), serializeConfig(config));
}

export function serializeConfig(config: AgnosgramConfig): string {
  const header =
    "# Agnosgram configuration. Plain YAML, reviewable in PRs.\n" +
    "# adapters/sdd toggles: auto = follow detection, on = force, off = never.\n\n";
  return header + stringifyYaml(config as unknown as YamlValue);
}

function normalizeConfig(raw: YamlValue): AgnosgramConfig {
  const base = defaultConfig();
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) return base;
  const obj = raw as Record<string, YamlValue>;

  if (typeof obj.version === "number") base.version = obj.version;
  if (typeof obj.staleness_days === "number" && obj.staleness_days > 0) {
    base.staleness_days = obj.staleness_days;
  }

  const journal = obj.journal;
  if (journal && typeof journal === "object" && !Array.isArray(journal)) {
    const committed = (journal as Record<string, YamlValue>).committed;
    if (typeof committed === "boolean") base.journal.committed = committed;
  }

  base.budgets = numberMap(obj.budgets, base.budgets);
  base.adapters = toggleMap(obj.adapters, base.adapters);
  base.sdd = toggleMap(obj.sdd, base.sdd);
  if (typeof obj.pack_budget === "number" && obj.pack_budget > 0) {
    base.pack_budget = obj.pack_budget;
  }
  return base;
}

function numberMap(
  raw: YamlValue | undefined,
  fallback: Record<string, number>,
): Record<string, number> {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return fallback;
  const out: Record<string, number> = {};
  for (const [k, v] of Object.entries(raw)) if (typeof v === "number") out[k] = v;
  return Object.keys(out).length > 0 ? out : fallback;
}

function toggleMap(
  raw: YamlValue | undefined,
  fallback: Record<string, Toggle>,
): Record<string, Toggle> {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return fallback;
  const out: Record<string, Toggle> = { ...fallback };
  for (const [k, v] of Object.entries(raw)) {
    if (v === "auto" || v === "on" || v === "off") out[k] = v;
  }
  return out;
}
