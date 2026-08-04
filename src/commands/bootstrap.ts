import { readdirSync } from "node:fs";
import { join } from "node:path";
import { parseCliArgs } from "../core/args.js";
import { loadConfig } from "../core/config.js";
import { detectAgents, detectSdd } from "../core/detect.js";
import { info, printJson, UserError } from "../core/output.js";
import { findProjectRoot, hasStore } from "../core/paths.js";

/** Files whose presence names a stack, so the prompt can point the agent at them. */
const STACK_SIGNALS: Array<{ file: string; hint: string }> = [
  { file: "package.json", hint: "Node/JavaScript or TypeScript (package.json)" },
  { file: "tsconfig.json", hint: "TypeScript (tsconfig.json)" },
  { file: "go.mod", hint: "Go (go.mod)" },
  { file: "Cargo.toml", hint: "Rust (Cargo.toml)" },
  { file: "pyproject.toml", hint: "Python (pyproject.toml)" },
  { file: "requirements.txt", hint: "Python (requirements.txt)" },
  { file: "pom.xml", hint: "Java/Maven (pom.xml)" },
  { file: "build.gradle", hint: "JVM/Gradle (build.gradle)" },
  { file: "Gemfile", hint: "Ruby (Gemfile)" },
  { file: "composer.json", hint: "PHP (composer.json)" },
];

const IGNORE_ENTRIES = new Set([
  ".git",
  ".agnosgram",
  "node_modules",
  "dist",
  "build",
  ".next",
  "target",
  "vendor",
  ".venv",
  "__pycache__",
]);

function topLevel(root: string): string[] {
  return readdirSync(root, { withFileTypes: true })
    .filter((e) => !IGNORE_ENTRIES.has(e.name) && !e.name.startsWith("."))
    .map((e) => (e.isDirectory() ? `${e.name}/` : e.name))
    .sort();
}

function buildPrompt(root: string): string {
  const entries = topLevel(root);
  const dirNames = new Set(readdirSync(root, { withFileTypes: true }).map((e) => e.name));
  const stack = STACK_SIGNALS.filter((s) => dirNames.has(s.file)).map((s) => s.hint);
  const sdd = detectSdd(root).map((f) => f.name);
  const agents = detectAgents(root).map((a) => a.name);
  const config = loadConfig(root);

  const budget = (f: string) => config.budgets[f] ?? "n/a";

  return `# Agnosgram bootstrap task (legacy onboarding)

This repository has an Agnosgram store but its context files are still empty
templates. Seed them from the code that already exists, so future sessions start
with real project knowledge instead of re-discovering it every time.

## What the tool already sees
- Top-level entries: ${entries.length ? entries.join(", ") : "(none)"}
- Stack signals: ${stack.length ? stack.join("; ") : "(none detected)"}
- SDD frameworks: ${sdd.length ? sdd.join(", ") : "(none)"}
- Agent config files: ${agents.length ? agents.join(", ") : "(none)"}

## Your task
Explore the codebase (start from the entries above, read entry points, build
files, and the largest modules) and fill in these files. Keep each within its
token budget - terse and high-signal, not exhaustive:

1. **context/architecture.md** (budget ${budget("context/architecture.md")} tokens)
   - Module map: top-level component -> responsibility (one line each).
   - Invariants: rules that must always hold (data flow, boundaries, "never do X").
   - Only what an agent must know before touching the code.

2. **context/domain.md** (budget ${budget("context/domain.md")} tokens)
   - Business/domain glossary: terms and rules an agent will NOT infer from code.
   - Skip anything obvious from the source.

3. **context/stack.md** (budget ${budget("context/stack.md")} tokens) if you can
   determine it: exact build/test/lint commands and pinned key versions.

## Rules
- Ground every statement in something you actually read; do not guess. Where you
  are unsure, say so briefly rather than inventing detail.
- Do not touch lessons/, decisions/, or the journal - those come from real
  sessions via \`agnosgram log\` and \`agnosgram distill\`, not from bootstrap.
- Preserve each file's existing headings; replace only the placeholder bullets.

## When done
Run \`agnosgram doctor\` and resolve anything it flags (budgets, links).
`;
}

export function runBootstrap(argv: string[]): void {
  const { values } = parseCliArgs({
    args: argv,
    allowPositionals: false,
    options: {
      json: { type: "boolean", default: false },
    },
  });

  const root = findProjectRoot();
  if (!hasStore(root)) {
    throw new UserError("No .agnosgram/ store found. Run `agnosgram init` first.");
  }

  const prompt = buildPrompt(root);
  if (values.json) printJson({ prompt });
  else process.stdout.write(prompt);
}
