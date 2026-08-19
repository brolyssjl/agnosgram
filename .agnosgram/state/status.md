# Status

_Small and volatile. Overwrite freely; history lives in the journal._

- **Focus:** FRI-002 resolved (install story): releases from `0.9.0` onward
  carry `agnosgram-linux-x64` / `agnosgram-darwin-arm64` binary assets, so
  `install.sh`'s primary path works; binaries are the canonical distribution
  and README/docs/install.md now document the read-only-global-npm-prefix
  case (nix, managed machines); `install.sh`'s fallback no longer dead-ends
  into a guaranteed-404 `npm install -g agnosgram` - it prints
  build-from-source steps instead. `0.11.0` tagged and released.
- **In flight:** nothing.
- **Next:** Milestone 6 Round 2 (the Rust port).
- **Blocked on:** nothing.

_Last updated: 2026-08-19_
