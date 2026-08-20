//! Port of `src/core/claudeHooks.ts`: opt-in Claude Code integration
//! (`agnosgram adapt --claude-hooks`) - a SessionStart hook that runs
//! `agnosgram pack` into context, a Stop hook that reminds (never forces) the
//! agent to run `agnosgram log`, and a skill file describing the CLI.
//!
//! Also carries the write-if-changed helper (`src/core/writeFile.ts` in the
//! TS tree) since both this module and `commands::adapt` need it and the
//! wave-1 core module list (see `docs/rust-port.md`) does not include a
//! standalone file for it.

use std::fs;
use std::path::Path;

use crate::core::json::Value;
use crate::core::output::UserError;
use crate::core::write_file::{write_if_changed, WriteOpts, WriteResult};

const HOOKS_REL_DIR: &str = ".claude/hooks";
const SETTINGS_REL_PATH: &str = ".claude/settings.json";
const SKILL_REL_PATH: &str = ".claude/skills/agnosgram/SKILL.md";

const SESSION_START_SCRIPT: &str = "agnosgram-session-start.mjs";
const STOP_SCRIPT: &str = "agnosgram-stop-reminder.mjs";

const SESSION_START_SCRIPT_CONTENT: &str = "#!/usr/bin/env node
// Managed by agnosgram (`adapt --claude-hooks`). Safe to regenerate; re-run that
// command to refresh after an upgrade.
import { execSync } from \"node:child_process\";

let context;
try {
  context = execSync(\"agnosgram pack\", { encoding: \"utf8\", stdio: [\"ignore\", \"pipe\", \"pipe\"] });
} catch (err) {
  const detail = err && err.stderr ? err.stderr.toString().trim() : err && err.message ? err.message : String(err);
  context = `(agnosgram pack failed - is it installed and is this an agnosgram project? ${detail})`;
}

process.stdout.write(
  JSON.stringify({
    hookSpecificOutput: {
      hookEventName: \"SessionStart\",
      additionalContext: context,
    },
  }),
);
";

const STOP_SCRIPT_CONTENT: &str = "#!/usr/bin/env node
// Managed by agnosgram (`adapt --claude-hooks`). Safe to regenerate; re-run that
// command to refresh after an upgrade.
import { readFileSync } from \"node:fs\";

let input = {};
try {
  input = JSON.parse(readFileSync(0, \"utf8\"));
} catch {
  input = {};
}

// A Stop hook can re-fire itself into a loop; stop_hook_active is Claude Code's
// signal that this hook already ran once this turn, so back off instead of nagging.
if (input.stop_hook_active) {
  process.exit(0);
}

process.stdout.write(
  JSON.stringify({
    hookSpecificOutput: {
      hookEventName: \"Stop\",
      additionalContext:
        \"Reminder: if anything worth remembering happened this session, run \" +
        \"`agnosgram log --did .. --learned .. --decided .. --avoid .. --next ..` \" +
        \"before you finish. This is a reminder only - deciding whether to log, and what to say, is yours.\",
    },
  }),
);
";

const SKILL_CONTENT: &str = "---
name: agnosgram
description: Read and update this project's Agnosgram memory (.agnosgram/) - status, lessons, decisions, and the session journal. Use when you need project context beyond what's in the code, or to record what happened this session.
---

# Agnosgram

This project keeps durable, agent-agnostic memory in `.agnosgram/` (plain Markdown,
reviewed in PRs). The CLI never calls an LLM and sends no telemetry.

## When to use each command
- **Session start:** `agnosgram pack` prints a token-budgeted bundle (status +
  lessons, + decisions if scoped). A SessionStart hook may have already loaded this
  into context - check before re-running.
- **Looking for something specific:** `agnosgram show <topic>` prints records
  matching an id, scope tag, or type.
- **Before trusting a plan against past failures:** `agnosgram advise <plan-path>`
  emits a contradiction-review prompt cross-checking the plan against
  `lessons/` and `decisions/`.
- **Session end:** `agnosgram log --did \"..\" --learned \"..\" --decided \"..\" --avoid \"..\" --next \"..\"`
  (at least one flag) appends a journal entry. A Stop hook may remind you to do this.
- **Something looks stale or off:** `agnosgram doctor` lints the store (schema,
  staleness, budgets, broken links, safety lints).

## Rules
- Never hand-edit `lessons/`, `decisions/`, or `journal/` directly - use
  `agnosgram log` to capture, `agnosgram distill` to curate.
- `state/status.md` is small and volatile - safe to overwrite freely.
- Read `.agnosgram/MEMORY.md` for the full reading protocol.
";

pub struct ClaudeHooksResult {
    pub written: Vec<WriteResult>,
}

/// True when a previous `--claude-hooks` run already installed the SessionStart hook.
pub fn hooks_installed(root: &Path) -> bool {
    root.join(HOOKS_REL_DIR).join(SESSION_START_SCRIPT).exists()
}

fn is_own_hook(hook: &Value, script_name: &str) -> bool {
    match hook.get("command") {
        Some(Value::String(cmd)) => cmd.contains(script_name),
        _ => false,
    }
}

/// Insert or refresh this tool's own entry within one hook event's array,
/// touching nothing else. Bails loudly if the existing shape isn't something
/// we can safely merge into, rather than guessing.
fn upsert_hook_group(
    existing: Option<Value>,
    event_name: &str,
    matcher: Option<&str>,
    hook_obj: Value,
    script_name: &str,
) -> Result<Value, UserError> {
    let mut groups: Vec<Value> = match existing {
        None => Vec::new(),
        Some(Value::Array(items)) => items,
        Some(_) => {
            return Err(UserError::new(format!(
                "{SETTINGS_REL_PATH}: hooks.{event_name} is not an array; merge --claude-hooks by hand."
            )))
        }
    };
    for g in &groups {
        let hooks_ok = matches!(g.get("hooks"), Some(Value::Array(_)));
        if !hooks_ok {
            return Err(UserError::new(format!(
                "{SETTINGS_REL_PATH}: a hooks.{event_name} entry is malformed; merge --claude-hooks by hand."
            )));
        }
    }

    let group_idx = groups.iter().position(|g| {
        matches!(g.get("hooks"), Some(Value::Array(hooks)) if hooks.iter().any(|h| is_own_hook(h, script_name)))
    });

    if let Some(idx) = group_idx {
        if let Value::Object(entries) = &mut groups[idx] {
            if let Some((_, Value::Array(hooks))) = entries.iter_mut().find(|(k, _)| k == "hooks") {
                if let Some(hidx) = hooks.iter().position(|h| is_own_hook(h, script_name)) {
                    hooks[hidx] = hook_obj;
                }
            }
        }
        return Ok(Value::Array(groups));
    }

    let mut new_group = Value::object();
    if let Some(m) = matcher {
        new_group.insert("matcher", m);
    }
    new_group.insert("hooks", Value::Array(vec![hook_obj]));
    groups.push(new_group);
    Ok(Value::Array(groups))
}

fn hook_command_obj(command: String, timeout: i64) -> Value {
    let mut v = Value::object();
    v.insert("type", "command");
    v.insert("command", command);
    v.insert("timeout", timeout);
    v
}

pub fn install_claude_hooks(root: &Path) -> Result<ClaudeHooksResult, UserError> {
    let mut written = Vec::new();

    written.push(write_if_changed(
        root,
        &format!("{HOOKS_REL_DIR}/{SESSION_START_SCRIPT}"),
        SESSION_START_SCRIPT_CONTENT,
        WriteOpts { executable: true },
    )?);
    written.push(write_if_changed(
        root,
        &format!("{HOOKS_REL_DIR}/{STOP_SCRIPT}"),
        STOP_SCRIPT_CONTENT,
        WriteOpts { executable: true },
    )?);

    let settings_path = root.join(SETTINGS_REL_PATH);
    let mut settings: Value = Value::object();
    if settings_path.exists() {
        let raw = fs::read_to_string(&settings_path).map_err(|e| UserError::new(e.to_string()))?;
        settings = crate::core::json::parse(&raw).map_err(|_| {
            UserError::new(format!(
                "{SETTINGS_REL_PATH} is not valid JSON; fix it by hand before using --claude-hooks."
            ))
        })?;
        if !matches!(settings, Value::Object(_)) {
            return Err(UserError::new(format!(
                "{SETTINGS_REL_PATH} does not contain a JSON object; cannot merge hooks automatically."
            )));
        }
    }

    let hooks_raw = settings.get("hooks").cloned();
    let mut hooks_value: Value = match &hooks_raw {
        None => Value::object(),
        Some(Value::Object(entries)) => Value::Object(entries.clone()),
        Some(_) => {
            return Err(UserError::new(format!(
                "{SETTINGS_REL_PATH}: \"hooks\" is not an object; cannot merge automatically."
            )))
        }
    };

    let session_start_hook = hook_command_obj(
        format!("node \"${{CLAUDE_PROJECT_DIR}}/{HOOKS_REL_DIR}/{SESSION_START_SCRIPT}\""),
        15,
    );
    let stop_hook = hook_command_obj(
        format!("node \"${{CLAUDE_PROJECT_DIR}}/{HOOKS_REL_DIR}/{STOP_SCRIPT}\""),
        5,
    );

    let session_start_existing = hooks_value.get("SessionStart").cloned();
    let new_session_start = upsert_hook_group(
        session_start_existing,
        "SessionStart",
        Some("startup|resume|clear|compact|fork"),
        session_start_hook,
        SESSION_START_SCRIPT,
    )?;
    hooks_value.insert("SessionStart", new_session_start);

    let stop_existing = hooks_value.get("Stop").cloned();
    let new_stop = upsert_hook_group(stop_existing, "Stop", None, stop_hook, STOP_SCRIPT)?;
    hooks_value.insert("Stop", new_stop);

    settings.insert("hooks", hooks_value);

    let next = crate::core::json::stringify_pretty(&settings) + "\n";
    written.push(write_if_changed(
        root,
        SETTINGS_REL_PATH,
        &next,
        WriteOpts::default(),
    )?);

    written.push(write_if_changed(
        root,
        SKILL_REL_PATH,
        SKILL_CONTENT,
        WriteOpts::default(),
    )?);

    Ok(ClaudeHooksResult { written })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::write_file::WriteAction;
    use std::fs;

    fn tmp_root(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("agnos-hooks-rs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn creates_hook_scripts_settings_and_skill_file() {
        let root = tmp_root("basic");
        let result = install_claude_hooks(&root).unwrap();
        let mut paths: Vec<&str> = result.written.iter().map(|w| w.path.as_str()).collect();
        paths.sort();
        assert_eq!(
            paths,
            vec![
                ".claude/hooks/agnosgram-session-start.mjs",
                ".claude/hooks/agnosgram-stop-reminder.mjs",
                ".claude/settings.json",
                ".claude/skills/agnosgram/SKILL.md",
            ]
        );
        assert!(result
            .written
            .iter()
            .all(|w| w.action == WriteAction::Created));

        let settings_text = fs::read_to_string(root.join(".claude/settings.json")).unwrap();
        let settings = crate::core::json::parse(&settings_text).unwrap();
        let session_start = settings.get("hooks").unwrap().get("SessionStart").unwrap();
        let first_group = &session_start.as_array().unwrap()[0];
        let cmd = first_group.get("hooks").unwrap().as_array().unwrap()[0]
            .get("command")
            .unwrap()
            .as_str()
            .unwrap();
        assert!(cmd.contains("agnosgram-session-start.mjs"));

        let skill = fs::read_to_string(root.join(".claude/skills/agnosgram/SKILL.md")).unwrap();
        assert!(skill.starts_with("---\nname: agnosgram"));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn is_idempotent_running_twice_reports_unchanged() {
        let root = tmp_root("idempotent");
        install_claude_hooks(&root).unwrap();
        let before = fs::read_to_string(root.join(".claude/settings.json")).unwrap();
        let second = install_claude_hooks(&root).unwrap();
        assert!(second
            .written
            .iter()
            .all(|w| w.action == WriteAction::Unchanged));
        let after = fs::read_to_string(root.join(".claude/settings.json")).unwrap();
        assert_eq!(before, after);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn preserves_unrelated_hooks_and_settings() {
        let root = tmp_root("preserve");
        fs::create_dir_all(root.join(".claude")).unwrap();
        let user_settings = r#"{
  "permissions": { "allow": ["Bash(git *)"] },
  "hooks": {
    "SessionStart": [{ "matcher": "startup", "hooks": [{ "type": "command", "command": "echo user-hook" }] }],
    "PreToolUse": [{ "matcher": "Bash", "hooks": [{ "type": "command", "command": "echo audit" }] }]
  }
}
"#;
        fs::write(root.join(".claude/settings.json"), user_settings).unwrap();

        install_claude_hooks(&root).unwrap();

        let settings_text = fs::read_to_string(root.join(".claude/settings.json")).unwrap();
        let settings = crate::core::json::parse(&settings_text).unwrap();
        assert!(settings.get("permissions").is_some());
        let session_start = settings
            .get("hooks")
            .unwrap()
            .get("SessionStart")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(session_start.len(), 2);
        assert_eq!(
            session_start[0].get("hooks").unwrap().as_array().unwrap()[0]
                .get("command")
                .unwrap()
                .as_str()
                .unwrap(),
            "echo user-hook"
        );

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn refreshing_updates_own_entry_in_place_instead_of_duplicating() {
        let root = tmp_root("refresh");
        install_claude_hooks(&root).unwrap();
        install_claude_hooks(&root).unwrap();
        let settings_text = fs::read_to_string(root.join(".claude/settings.json")).unwrap();
        let settings = crate::core::json::parse(&settings_text).unwrap();
        assert_eq!(
            settings
                .get("hooks")
                .unwrap()
                .get("SessionStart")
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            settings
                .get("hooks")
                .unwrap()
                .get("Stop")
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            1
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn errors_cleanly_when_settings_json_is_not_valid_json() {
        let root = tmp_root("badjson");
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::write(root.join(".claude/settings.json"), "{ not json").unwrap();
        assert!(install_claude_hooks(&root).is_err());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn errors_cleanly_when_session_start_is_not_an_array() {
        let root = tmp_root("badarray");
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::write(
            root.join(".claude/settings.json"),
            r#"{"hooks":{"SessionStart":"not-an-array"}}"#,
        )
        .unwrap();
        assert!(install_claude_hooks(&root).is_err());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn hook_commands_quote_claude_project_dir() {
        let root = tmp_root("quote");
        install_claude_hooks(&root).unwrap();
        let settings_text = fs::read_to_string(root.join(".claude/settings.json")).unwrap();
        let settings = crate::core::json::parse(&settings_text).unwrap();
        let hooks = settings.get("hooks").unwrap();
        let session_cmd = hooks.get("SessionStart").unwrap().as_array().unwrap()[0]
            .get("hooks")
            .unwrap()
            .as_array()
            .unwrap()[0]
            .get("command")
            .unwrap()
            .as_str()
            .unwrap();
        let stop_cmd = hooks.get("Stop").unwrap().as_array().unwrap()[0]
            .get("hooks")
            .unwrap()
            .as_array()
            .unwrap()[0]
            .get("command")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(
            session_cmd,
            "node \"${CLAUDE_PROJECT_DIR}/.claude/hooks/agnosgram-session-start.mjs\""
        );
        assert_eq!(
            stop_cmd,
            "node \"${CLAUDE_PROJECT_DIR}/.claude/hooks/agnosgram-stop-reminder.mjs\""
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn session_start_matcher_includes_fork() {
        let root = tmp_root("matcher");
        install_claude_hooks(&root).unwrap();
        let settings_text = fs::read_to_string(root.join(".claude/settings.json")).unwrap();
        let settings = crate::core::json::parse(&settings_text).unwrap();
        let matcher = settings
            .get("hooks")
            .unwrap()
            .get("SessionStart")
            .unwrap()
            .as_array()
            .unwrap()[0]
            .get("matcher")
            .unwrap()
            .as_str()
            .unwrap();
        let mut sources: Vec<&str> = matcher.split('|').collect();
        sources.sort();
        assert_eq!(
            sources,
            vec!["clear", "compact", "fork", "resume", "startup"]
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn refreshing_never_touches_an_existing_matcher_it_did_not_write() {
        let root = tmp_root("keepmatcher");
        fs::create_dir_all(root.join(".claude")).unwrap();
        let existing = r#"{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup",
        "hooks": [
          { "type": "command", "command": "echo co-located-user-hook" },
          { "type": "command", "command": "node \"${CLAUDE_PROJECT_DIR}/.claude/hooks/agnosgram-session-start.mjs\"" }
        ]
      }
    ]
  }
}
"#;
        fs::write(root.join(".claude/settings.json"), existing).unwrap();

        install_claude_hooks(&root).unwrap();

        let settings_text = fs::read_to_string(root.join(".claude/settings.json")).unwrap();
        let settings = crate::core::json::parse(&settings_text).unwrap();
        let session_start = settings
            .get("hooks")
            .unwrap()
            .get("SessionStart")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(session_start.len(), 1);
        assert_eq!(
            session_start[0].get("matcher").unwrap().as_str().unwrap(),
            "startup"
        );
        assert_eq!(
            session_start[0].get("hooks").unwrap().as_array().unwrap()[0]
                .get("command")
                .unwrap()
                .as_str()
                .unwrap(),
            "echo co-located-user-hook"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn hook_scripts_are_written_executable() {
        let root = tmp_root("exec");
        install_claude_hooks(&root).unwrap();
        let path = root.join(".claude/hooks/agnosgram-session-start.mjs");
        assert!(path.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode();
            assert!(mode & 0o100 != 0);
        }
        fs::remove_dir_all(&root).unwrap();
    }
}
