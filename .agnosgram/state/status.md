# Status

_Small and volatile. Overwrite freely; history lives in the journal._

- **Focus:** Milestone 6 Round 2 (the Rust port) implemented on branch
  `milestone-6-rust-port`: zero-dependency crate at `rust/` (1:1 file map to
  the TS source, plan in `docs/rust-port.md`), 202 Rust unit tests, and the
  full conformance suite green against the Rust binary on darwin-arm64
  (104/104 via `AGNOSGRAM_BIN`). CI gains a rust job on ubuntu + macos (the
  ubuntu leg is the linux-x64 proof) and the release pipeline now builds
  cargo binaries with a tag/package.json/Cargo.toml version guard and a
  conformance hard gate.
- **In flight:** PR #11 (`milestone-6-rust-port`), CI fully green including
  the ubuntu rust leg (linux-x64 conformance proven), Round 2 roadmap boxes
  ticked. Awaiting merge.
- **Next:** merge the port PR, then Round 3 (upgrade story, rc soak on the
  real host projects, docs site + case study, `1.0.0` with the Rust binary
  canonical).
- **Blocked on:** nothing.

_Last updated: 2026-08-20_
