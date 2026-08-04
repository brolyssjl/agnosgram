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
import { parseCliArgs } from "../core/args.js";
import { loadConfig } from "../core/config.js";
import { KNOWN_CONFIDENCE } from "../core/frontmatter.js";
import { FRICTION_FILE, FRICTION_TYPE, loadFrictionRecords } from "../core/meta.js";
import { info, printStructured, UserError } from "../core/output.js";
import { findProjectRoot, hasStore, memoryDir } from "../core/paths.js";
import { resolveFormat } from "../core/serialize.js";
import { isoDate } from "../core/templates.js";

/** The real remote this repo lives at - not the `agnosgram/agnosgram` org
 * slug used in docs URLs, which is currently a dead link. `--share` must
 * name this exact repo, or the printed command would file the issue on
 * whatever host-project repo the CLI happens to be run from instead. */
const AGNOSGRAM_REPO = "brolyssjl/agnosgram";

/** Scope tags are written into YAML frontmatter as an inline flow sequence
 * (`[a, b]`); restricting them to a safe charset keeps that output canonical
 * instead of relying on parser leniency for anything containing `]`, `:`, etc. */
const SCOPE_TAG_PATTERN = /^[A-Za-z0-9._-]+$/;

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

/** Create meta/friction.md on first use. `feedback` writes only under
 * `.agnosgram/meta/` - never config.yml or anything else - so a project that
 * wants to cap it with a budget adds `budgets: {"meta/friction.md": N}` to
 * config.yml by hand; `doctor` already enforces any budget entry that names
 * an existing file, meta/ included (see core/meta.ts, doctor.ts). */
function ensureMetaStore(root: string): string {
  const dir = join(memoryDir(root), "meta");
  const file = join(dir, "friction.md");
  if (!existsSync(file)) {
    mkdirSync(dir, { recursive: true });
    writeFileSync(file, frictionMd());
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
 * printed only behind the explicit, default-off `--share` flag. Always
 * targets the real agnosgram repo via `--repo` - `agnosgram` runs in the
 * host project's cwd, so without it the issue would be filed on whatever
 * unrelated repo the command happens to run from. No `--label`: an
 * unpinned label would fail on a repo that has not defined it. */
function shareCommand(id: string, text: string, scope: string[]): string {
  const title = text.length > 72 ? `${text.slice(0, 69)}...` : text;
  const body = [`Captured via \`agnosgram feedback\` (${id}).`, "", text, `\nScope: ${scope.join(", ")}`].join("\n");
  return `gh issue create --repo ${AGNOSGRAM_REPO} --title ${shellQuote(title)} --body ${shellQuote(body)}`;
}

export function runFeedback(argv: string[]): void {
  const { values, positionals } = parseCliArgs({
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
  for (const tag of scope) {
    if (!SCOPE_TAG_PATTERN.test(tag)) {
      throw new UserError(
        `--scope tag "${tag}" must contain only letters, digits, dot, dash, or underscore.`,
      );
    }
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
