import { SDD_FRAMEWORKS } from "../core/detect.js";

/**
 * An SDD framework to hint at, plus the specific directory to point the agent to.
 * `matchedPath` is `null` when the framework was forced `on` in `config.yml`
 * without ever being detected on disk - there is no real path to point at.
 */
export interface SddHint {
  key: string;
  matchedPath: string | null;
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
  /**
   * A legacy single-file convention this tool also reads (e.g. Cline's older
   * `.clinerules` file, before the `.clinerules/` directory convention). If this
   * path already exists as a plain file when applying, write there instead of
   * `targetPath`, merging into whatever the user already has - never `mkdir`
   * over a file that's already there.
   */
  legacyTargetPath?: string;
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
    legacyTargetPath: ".clinerules",
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

/**
 * Hint lines describing coexistence with each detected SDD framework, pointing at
 * the actual matched directory. The hint text itself lives on each SDD_FRAMEWORKS
 * entry (single registry) - nothing here can silently drop a framework that's
 * missing a template, since `hint` is a required field on `SddFramework`.
 */
function sddHintLines(hints: SddHint[]): string[] {
  return hints.flatMap((h) => {
    const framework = SDD_FRAMEWORKS.find((f) => f.key === h.key);
    return framework ? [framework.hint(h.matchedPath)] : [];
  });
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
