/**
 * `agnosgram show <topic>` - print stored records matching a topic, for agents
 * with weak file navigation (or humans who just want the relevant slice without
 * opening `.agnosgram/` by hand). Read-only; never edits the store.
 *
 * Match order (first non-empty wins, no fuzzy matching - deferred):
 *   1. exact record id (`LES-001`)
 *   2. case-insensitive exact scope tag (`backend`, `Backend`, ...)
 *   3. type name (`pitfall` | `convention` | `decision`)
 */
import { parseArgs } from "node:util";
import { info, printStructured, UserError, warn } from "../core/output.js";
import { findProjectRoot, hasStore } from "../core/paths.js";
import {
  allScopes,
  loadRecords,
  renderRecordBlock,
  toFlatRecord,
  type StoreRecord,
} from "../core/records.js";
import { resolveFormat } from "../core/serialize.js";

const KNOWN_TYPES = ["pitfall", "convention", "decision"];

export function matchRecords(
  records: StoreRecord[],
  topic: string,
  typeFilter?: string,
): StoreRecord[] {
  const pool = typeFilter ? records.filter((r) => r.frontmatter.type === typeFilter) : records;
  const topicLower = topic.toLowerCase();

  const byId = pool.filter((r) => r.frontmatter.id === topic);
  if (byId.length > 0) return byId;

  const byScope = pool.filter((r) =>
    r.frontmatter.scope.some((s) => s.toLowerCase() === topicLower),
  );
  if (byScope.length > 0) return byScope;

  if ((KNOWN_TYPES as string[]).includes(topicLower)) {
    const byType = pool.filter((r) => r.frontmatter.type === topicLower);
    if (byType.length > 0) return byType;
  }

  return [];
}

/** Human rendering: records as they look on disk, grouped under a per-file heading. */
function renderHuman(records: StoreRecord[]): string {
  if (records.length === 0) return "";
  const lines: string[] = [];
  let currentFile = "";
  for (const r of records) {
    if (r.file !== currentFile) {
      currentFile = r.file;
      lines.push(`## ${currentFile}`, "");
    }
    lines.push(renderRecordBlock(r), "");
  }
  return lines.join("\n").trimEnd() + "\n";
}

export function runShow(argv: string[]): void {
  const { values, positionals } = parseArgs({
    args: argv,
    allowPositionals: true,
    options: {
      type: { type: "string" },
      json: { type: "boolean", default: false },
      format: { type: "string" },
    },
  });

  const format = resolveFormat(values);

  const topic = positionals[0];
  if (!topic || topic.trim() === "") {
    throw new UserError("Usage: agnosgram show <topic> [--type pitfall|convention|decision]");
  }

  if (values.type !== undefined && !(KNOWN_TYPES as string[]).includes(values.type)) {
    throw new UserError(`--type must be one of ${KNOWN_TYPES.join(", ")}, got "${values.type}"`);
  }

  const root = findProjectRoot();
  if (!hasStore(root)) {
    throw new UserError("No .agnosgram/ store found. Run `agnosgram init` first.");
  }

  const records = loadRecords(root);
  const matches = matchRecords(records, topic.trim(), values.type);

  if (matches.length === 0) {
    const scopes = allScopes(root);
    warn(`No records match "${topic}".`);
    warn(
      scopes.length > 0
        ? `Known scopes: ${scopes.join(", ")}. Types: ${KNOWN_TYPES.join(", ")}.`
        : `Types: ${KNOWN_TYPES.join(", ")}. (No scope tags in the store yet.)`,
    );
    process.exitCode = 1;
    return;
  }

  if (format === "human") {
    info(renderHuman(matches).trimEnd());
  } else {
    printStructured(matches.map(toFlatRecord), format);
  }
}
