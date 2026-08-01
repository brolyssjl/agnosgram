/**
 * `agnosgram reflect` - turns tool friction (`meta/friction.md`) and recent
 * journal months into improvement proposals and candidate roadmap
 * milestones. Same prompt-emitting pattern as `distill`/`advise`/`bootstrap`
 * (see those files): the CLI never calls an LLM, only assembles a prompt for
 * whatever agent is present.
 *
 * Unlike `distill`/`advise`, `reflect` has no `--validate` step - its output
 * is proposals for a human to read, not a mechanically-checkable report.
 * `reflect` itself performs no writes to the repo, and its prompt states
 * explicitly that ROADMAP.md is owner-edited: proposals go to stdout or a
 * separate artifact, never applied automatically. See
 * `.agnosgram/decisions/0003-human-in-the-loop-reflexive-loop.md`.
 */
import { parseArgs } from "node:util";
import { loadFrictionRecords } from "../core/meta.js";
import { printStructured, UserError } from "../core/output.js";
import { findProjectRoot, hasStore } from "../core/paths.js";
import type { StoreRecord } from "../core/records.js";
import { resolveFormat } from "../core/serialize.js";
import { journalMonths } from "../core/store.js";

/** Envelope version for `reflect --json` (independent of the store format version). */
export const REFLECT_SCHEMA_VERSION = 1;

/** Recent journal months included by default when `--months` is not given. */
const DEFAULT_MONTHS = 3;

function frictionDigest(records: StoreRecord[]): string {
  if (records.length === 0) return "_(no friction entries yet - run `agnosgram feedback \"...\"` to capture some)_";
  const rows = records
    .slice()
    .sort((a, b) => a.frontmatter.id.localeCompare(b.frontmatter.id))
    .map((r) => {
      const normalized = r.body.replace(/\s+/g, " ").trim();
      const truncated = normalized.length > 100;
      const snippet = normalized.slice(0, 100).replace(/\|/g, "\\|");
      return `| ${r.frontmatter.id} | ${r.frontmatter.scope.join(",")} | ${r.frontmatter.confidence} | ${r.frontmatter.created} | ${snippet}${truncated ? "..." : ""} |`;
    });
  const header = "| id | scope | confidence | created | excerpt |\n|---|---|---|---|---|";
  return `${header}\n${rows.join("\n")}`;
}

function buildPrompt(months: string[], friction: StoreRecord[]): string {
  return `# Agnosgram reflect task

You are reviewing how well Agnosgram itself is serving this project - not the
host project's own code or memory. Turn tool friction and recent session
history into a small set of concrete improvement proposals and, where
warranted, candidate roadmap milestones. Do NOT invent friction that was never
reported; only propose what the sources below actually support.

## Sources to read
- Friction entries: \`.agnosgram/meta/friction.md\` (this project's tool
  friction, never host-project memory).
- Recent journal months: ${months.length ? months.map((m) => `journal/${m}.md`).join(", ") : "(none yet)"}
  Look for recurring \`Avoid\`/\`Learned\` lines that point at friction with the
  tool itself, not the host project.
- Current roadmap: \`ROADMAP.md\` (read-only context - see the rule below).

## Digest: every friction entry currently captured
${frictionDigest(friction)}

## Output rules
1. Group related friction into themes; do not propose one item per friction
   entry when several describe the same underlying gap.
2. For each proposal, state: the friction it addresses (cite \`FRI-\` ids),
   the concrete change, and its expected effect. Keep each to a few sentences.
3. Where a proposal is substantial enough to be a milestone rather than a
   small fix, suggest it as a **candidate roadmap milestone** - a title and a
   one-paragraph scope, not a full spec.
4. Present every proposal in this reviewable form:
   \`\`\`
   ### Proposal: <short title>
   - Addresses: FRI-001, FRI-004
   - Change: <what to do>
   - Effect: <why it helps>
   - Candidate milestone: <yes/no - if yes, a one-line scope>
   \`\`\`

## Rule: ROADMAP.md is owner-edited (non-negotiable)
Print your proposals to stdout for a human to read, or if asked to keep a
durable record, write them to a **new** file (e.g. a dated notes file the
human names) - never to \`ROADMAP.md\` directly, and never to \`lessons/\`,
\`decisions/\`, \`context/\`, or \`state/status.md\`. This command proposes; it
never disposes. A human decides which proposals become real roadmap items and
edits \`ROADMAP.md\` themselves.

## When done
Nothing to validate mechanically - this is a proposal, not a schema-checked
report. If a proposal implies a store or CLI change, remember any schema or
layout change must land together with a \`doctor\` change (CON-003).
`;
}

export function runReflect(argv: string[]): void {
  const { values } = parseArgs({
    args: argv,
    allowPositionals: false,
    options: {
      months: { type: "string" },
      json: { type: "boolean", default: false },
      format: { type: "string" },
    },
  });

  const format = resolveFormat(values);
  const root = findProjectRoot();
  if (!hasStore(root)) {
    throw new UserError("No .agnosgram/ store found. Run `agnosgram init` first.");
  }

  let monthCount = DEFAULT_MONTHS;
  if (values.months !== undefined) {
    // Strict: reject anything parseInt would otherwise accept by truncating
    // or stopping early (e.g. "2.5" -> 2, "3abc" -> 3).
    if (!/^[1-9]\d*$/.test(values.months)) {
      throw new UserError(`--months must be a positive integer, got "${values.months}"`);
    }
    monthCount = Number.parseInt(values.months, 10);
  }

  const allMonths = journalMonths(root);
  const months = allMonths.slice(Math.max(0, allMonths.length - monthCount));
  const friction = loadFrictionRecords(root);

  const prompt = buildPrompt(months, friction);

  if (format !== "human") {
    printStructured(
      {
        agnosgram_reflect: REFLECT_SCHEMA_VERSION,
        friction_count: friction.length,
        friction_ids: friction.map((r) => r.frontmatter.id),
        journal_months: months,
        prompt,
      },
      format,
    );
  } else {
    process.stdout.write(prompt);
  }
}
