import { appendFileSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { loadConfig } from "../core/config.js";
import { info, printStructured, UserError } from "../core/output.js";
import { findProjectRoot, hasStore, memoryDir } from "../core/paths.js";
import { resolveFormat } from "../core/serialize.js";
import { journalMd, journalMonth } from "../core/templates.js";

const SLOTS: Array<[keyof SlotValues, string]> = [
  ["did", "Did"],
  ["learned", "Learned"],
  ["decided", "Decided"],
  ["avoid", "Avoid"],
  ["next", "Next"],
];

interface SlotValues {
  did?: string;
  learned?: string;
  decided?: string;
  avoid?: string;
  next?: string;
}

function localTimestamp(d = new Date()): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** Best-effort current branch from `.git/HEAD`; null if not a repo or detached. */
export function currentBranch(root: string): string | null {
  const head = join(root, ".git", "HEAD");
  if (!existsSync(head)) return null;
  const m = /ref:\s*refs\/heads\/(.+)/.exec(readFileSync(head, "utf8").trim());
  return m ? m[1]!.trim() : null;
}

export function formatEntry(
  slots: SlotValues,
  meta: { agent: string; branch: string | null; when?: Date },
): string {
  const heading = ["##", localTimestamp(meta.when), "·", meta.agent]
    .concat(meta.branch ? ["·", meta.branch] : [])
    .join(" ");
  const lines = [heading];
  for (const [key, label] of SLOTS) {
    const value = slots[key];
    if (value && value.trim() !== "") lines.push(`- **${label}:** ${value.trim()}`);
  }
  return lines.join("\n") + "\n";
}

function readStdin(): string {
  try {
    return readFileSync(0, "utf8");
  } catch {
    return "";
  }
}

export function runLog(argv: string[]): void {
  const { values } = parseArgs({
    args: argv,
    allowPositionals: false,
    options: {
      did: { type: "string" },
      learned: { type: "string" },
      decided: { type: "string" },
      avoid: { type: "string" },
      next: { type: "string" },
      agent: { type: "string" },
      branch: { type: "string" },
      stdin: { type: "boolean", default: false },
      json: { type: "boolean", default: false },
      format: { type: "string" },
    },
  });

  const format = resolveFormat(values);

  const root = findProjectRoot();
  if (!hasStore(root)) {
    throw new UserError("No .agnosgram/ store found. Run `agnosgram init` first.");
  }
  loadConfig(root); // validates the store is well-formed before we append

  const agent = values.agent?.trim() || process.env.AGNOSGRAM_AGENT?.trim() || "agent";
  const branch = values.branch?.trim() || currentBranch(root);

  let entry: string;
  if (values.stdin) {
    const raw = readStdin().trim();
    if (raw === "") throw new UserError("--stdin given but nothing was piped in.");
    // A raw stdin entry is trusted as-authored; only ensure it starts as a heading.
    entry = raw.startsWith("##") ? raw + "\n" : `## ${localTimestamp()} · ${agent}${branch ? ` · ${branch}` : ""}\n${raw}\n`;
  } else {
    const slots: SlotValues = {
      did: values.did,
      learned: values.learned,
      decided: values.decided,
      avoid: values.avoid,
      next: values.next,
    };
    if (!SLOTS.some(([k]) => slots[k]?.trim())) {
      throw new UserError(
        "Nothing to log. Provide at least one of --did/--learned/--decided/--avoid/--next, " +
          "or pipe a full entry with --stdin.",
      );
    }
    entry = formatEntry(slots, { agent, branch });
  }

  const month = journalMonth();
  const file = join(memoryDir(root), "journal", `${month}.md`);
  if (!existsSync(file)) writeFileSync(file, journalMd(month));
  appendFileSync(file, "\n" + entry);

  const relFile = join(".agnosgram", "journal", `${month}.md`);
  if (format !== "human") {
    printStructured({ file: relFile, agent, branch, entry: entry.trimEnd() }, format);
    return;
  }
  info(`Logged to ${relFile}`);
}
