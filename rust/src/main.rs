//! Port of `src/cli.ts`: argv dispatch, help/version, exit codes. Mirrors the
//! TS entry point exactly - see `docs/rust-port.md`'s exactness contract.
//! Wave 2 replaces each `commands::*::run` body without touching this file.

// Wave 1 scaffolds every `core/` module ahead of the commands that will call
// into them (see docs/rust-port.md's wave plan): every command is still a
// stub, so most `core/` exports are unused until wave 2 wires them up. Allow
// dead code for that transitional state rather than defeat clippy's
// `-D warnings` gate with artificial uses. Each `core/*.rs` module still
// carries its own `#[cfg(test)]` coverage, exercised by `cargo test`.
#![allow(dead_code)]

mod commands;
mod core;

use core::output::{warn, UserError};

/// `cli.ts` reads this from `package.json` at runtime; there is no
/// `package.json` next to an installed binary, so it is a constant here.
/// Bump this on every release, in lockstep with `package.json`'s `version`
/// and `rust/Cargo.toml`'s `version`.
const VERSION: &str = "0.11.0";

/// Byte-for-byte copy of `src/cli.ts`'s `HELP` template literal, with
/// `${ADAPTER_KEYS.join("|")}` already resolved to its frozen value
/// (`claude|cursor|windsurf|cline|roo|agents`, see `src/adapters/index.ts`).
const HELP: &str = r#"agnosgram - agent-agnostic, per-project memory (plain Markdown in your repo)

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
  agnosgram feedback "<text>" [--scope <tag,...>] [--confidence low|medium|high]
                    [--stdin] [--share] [--format json|toon]
  agnosgram reflect [--months <n>] [--format json|toon]

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
  feedback  Capture tool friction into .agnosgram/meta/ (never host-project memory).
            --share prints a ready-to-run "gh issue create" command; never runs it.
  reflect   Emit a prompt turning friction + recent journal months into proposals.

Global:
  -h, --help       Show this help.
  -v, --version    Print version.
  --format <fmt>   json (default with --json) or toon, where a command supports it.

Docs: https://github.com/agnosgram/agnosgram
"#;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let command = argv.first().cloned();
    let rest: Vec<String> = if argv.is_empty() {
        Vec::new()
    } else {
        argv[1..].to_vec()
    };

    match command.as_deref() {
        None | Some("-h") | Some("--help") | Some("help") => {
            print!("{HELP}");
            return;
        }
        Some("-v") | Some("--version") | Some("version") => {
            println!("{VERSION}");
            return;
        }
        _ => {}
    }

    let command = command.unwrap();
    let result = match command.as_str() {
        "init" => commands::init::run(rest),
        "adapt" => commands::adapt::run(rest),
        "log" => commands::log::run(rest),
        "doctor" => commands::doctor::run(rest),
        "distill" => commands::distill::run(rest),
        "bootstrap" => commands::bootstrap::run(rest),
        "show" => commands::show::run(rest),
        "pack" => commands::pack::run(rest),
        "advise" => commands::advise::run(rest),
        "feedback" => commands::feedback::run(rest),
        "reflect" => commands::reflect::run(rest),
        _ => {
            warn(&format!("Unknown command: {command}\n"));
            print!("{HELP}");
            std::process::exit(2);
        }
    };

    if let Err(UserError(message)) = result {
        warn(&format!("error: {message}"));
        std::process::exit(1);
    }
}
