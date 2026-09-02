//! Port of `src/cli.ts`: argv dispatch, help/version, exit codes.

mod adapters;
mod claude_hooks;
mod commands;
mod core;

use core::output::{warn, UserError};

/// `cli.ts` reads this from `package.json` at runtime; there is no
/// `package.json` next to an installed binary, so it is compiled in from
/// `rust/Cargo.toml`'s `version` - one fewer place to bump per release.
/// The release workflow's version guard keeps it in lockstep with the tag
/// and `package.json`.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Originally a byte-for-byte copy of `src/cli.ts`'s (retired) `HELP`
/// template literal, with `${ADAPTER_KEYS.join("|")}` already resolved to
/// its frozen value (`claude|cursor|windsurf|cline|roo|agents`, see
/// `src/adapters/index.ts`); has since gained small Rust-only clarifications
/// (e.g. the `doctor --strict` exit-code note, agnosgram#28) that have no TS
/// counterpart to stay byte-identical with.
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
            --strict fails (exit 1) on warnings alone, not just errors.
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

/// Per-subcommand usage block, shown for `agnosgram <command> -h|--help`
/// instead of the "Unknown option" a raw `parse_cli_args` call would raise
/// (none of the per-command `ArgsConfig`s declare `-h`/`--help` as an
/// option). Lifted verbatim from `HELP`'s `Usage:`/`Commands:` entries for
/// that command, the same approach gate's FRI-004 fix used: intercept help
/// once, centrally, before a subcommand's own arg parser ever sees the argv.
/// `None` for anything not in the dispatch table below, so an unknown
/// command falls through to the existing "Unknown command" + full `HELP`.
fn command_usage(command: &str) -> Option<&'static str> {
    match command {
        "init" => Some(
            "Usage:\n  agnosgram init [--adapt <list|none>] [--force] [--no-journal-commit] [--json]\n\n\
             Scaffold .agnosgram/, detect SDD frameworks + agents, write adapters.\n",
        ),
        "adapt" => Some(
            "Usage:\n  agnosgram adapt [claude|cursor|windsurf|cline|roo|agents ...] [--all] [--refresh]\n                  [--claude-hooks] [--json]\n\n\
             Insert/refresh the managed pointer block in an agent's config file.\n\
             --claude-hooks installs opt-in SessionStart/Stop hooks + a skill.\n",
        ),
        "log" => Some(
            "Usage:\n  agnosgram log   [--did .. --learned .. --decided .. --avoid .. --next ..]\n                  [--agent <name>] [--branch <name>] [--stdin] [--format json|toon]\n\n\
             Append a journal entry (agents call this at session end).\n",
        ),
        "doctor" => Some(
            "Usage:\n  agnosgram doctor [--strict] [--format json|toon]\n\n\
             Lint the store: schema, staleness, budgets, links, ids, safety.\n",
        ),
        "distill" => Some(
            "Usage:\n  agnosgram distill [--validate <file>] [--archive <YYYY-MM>] [--json]\n\n\
             Emit a compaction prompt; validate a distilled result; archive months.\n",
        ),
        "bootstrap" => Some(
            "Usage:\n  agnosgram bootstrap [--json]\n\n\
             Emit a prompt seeding context/ from an existing codebase.\n",
        ),
        "show" => Some(
            "Usage:\n  agnosgram show <topic> [--type pitfall|convention|decision] [--format json|toon]\n\n\
             Print records matching a topic (id, scope tag, or type).\n",
        ),
        "pack" => Some(
            "Usage:\n  agnosgram pack [--scope <tag>] [--budget <n>] [--format json|toon]\n\n\
             Token-budgeted context bundle: status + lessons (+ decisions if scoped).\n",
        ),
        "advise" => Some(
            "Usage:\n  agnosgram advise <plan-path> [--out <file>] [--format json|toon]\n  agnosgram advise --validate <report-file> [--strict] [--format json|toon]\n\n\
             Emit a plan-vs-memory contradiction review prompt; --validate a report.\n",
        ),
        "feedback" => Some(
            "Usage:\n  agnosgram feedback \"<text>\" [--scope <tag,...>] [--confidence low|medium|high]\n                    [--stdin] [--share] [--format json|toon]\n\n\
             Capture tool friction into .agnosgram/meta/ (never host-project memory).\n\
             --share prints a ready-to-run \"gh issue create\" command; never runs it.\n",
        ),
        "reflect" => Some(
            "Usage:\n  agnosgram reflect [--months <n>] [--format json|toon]\n\n\
             Emit a prompt turning friction + recent journal months into proposals.\n",
        ),
        _ => None,
    }
}

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

    // Every subcommand's own `ArgsConfig` is silent on `-h`/`--help` (they
    // are not options any command declares), so without this check
    // `parse_cli_args` would reject them as "Unknown option '-h'" - the
    // exact bug this block exists to fix. Checked before dispatch, so a
    // subcommand's `run` never sees `-h`/`--help` in its argv at all.
    if rest.iter().any(|a| a == "-h" || a == "--help") {
        if let Some(usage) = command_usage(&command) {
            print!("{usage}\nSee `agnosgram --help` for the full command list.\n");
            return;
        }
    }

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
