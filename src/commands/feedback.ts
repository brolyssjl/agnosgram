/**
 * `agnosgram feedback "<text>"` - capture tool friction (using Agnosgram
 * itself, not the host project) into `.agnosgram/meta/friction.md`. This is
 * a strictly separate namespace from host-project memory: `pack`/`show`/
 * `advise` never surface it (see core/meta.ts, core/records.ts). `init`
 * never scaffolds `meta/`; this command creates it on first use.
 *
 * Feeds `agnosgram reflect`, which turns friction entries into improvement
 * proposals for a human to review - see docs/feedback.md and
 * `.agnosgram/decisions/0003-human-in-the-loop-reflexive-loop.md`.
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { loadConfig, saveConfig } from "../core/config.js";
import { KNOWN_CONFIDENCE } from "../core/frontmatter.js";
import { FRICTION_FILE, FRICTION_TYPE, loadFrictionRecords } from "../core/meta.js";
import { info, printStructured, UserError } from "../core/output.js";
import { findProjectRoot, hasStore, memoryDir } from "../core/paths.js";
import { resolveFormat } from "../core/serialize.js";
import { isoDate } from "../core/templates.js";

/** Per-file token budget applied to meta/friction.md once it exists (additive
 * config key - the same "unknown keys are ignored" contract as pack_budget). */
const FRICTION_BUDGET = 1500;

function readStdin(): string {
  try {
    return readFileSync(0, "utf8");
  } catch {
    return "";
  }
}

function frictionMd(): string {
  return `# Friction - tool-usage friction, not host-project memory

_Captured with \`agnosgram feedback\`. Never read by \`pack\`/\`show\`/\`advise\` -
this namespace is about Agnosgram itself, and feeds \`agnosgram reflect\`. See
docs/feedback.md._
`;
}

/** Create meta/friction.md on first use and register its token budget. Both
 * steps are additive-only: a store that never calls `feedback` never gets
 * either, keeping `init`'s default output unchanged (per the M5 brief). */
function ensureMetaStore(root: string): string {
  const dir = join(memoryDir(root), "meta");
  const file = join(dir, "friction.md");
  if (!existsSync(file)) {
    mkdirSync(dir, { recursive: true });
    writeFileSync(file, frictionMd());

    const config = loadConfig(root);
    if (config.budgets[FRICTION_FILE] === undefined) {
      config.budgets[FRICTION_FILE] = FRICTION_BUDGET;
      saveConfig(root, config);
    }
  }
  return file;
}

function nextFrictionId(root: string): string {
  let max = 0;
  for (const rec of loadFrictionRecords(root)) {
    const m = /^FRI-(\d+)$/.exec(rec.frontmatter.id);
    if (m) max = Math.max(max, Number.parseInt(m[1]!, 10));
  }
  return `FRI-${String(max + 1).padStart(3, "0")}`;
}

/** Single-quote a shell argument, escaping embedded single quotes. */
function shellQuote(s: string): string {
  return `'${s.replace(/'/g, "'\\''")}'`;
}

/** A ready-to-run (never executed by Agnosgram) `gh issue create` command,
 * printed only behind the explicit, default-off `--share` flag. */
function shareCommand(id: string, text: string, scope: string[]): string {
  const title = text.length > 72 ? `${text.slice(0, 69)}...` : text;
  const body = [`Captured via \`agnosgram feedback\` (${id}).`, "", text, `\nScope: ${scope.join(", ")}`].join("\n");
  return `gh issue create --title ${shellQuote(title)} --body ${shellQuote(body)} --label feedback`;
}

export function runFeedback(argv: string[]): void {
  const { values, positionals } = parseArgs({
    args: argv,
    allowPositionals: true,
    options: {
      scope: { type: "string" },
      confidence: { type: "string" },
      stdin: { type: "boolean", default: false },
      share: { type: "boolean", default: false },
      json: { type: "boolean", default: false },
      format: { type: "string" },
    },
  });

  const format = resolveFormat(values);
  const root = findProjectRoot();
  if (!hasStore(root)) {
    throw new UserError("No .agnosgram/ store found. Run `agnosgram init` first.");
  }
  loadConfig(root); // validates the store is well-formed before we write

  let text = positionals.join(" ").trim();
  if (values.stdin) {
    const raw = readStdin().trim();
    if (raw === "") throw new UserError("--stdin given but nothing was piped in.");
    text = raw;
  }
  if (text === "") {
    throw new UserError('Usage: agnosgram feedback "<text>" [--scope <tag,...>] [--confidence low|medium|high] [--share]');
  }

  const confidence = values.confidence?.trim() || "medium";
  if (!(KNOWN_CONFIDENCE as readonly string[]).includes(confidence)) {
    throw new UserError(`--confidence must be one of ${KNOWN_CONFIDENCE.join(", ")}, got "${confidence}"`);
  }
  const scope = (values.scope ?? "cli")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  if (scope.length === 0) {
    throw new UserError("--scope must not be empty when given.");
  }

  const file = ensureMetaStore(root);
  const id = nextFrictionId(root);
  const date = isoDate();
  const entry =
    `\n---\nid: ${id}\ntype: ${FRICTION_TYPE}\nscope: [${scope.join(", ")}]\n` +
    `confidence: ${confidence}\ncreated: ${date}\nlast_verified: ${date}\nsource: ${FRICTION_FILE}\n---\n${text}\n`;

  const prior = readFileSync(file, "utf8");
  writeFileSync(file, prior.replace(/\n*$/, "\n") + entry);

  const relFile = join(".agnosgram", FRICTION_FILE);
  const share = values.share ? shareCommand(id, text, scope) : undefined;

  if (format !== "human") {
    printStructured({ id, file: relFile, scope, confidence, text, share: share ?? null }, format);
    return;
  }
  info(`Logged ${id} to ${relFile}`);
  if (share) {
    info("");
    info("Share this as a GitHub issue - run it yourself, Agnosgram never executes it:");
    info(`  ${share}`);
  }
}
