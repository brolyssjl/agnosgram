//! One module per `src/core/*.ts` file, same names, ported 1:1 against its
//! TS counterpart (see `docs/rust-port.md`). `json.rs` has no TS counterpart;
//! it exists because std Rust has no JSON support and the dependency policy
//! is zero external crates.

pub mod args;
pub mod config;
pub mod dates;
pub mod detect;
pub mod freshness;
pub mod frontmatter;
pub mod json;
pub mod lint;
pub mod markers;
pub mod meta;
pub mod output;
pub mod paths;
pub mod records;
pub mod serialize;
pub mod store;
pub mod templates;
pub mod tokens;
pub mod toon;
pub mod yaml;
