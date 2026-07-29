import { SDD_FRAMEWORKS } from "../core/detect.js";

/** An SDD framework to hint at, plus the specific directory to point the agent to. */
export interface SddHint {
  key: string;
  matchedPath: string;
}

/**
 * An adapter targets one agent's config file. Every adapter injects the *same*
 * ~10-line pointer body (principle 3: one source template; content lives only in
 * `.agnosgram/`). Shared files (CLAUDE.md, AGENTS.md) get the managed block
 * merged into user content; a dedicated file (Cursor `.mdc`) is fully ours.
 */
export interface Adapter {
  key: string;
  name: string;
  /** Target path relative to the project root. */
  targetPath: string;
  /** True when the whole file belongs to Agnosgram (dedicated rule file). */
  dedicatedFile: boolean;
  /** Content written above the managed block when creating a dedicated file. */
  preamble?: string;
}

export const ADAPTERS: Record<string, Adapter> = {
  claude: {
    key: "claude",
    name: "Claude Code",
    targetPath: "CLAUDE.md",
    dedicatedFile: false,
  },
  cursor: {
    key: "cursor",
    name: "Cursor",
    targetPath: ".cursor/rules/agnosgram.mdc",
    dedicatedFile: true,
    preamble: "---\ndescription: Project memory protocol (Agnosgram)\nalwaysApply: true\n---\n",
  },
  windsurf: {
    key: "windsurf",
    name: "Windsurf",
    targetPath: ".windsurf/rules/agnosgram.md",
    dedicatedFile: true,
    preamble: "---\ntrigger: always_on\n---\n",
  },
  cline: {
    key: "cline",
    name: "Cline",
    targetPath: ".clinerules/agnosgram.md",
    dedicatedFile: true,
  },
  roo: {
    key: "roo",
    name: "Roo Code",
    targetPath: ".roo/rules/agnosgram.md",
    dedicatedFile: true,
  },
  agents: {
    key: "agents",
    name: "AGENTS.md",
    targetPath: "AGENTS.md",
    dedicatedFile: false,
  },
};

export const ADAPTER_KEYS = Object.keys(ADAPTERS);

/** Hint lines describing coexistence with each detected SDD framework, pointing at the actual matched directory. */
function sddHintLines(hints: SddHint[]): string[] {
  const templates: Record<string, (path: string) => string> = {
    openspec: (p) =>
      `- OpenSpec is present (\`${p}\`): specs live there; memory records *why* and *what failed*, linking to specs by path.`,
    speckit: (p) =>
      `- Spec Kit is present (\`${p}\`): the constitution stays authoritative for principles; Agnosgram holds empirical lessons.`,
    bmad: (p) =>
      `- BMAD is present (\`${p}\`): QA/review steps should read \`.agnosgram/lessons/pitfalls.md\`; retro output goes to the journal.`,
    agentos: (p) =>
      `- Agent OS is present (\`${p}\`): \`standards/\` stays authoritative for style; Agnosgram holds project-local exceptions and history.`,
  };
  const known = new Set(SDD_FRAMEWORKS.map((f) => f.key));
  return hints
    .filter((h) => known.has(h.key) && templates[h.key])
    .map((h) => templates[h.key]!(h.matchedPath));
}

/** The shared pointer body injected into every adapter target. */
export function buildPointerBody(sddHints: SddHint[] = []): string {
  const lines = [
    "## Project memory (Agnosgram)",
    "",
    "This project keeps durable, agent-agnostic memory in `.agnosgram/` (plain",
    "Markdown, reviewed in PRs). **Before doing any work:**",
    "",
    "1. Read `.agnosgram/MEMORY.md` and follow its reading protocol.",
    "2. Always load `.agnosgram/state/status.md` and `.agnosgram/lessons/*`.",
    "3. Read `.agnosgram/context/*` and `.agnosgram/decisions/` only for the",
    "   areas you are about to touch.",
    "4. Before ending the session, record what happened with `agnosgram log`.",
  ];
  const hints = sddHintLines(sddHints);
  if (hints.length > 0) {
    lines.push("", "Coexisting tools detected:", ...hints);
  }
  return lines.join("\n");
}
