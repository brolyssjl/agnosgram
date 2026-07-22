#!/usr/bin/env node
import { runAdapt } from "./commands/adapt.js";
import { runInit } from "./commands/init.js";
import { runLog } from "./commands/log.js";
import { UserError, warn } from "./core/output.js";

const VERSION = "0.1.0";

const HELP = `agnosgram — agent-agnostic, per-project memory (plain Markdown in your repo)

Usage:
  agnosgram init [--adapt <list|none>] [--force] [--no-journal-commit] [--json]
  agnosgram adapt [claude|cursor|agents ...] [--all] [--refresh] [--json]
  agnosgram log   [--did .. --learned .. --decided .. --avoid .. --next ..]
                  [--agent <name>] [--branch <name>] [--stdin] [--json]

Commands:
  init    Scaffold .agnosgram/, detect SDD frameworks + agents, write adapters.
  adapt   Insert/refresh the managed pointer block in an agent's config file.
  log     Append a journal entry (agents call this at session end).

Global:
  -h, --help       Show this help.
  -v, --version    Print version.

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
