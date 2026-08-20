//! Port of `src/core/output.ts`: the shared error type and stdout/stderr
//! helpers every command uses instead of writing to the streams directly.

use std::fmt;
use std::io::Write;

use super::json::Value;
use super::serialize::{self, Format};

/// Port of the TS `UserError` class: a reported failure (as opposed to a bug),
/// caught in `main` and turned into `error: <message>` on stderr + exit 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserError(pub String);

impl UserError {
    pub fn new(message: impl Into<String>) -> Self {
        UserError(message.into())
    }
}

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for UserError {}

/// `info(msg)`: `process.stdout.write(msg + "\n")`.
pub fn info(msg: &str) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(msg.as_bytes());
    let _ = lock.write_all(b"\n");
}

/// `warn(msg)`: `process.stderr.write(msg + "\n")`.
pub fn warn(msg: &str) {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    let _ = lock.write_all(msg.as_bytes());
    let _ = lock.write_all(b"\n");
}

/// `printJson(value)`.
pub fn print_json(value: &Value) {
    info(&serialize::serialize(value, Format::Json));
}

/// `printStructured(value, format)`.
pub fn print_structured(value: &Value, format: Format) {
    info(&serialize::serialize(value, format));
}
