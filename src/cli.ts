#!/usr/bin/env node
import { runAdapt } from "./commands/adapt.js";
import { runAdvise } from "./commands/advise.js";
import { runBootstrap } from "./commands/bootstrap.js";
import { runDistill } from "./commands/distill.js";
import { runDoctor } from "./commands/doctor.js";
import { runInit } from "./commands/init.js";
import { runLog } from "./commands/log.js";
import { runPack } from "./commands/pack.js";
import { runShow } from "./commands/show.js";
import { UserError, warn } from "./core/output.js";

const VERSION = "0.9.0";

const HELP = `agnosgram - agent-agnostic, per-project memory (plain Markdown in your repo)

Usage:
  agnosgram init [--adapt <list|none>] [--force] [--no-journal-commit] [--json]
  agnosgram adapt [claude|cursor|windsurf|cline|roo|agents ...] [--all] [--refresh]
                  [--claude-hooks] [--json]
  agnosgram log   [--did .. --learned .. --decided .. --avoid .. --next ..]
                  [--agent <name>] [--branch <name>] [--stdin] [--format json|toon]
  agnosgram doctor [--strict] [--format json|toon]
  agnosgram distill [--validate <file>] [--archive <YYYY-MM>] [--json]
  agnosgram bootstrap [--json]
  agnosgram show <topic> [--type pitfall|convention|decision] [--format json|toon]
  agnosgram pack [--scope <tag>] [--budget <n>] [--format json|toon]
  agnosgram advise <plan-path> [--out <file>] [--format json|toon]
  agnosgram advise --validate <report-file> [--strict] [--format json|toon]

Commands:
  init      Scaffold .agnosgram/, detect SDD frameworks + agents, write adapters.
  adapt     Insert/refresh the managed pointer block in an agent's config file.
            --claude-hooks installs opt-in SessionStart/Stop hooks + a skill.
  log       Append a journal entry (agents call this at session end).
  doctor    Lint the store: schema, staleness, budgets, links, ids, safety.
  distill   Emit a compaction prompt; validate a distilled result; archive months.
  bootstrap Emit a prompt seeding context/ from an existing codebase.
  show      Print records matching a topic (id, scope tag, or type).
  pack      Token-budgeted context bundle: status + lessons (+ decisions if scoped).
  advise    Emit a plan-vs-memory contradiction review prompt; --validate a report.

Global:
  -h, --help       Show this help.
  -v, --version    Print version.
  --format <fmt>   json (default with --json) or toon, where a command supports it.

Docs: https://github.com/agnosgram/agnosgram
`;

function main(): void {
  const argv = process.argv.slice(2);
  const [command, ...rest] = argv;

  if (command === undefined || command === "-h" || command === "--help" || command === "help") {
    process.stdout.write(HELP);
    return;
  }
  if (command === "-v" || command === "--version" || command === "version") {
    process.stdout.write(VERSION + "\n");
    return;
  }

  try {
    switch (command) {
      case "init":
        runInit(rest);
        break;
      case "adapt":
        runAdapt(rest);
        break;
      case "log":
        runLog(rest);
        break;
      case "doctor":
        runDoctor(rest);
        break;
      case "distill":
        runDistill(rest);
        break;
      case "bootstrap":
        runBootstrap(rest);
        break;
      case "show":
        runShow(rest);
        break;
      case "pack":
        runPack(rest);
        break;
      case "advise":
        runAdvise(rest);
        break;
      default:
        warn(`Unknown command: ${command}\n`);
        process.stdout.write(HELP);
        process.exitCode = 2;
    }
  } catch (err) {
    if (err instanceof UserError) {
      warn(`error: ${err.message}`);
      process.exitCode = 1;
    } else {
      throw err;
    }
  }
}

main();
