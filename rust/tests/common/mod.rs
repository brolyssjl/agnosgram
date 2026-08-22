//! Shared harness for the conformance test suite (`tests/*_conformance.rs`).
//! Exercises the CLI surface by spawning a real binary - never by calling
//! command internals - so the exact same behavior can be checked against any
//! implementation of the frozen surface via `AGNOSGRAM_BIN` (see
//! CONTRIBUTING.md). Falls back to the binary this crate just built.
//! std only - no dev-dependencies (CON-001: zero-dependency constraint).
#![allow(dead_code)]

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct CliRun {
    pub stdout: String,
    pub stderr: String,
    pub status: i32,
}

fn bin_command() -> Vec<String> {
    match env::var("AGNOSGRAM_BIN") {
        Ok(over) if !over.trim().is_empty() => over.split_whitespace().map(String::from).collect(),
        _ => vec![env!("CARGO_BIN_EXE_agnosgram").to_string()],
    }
}

/// Run one CLI invocation with the given args and working directory.
pub fn run_cli(args: &[&str], cwd: &Path) -> CliRun {
    run_cli_stdin(args, cwd, None)
}

/// Run one CLI invocation, optionally piping `input` to stdin.
pub fn run_cli_stdin(args: &[&str], cwd: &Path, input: Option<&str>) -> CliRun {
    let parts = bin_command();
    let (cmd, prefix) = parts
        .split_first()
        .expect("AGNOSGRAM_BIN must not be empty");
    let mut command = Command::new(cmd);
    command
        .args(prefix)
        .args(args)
        .current_dir(cwd)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("failed to spawn agnosgram binary");
    if let Some(text) = input {
        child
            .stdin
            .take()
            .expect("stdin was piped")
            .write_all(text.as_bytes())
            .expect("failed to write stdin");
    }
    let output = child
        .wait_with_output()
        .expect("failed to wait on agnosgram binary");
    CliRun {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        status: output.status.code().unwrap_or(-1),
    }
}

/// A self-cleaning temp directory (std-only stand-in for `mkdtemp`).
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(prefix: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos();
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!("{prefix}-{}-{nanos}-{n}", std::process::id()));
        fs::create_dir_all(&path).expect("failed to create temp dir");
        TempDir { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Runs `init --adapt none` in `root` and panics unless it exits 0. Mirrors
/// the shared `beforeEach` scaffolding step used across the TS conformance
/// suite.
pub fn init_store(root: &Path) {
    let res = run_cli(&["init", "--adapt", "none"], root);
    assert_eq!(res.status, 0, "init --adapt none failed: {}", res.stderr);
}

/// Writes `content` to `root/.agnosgram/<rel>`, creating parent dirs.
pub fn write_store_file(root: &Path, rel: &str, content: &str) {
    let full = root.join(".agnosgram").join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("failed to create parent dirs");
    }
    fs::write(&full, content).expect("failed to write store file");
}

/// The sole `YYYY-MM.md` journal file under `.agnosgram/journal/` (not
/// `archive/`), read from disk instead of recomputed - avoids reimplementing
/// calendar math with std-only integration tests.
pub fn current_journal_month(root: &Path) -> String {
    let dir = root.join(".agnosgram").join("journal");
    let mut months: Vec<String> = fs::read_dir(&dir)
        .expect("failed to read journal dir")
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| is_month_filename(name))
        .map(|name| name.trim_end_matches(".md").to_string())
        .collect();
    months.sort();
    months
        .pop()
        .expect("expected at least one YYYY-MM.md journal file")
}

fn is_month_filename(name: &str) -> bool {
    let stem = match name.strip_suffix(".md") {
        Some(s) => s,
        None => return false,
    };
    let bytes = stem.as_bytes();
    bytes.len() == 7
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
}

/// True if `s` is exactly `\d+\.\d+\.\d+` (a bare semver, no pre-release).
pub fn looks_like_semver(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// True if any line, trimmed, equals `line` exactly (stand-in for a `^...$`
/// multiline regex anchor).
pub fn has_exact_line(haystack: &str, line: &str) -> bool {
    haystack.lines().any(|l| l.trim() == line)
}

/// Recursive snapshot of every regular file's mtime under `dir`, keyed by
/// path relative to `dir` - used for before/after no-writes assertions.
pub fn snapshot(dir: &Path) -> std::collections::BTreeMap<PathBuf, SystemTime> {
    let mut out = std::collections::BTreeMap::new();
    snapshot_into(dir, dir, &mut out);
    out
}

/// Minimal JSON value + parser (std-only stand-in for `JSON.parse`), used to
/// make `--json` assertions robust instead of substring-matching formatted
/// output. Parsing only - tests never need to emit JSON.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn parse(input: &str) -> Json {
        let chars: Vec<char> = input.chars().collect();
        let mut pos = 0usize;
        let value = parse_value(&chars, &mut pos).expect("invalid JSON in test fixture/output");
        skip_ws(&chars, &mut pos);
        assert!(pos == chars.len(), "trailing content after JSON value");
        value
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Json>> {
        match self {
            Json::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }
}

fn skip_ws(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos].is_whitespace() {
        *pos += 1;
    }
}

fn parse_value(chars: &[char], pos: &mut usize) -> Option<Json> {
    skip_ws(chars, pos);
    match chars.get(*pos)? {
        '{' => parse_object(chars, pos),
        '[' => parse_array(chars, pos),
        '"' => parse_string(chars, pos).map(Json::String),
        't' => parse_literal(chars, pos, "true", Json::Bool(true)),
        'f' => parse_literal(chars, pos, "false", Json::Bool(false)),
        'n' => parse_literal(chars, pos, "null", Json::Null),
        _ => parse_number(chars, pos),
    }
}

fn parse_literal(chars: &[char], pos: &mut usize, lit: &str, value: Json) -> Option<Json> {
    let lit_chars: Vec<char> = lit.chars().collect();
    if chars.len() - *pos < lit_chars.len() {
        return None;
    }
    if chars[*pos..*pos + lit_chars.len()] == lit_chars[..] {
        *pos += lit_chars.len();
        Some(value)
    } else {
        None
    }
}

fn parse_number(chars: &[char], pos: &mut usize) -> Option<Json> {
    let start = *pos;
    if chars.get(*pos) == Some(&'-') {
        *pos += 1;
    }
    while chars.get(*pos).is_some_and(|c| c.is_ascii_digit()) {
        *pos += 1;
    }
    if chars.get(*pos) == Some(&'.') {
        *pos += 1;
        while chars.get(*pos).is_some_and(|c| c.is_ascii_digit()) {
            *pos += 1;
        }
    }
    if matches!(chars.get(*pos), Some('e') | Some('E')) {
        *pos += 1;
        if matches!(chars.get(*pos), Some('+') | Some('-')) {
            *pos += 1;
        }
        while chars.get(*pos).is_some_and(|c| c.is_ascii_digit()) {
            *pos += 1;
        }
    }
    if *pos == start {
        return None;
    }
    let text: String = chars[start..*pos].iter().collect();
    text.parse::<f64>().ok().map(Json::Number)
}

fn parse_string(chars: &[char], pos: &mut usize) -> Option<String> {
    if chars.get(*pos) != Some(&'"') {
        return None;
    }
    *pos += 1;
    let mut out = String::new();
    loop {
        let c = *chars.get(*pos)?;
        *pos += 1;
        match c {
            '"' => return Some(out),
            '\\' => {
                let esc = *chars.get(*pos)?;
                *pos += 1;
                match esc {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'u' => {
                        let hex: String = chars.get(*pos..*pos + 4)?.iter().collect();
                        *pos += 4;
                        let code = u32::from_str_radix(&hex, 16).ok()?;
                        out.push(char::from_u32(code)?);
                    }
                    other => out.push(other),
                }
            }
            other => out.push(other),
        }
    }
}

fn parse_array(chars: &[char], pos: &mut usize) -> Option<Json> {
    *pos += 1; // consume '['
    let mut items = Vec::new();
    skip_ws(chars, pos);
    if chars.get(*pos) == Some(&']') {
        *pos += 1;
        return Some(Json::Array(items));
    }
    loop {
        let value = parse_value(chars, pos)?;
        items.push(value);
        skip_ws(chars, pos);
        match chars.get(*pos) {
            Some(',') => {
                *pos += 1;
            }
            Some(']') => {
                *pos += 1;
                return Some(Json::Array(items));
            }
            _ => return None,
        }
    }
}

fn parse_object(chars: &[char], pos: &mut usize) -> Option<Json> {
    *pos += 1; // consume '{'
    let mut entries = Vec::new();
    skip_ws(chars, pos);
    if chars.get(*pos) == Some(&'}') {
        *pos += 1;
        return Some(Json::Object(entries));
    }
    loop {
        skip_ws(chars, pos);
        let key = parse_string(chars, pos)?;
        skip_ws(chars, pos);
        if chars.get(*pos) != Some(&':') {
            return None;
        }
        *pos += 1;
        let value = parse_value(chars, pos)?;
        entries.push((key, value));
        skip_ws(chars, pos);
        match chars.get(*pos) {
            Some(',') => {
                *pos += 1;
            }
            Some('}') => {
                *pos += 1;
                return Some(Json::Object(entries));
            }
            _ => return None,
        }
    }
}

fn snapshot_into(
    dir: &Path,
    base: &Path,
    out: &mut std::collections::BTreeMap<PathBuf, SystemTime>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let full = entry.path();
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            snapshot_into(&full, base, out);
        } else if file_type.is_file() {
            let mtime = full
                .metadata()
                .and_then(|m| m.modified())
                .expect("failed to read mtime");
            let rel = full
                .strip_prefix(base)
                .expect("file must be under base")
                .to_path_buf();
            out.insert(rel, mtime);
        }
    }
}
