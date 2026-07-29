import { existsSync, statSync } from "node:fs";
import { join } from "node:path";

/**
 * Detection is advisory only (principle 6): results become hint lines in pointer
 * blocks and defaults for which adapters to offer. Nothing here is load-bearing -
 * a missing or extra detection can only change hints, never corrupt memory.
 */

export interface SddFramework {
  key: string;
  name: string;
  /** Path fragments (relative to root); presence of any one counts as detected. */
  markers: string[];
  /**
   * The pointer-block coexistence line for this framework, given the matched
   * directory (or `null` when forced `on` in config without ever being
   * detected - phrased so it never claims a path that isn't really there).
   * Required so a new framework can't be added to this registry without also
   * defining its hint text - no separate table to keep in sync, and no
   * silent no-op if one is missed.
   */
  hint: (matchedPath: string | null) => string;
}

/** Build a `hint` function: "X is present (`path`): tail" or, with no real path, "X support is enabled ...: tail". */
function sddHint(name: string, tail: string): (matchedPath: string | null) => string {
  return (matchedPath) =>
    matchedPath
      ? `- ${name} is present (\`${matchedPath}\`): ${tail}`
      : `- ${name} support is enabled (no directory detected on disk): ${tail}`;
}

export const SDD_FRAMEWORKS: SddFramework[] = [
  {
    key: "openspec",
    name: "OpenSpec",
    markers: ["openspec"],
    hint: sddHint(
      "OpenSpec",
      "specs live there; memory records *why* and *what failed*, linking to specs by path.",
    ),
  },
  {
    key: "speckit",
    name: "Spec Kit",
    markers: [".specify"],
    hint: sddHint(
      "Spec Kit",
      "the constitution stays authoritative for principles; Agnosgram holds empirical lessons.",
    ),
  },
  {
    key: "bmad",
    name: "BMAD",
    markers: ["_bmad", ".bmad-core", "_bmad-core"],
    hint: sddHint(
      "BMAD",
      "QA/review steps should read `.agnosgram/lessons/pitfalls.md`; retro output goes to the journal.",
    ),
  },
  {
    key: "agentos",
    name: "Agent OS",
    markers: ["agent-os", ".agent-os"],
    hint: sddHint(
      "Agent OS",
      "`standards/` stays authoritative for style; Agnosgram holds project-local exceptions and history.",
    ),
  },
];

export interface AgentTarget {
  key: string;
  name: string;
  /** Existing paths that indicate this agent is already in use. */
  signals: string[];
}

export const AGENT_TARGETS: AgentTarget[] = [
  { key: "claude", name: "Claude Code", signals: ["CLAUDE.md", ".claude"] },
  { key: "cursor", name: "Cursor", signals: [".cursor"] },
  { key: "windsurf", name: "Windsurf", signals: [".windsurf", ".windsurfrules"] },
  { key: "cline", name: "Cline", signals: [".clinerules"] },
  { key: "roo", name: "Roo Code", signals: [".roo", ".roorules"] },
  {
    key: "agents",
    name: "AGENTS.md (Codex / OpenCode / generic)",
    signals: ["AGENTS.md", ".codex", ".opencode", "opencode.json"],
  },
];

/** An SDD framework found on disk, plus which marker matched (for pointing hints at it). */
export interface SddDetection extends SddFramework {
  /**
   * The specific marker path that was found, e.g. `openspec/`. Only ever carries
   * a trailing slash when it's a real, `statSync`-confirmed directory - a marker
   * that happens to exist as a plain file is reported without one, so callers
   * never assert a directory that isn't actually there.
   */
  matchedPath: string;
}

export function detectSdd(root: string): SddDetection[] {
  return SDD_FRAMEWORKS.flatMap((f) => {
    const hit = f.markers.find((m) => existsSync(join(root, m)));
    if (!hit) return [];
    const isDirectory = statSync(join(root, hit)).isDirectory();
    return [{ ...f, matchedPath: isDirectory ? `${hit}/` : hit }];
  });
}

export function detectAgents(root: string): AgentTarget[] {
  return AGENT_TARGETS.filter((a) => a.signals.some((s) => existsSync(join(root, s))));
}
