import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { ADAPTER_KEYS, ADAPTERS } from "../adapters/index.js";
import { parseCliArgs } from "../core/args.js";
import { defaultConfig, saveConfig, type AgnosgramConfig } from "../core/config.js";
import { detectAgents, detectSdd } from "../core/detect.js";
import { info, printJson, UserError } from "../core/output.js";
import { hasStore, memoryDir } from "../core/paths.js";
import {
  architectureMd,
  conventionsMd,
  decisionsReadme,
  domainMd,
  isoDate,
  journalMd,
  journalMonth,
  memoryMd,
  pitfallsMd,
  statusMd,
  stackMd,
} from "../core/templates.js";
import { applyAdapter, resolveSddHints, type AdaptResult } from "./adapt.js";

function scaffold(root: string): void {
  const dir = memoryDir(root);
  const date = isoDate();
  const month = journalMonth();

  for (const sub of ["state", "context", "decisions", "lessons", "journal"]) {
    mkdirSync(join(dir, sub), { recursive: true });
  }

  const files: Record<string, string> = {
    "MEMORY.md": memoryMd(date),
    "state/status.md": statusMd(date),
    "context/architecture.md": architectureMd(),
    "context/stack.md": stackMd(),
    "context/domain.md": domainMd(),
    "decisions/README.md": decisionsReadme(),
    "lessons/pitfalls.md": pitfallsMd(),
    "lessons/conventions.md": conventionsMd(),
    [`journal/${month}.md`]: journalMd(month),
  };
  for (const [rel, content] of Object.entries(files)) {
    writeFileSync(join(dir, rel), content);
  }
}

/** Parse `--adapt` into an explicit target list, `[]` for "none", or null for default. */
function parseAdaptOption(raw: string | undefined): string[] | null {
  if (raw === undefined) return null;
  const value = raw.trim().toLowerCase();
  if (value === "none") return [];
  const targets = value
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  const unknown = targets.filter((t) => !ADAPTERS[t]);
  if (unknown.length > 0) {
    throw new UserError(
      `Unknown adapter(s) in --adapt: ${unknown.join(", ")}. Known: ${ADAPTER_KEYS.join(", ")}.`,
    );
  }
  return targets;
}

export function runInit(argv: string[]): void {
  const { values } = parseCliArgs({
    args: argv,
    allowPositionals: false,
    options: {
      force: { type: "boolean", default: false },
      adapt: { type: "string" },
      json: { type: "boolean", default: false },
      "no-journal-commit": { type: "boolean", default: false },
    },
  });

  const root = process.cwd();
  if (hasStore(root) && !values.force) {
    throw new UserError(
      `.agnosgram/ already exists at ${root}. Use --force to re-scaffold missing files, ` +
        "or edit the store directly.",
    );
  }

  scaffold(root);

  const detectedSdd = detectSdd(root);
  const detectedAgents = detectAgents(root);

  const config: AgnosgramConfig = defaultConfig();
  if (values["no-journal-commit"]) config.journal.committed = false;

  // Decide which adapters to write. Default: every detected agent. `--adapt` overrides.
  const explicit = parseAdaptOption(values.adapt);
  const adaptTargets = explicit ?? detectedAgents.map((a) => a.key).filter((k) => ADAPTERS[k]);
  for (const key of adaptTargets) config.adapters[key] = "on";

  saveConfig(root, config);

  const sddHints = resolveSddHints(root, config);
  const adaptResults: AdaptResult[] = adaptTargets.map((key) =>
    applyAdapter(root, ADAPTERS[key]!, sddHints),
  );

  if (values.json) {
    printJson({
      root,
      created: `${root}/.agnosgram`,
      detectedSdd: detectedSdd.map((f) => ({ key: f.key, matchedPath: f.matchedPath })),
      detectedAgents: detectedAgents.map((a) => a.key),
      adapters: adaptResults,
    });
    return;
  }

  info(`Initialized Agnosgram memory in ${root}/.agnosgram`);
  info("");
  info("  Scaffolded: MEMORY.md, config.yml, state/, context/, decisions/, lessons/, journal/");
  if (detectedSdd.length > 0) {
    info(`  Detected SDD: ${detectedSdd.map((f) => `${f.name} (${f.matchedPath})`).join(", ")}`);
    info("  Agnosgram will not touch these files - adapter hints will point agents at them.");
  }
  if (detectedAgents.length > 0) {
    info(`  Detected agents: ${detectedAgents.map((a) => a.name).join(", ")}`);
  }
  if (adaptResults.length > 0) {
    info("");
    info("  Adapters written:");
    for (const r of adaptResults) {
      info(`    ${r.action.padEnd(9)} ${r.path}  (${ADAPTERS[r.adapter]!.name})`);
    }
  } else {
    info("");
    info(`  No adapters written. Add one with \`agnosgram adapt ${ADAPTER_KEYS.join("|")}\`.`);
  }
  info("");
  info("Next: fill in state/status.md and context/*, then commit .agnosgram/ to the repo.");
}
