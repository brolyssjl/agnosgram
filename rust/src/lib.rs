//! Library face of the `agnosgram` crate, exposing `core::*` so
//! `rust/tests/*.rs` integration tests can exercise the real shipped
//! encoders/estimators directly (e.g. `bench_tokens.rs`) instead of
//! reimplementing them. The CLI binary (`src/main.rs`) does not depend on
//! this - it declares its own module tree, compiled separately, same as
//! before this existed.

pub mod adapters;
pub mod claude_hooks;
pub mod commands;
pub mod core;
