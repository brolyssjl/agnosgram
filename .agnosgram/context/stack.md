# Stack

_Languages, tooling, and the exact commands to build / test / lint. Pin versions
where they matter._

## Commands
- **Build:** `cargo build --release --manifest-path rust/Cargo.toml` -> `rust/target/release/agnosgram`
- **Test:** `cargo test --manifest-path rust/Cargo.toml` (unit tests + `rust/tests/*_conformance.rs` + the token benchmark gates in `rust/tests/bench_tokens.rs`)
- **Lint:** `cargo fmt --check --manifest-path rust/Cargo.toml`, `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings`

## Versions & constraints
- Rust, edition 2021, stable toolchain. Single crate (`rust/`, package `agnosgram`) with both a `lib` and a `bin` target - the lib exists so `rust/tests/` can call the shipped `core::*` code directly instead of reimplementing it.
- **Zero dependencies** - non-negotiable (see decisions/0001, extended to Rust at the port). No `[dependencies]`, no `[dev-dependencies]`: JSON, YAML, TOON, and frontmatter are hand-rolled; local time uses a direct `extern "C"` binding to `localtime_r`; `rust/tests/` integration tests use `std::process::Command` only, no `assert_cmd`.
- CLI arg parsing: hand-rolled in `rust/src/core/args.rs`. Tests: `#[cfg(test)] mod tests` (unit) + `rust/tests/*_conformance.rs` (integration, spawns the real binary).
- No npm anywhere: no `package.json`, no Node toolchain. The TypeScript implementation this crate was ported from was retired 2026-08-22.
