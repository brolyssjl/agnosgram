import { existsSync } from "node:fs";
import { join } from "node:path";

/**
 * Detection is advisory only (principle 6): results become hint lines in pointer
 * blocks and defaults for which adapters to offer. Nothing here is load-bearing —
 * a missing or extra detection can only change hints, never corrupt memory.
 */

export interface SddFramework {
  key: string;
  name: string;
  /** Path fragments (relative to root); presence of any one counts as detected. */
  markers: string[];
}

export const SDD_FRAMEWORKS: SddFramework[] = [
  { key: "openspec", name: "OpenSpec", markers: ["openspec"] },
  { key: "speckit", name: "Spec Kit", markers: [".specify"] },
  { key: "bmad", name: "BMAD", markers: ["_bmad", ".bmad-core", "_bmad-core"] },
  { key: "agentos", name: "Agent OS", markers: ["agent-os", ".agent-os"] },
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
  { key: "agents", name: "AGENTS.md (Codex / OpenCode / generic)", signals: ["AGENTS.md"] },
];

export function detectSdd(root: string): SddFramework[] {
  return SDD_FRAMEWORKS.filter((f) => f.markers.some((m) => existsSync(join(root, m))));
}

export function detectAgents(root: string): AgentTarget[] {
  return AGENT_TARGETS.filter((a) => a.signals.some((s) => existsSync(join(root, s))));
}
