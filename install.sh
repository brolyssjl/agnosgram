#!/usr/bin/env bash
# Install `agnosgram` - no npm involved, ever (Milestone 6 owner decision).
#
# Primary path: fetch the single-file binary for this platform from the
# latest GitHub release, verify it against the release's SHA256SUMS, and
# place it on PATH. Falls back to build-from-source instructions (cargo)
# when no matching binary asset exists yet, or when verification fails.
#
# Built platforms (keep in sync with .github/workflows/release.yml's
# build-binaries matrix - update both together when adding a platform):
# linux-x64, darwin-arm64.
#
# Env vars:
#   AGNOSGRAM_INSTALL_DIR  where to install (default: $HOME/.local/bin)
#   AGNOSGRAM_VERSION      pin a version, e.g. "1.0.0" (default: latest release)
#   AGNOSGRAM_KEEP_OLD_INSTALLS  set to 1 to keep obsolete pre-Rust install
#                          dirs (~/.local/share/brainstorm-tools/agnosgram-v*)
#                          instead of removing them after a verified install
#
# Usage: curl -fsSL https://raw.githubusercontent.com/brolyssjl/agnosgram/main/install.sh | bash
set -euo pipefail

REPO="brolyssjl/agnosgram"
BIN_NAME="agnosgram"
INSTALL_DIR="${AGNOSGRAM_INSTALL_DIR:-$HOME/.local/bin}"
BUILT_PLATFORMS="linux-x64, darwin-arm64"

os() {
  case "$(uname -s)" in
    Linux) echo "linux" ;;
    Darwin) echo "darwin" ;;
    *) echo "unsupported" ;;
  esac
}

arch() {
  case "$(uname -m)" in
    x86_64|amd64) echo "x64" ;;
    arm64|aarch64) echo "arm64" ;;
    *) echo "unsupported" ;;
  esac
}

resolve_tag() {
  # Empty means "latest" (the /releases/latest/download/ redirect path).
  # Pinned means the download URL/gh call targets an exact release tag.
  if [ -n "${AGNOSGRAM_VERSION:-}" ]; then
    echo "v${AGNOSGRAM_VERSION}"
  fi
}

download_asset() {
  local asset="$1" dest="$2" tag="$3"
  local url
  if [ -n "$tag" ]; then
    url="https://github.com/${REPO}/releases/download/${tag}/${asset}"
  else
    # /releases/latest/download/<asset> redirects straight to the current
    # release's asset - no need to resolve the tag via the (rate-limited)
    # api.github.com first. Works unauthenticated.
    url="https://github.com/${REPO}/releases/latest/download/${asset}"
  fi
  if curl -fsSL "$url" -o "$dest"; then
    return 0
  fi
  # Fall back to gh if the plain download failed (rate limiting, a flaky
  # network, or similar). gh reuses your existing auth and sees the same assets.
  if command -v gh >/dev/null 2>&1; then
    echo "Direct download failed - retrying via gh..." >&2
    if gh release download ${tag:+"$tag"} --repo "$REPO" --pattern "$asset" --output "$dest" --clobber; then
      return 0
    fi
  fi
  return 1
}

# Verifies `file` against the entry for `asset` in `sums_file` (a
# SHA256SUMS-format file: "<hash>  <name>" per line, one line per released
# asset). Portable across macOS (shasum) and Linux (sha256sum) - the hash is
# checked against `file`'s actual path, not the original asset name, since
# `file` is a temp download, not yet moved into place.
verify_checksum() {
  local file="$1" asset="$2" sums_file="$3"
  local expected
  expected="$(awk -v a="$asset" '$2==a { print $1; exit }' "$sums_file")"
  if [ -z "$expected" ]; then
    echo "SHA256SUMS has no entry for ${asset}." >&2
    return 1
  fi
  if command -v sha256sum >/dev/null 2>&1; then
    printf '%s  %s\n' "$expected" "$file" | sha256sum -c - >/dev/null 2>&1
  elif command -v shasum >/dev/null 2>&1; then
    printf '%s  %s\n' "$expected" "$file" | shasum -a 256 -c - >/dev/null 2>&1
  else
    echo "Neither sha256sum nor shasum is available - cannot verify checksum." >&2
    return 1
  fi
}

# Best-effort build-provenance check on top of the checksum above. The
# checksum only proves the binary matches SHA256SUMS from the same release -
# it says nothing if both were replaced together (a compromised GitHub
# account or Actions token can do that). `gh attestation verify` checks the
# binary against the SLSA provenance attestation release.yml records via
# `actions/attest-build-provenance`, which is signed through GitHub's OIDC
# issuer and Sigstore, not just committed alongside the asset.
#
# Skipped (with a note, not a failure) when `gh` isn't installed, since it's
# the only tool that can check this. Treated as a pass when the release
# predates attestations - older releases have none, and gh (as of 2.96.0)
# reports that two different ways depending on how it resolved the subject:
# a plain "no attestations found", or (verified against the real
# gate-linux-x64/agnosgram-darwin-arm64 v1.5.1 assets, which predate this
# feature) an HTTP 404 from the attestations API, e.g.
#   Error: HTTP 404: Not Found (https://api.github.com/repos/OWNER/REPO/attestations/sha256:...?per_page=30&predicate_type=...)
# Any other failure (signature mismatch, wrong repo, a different HTTP error)
# aborts the install and removes the temp files, same as a checksum mismatch.
verify_provenance() {
  local file="$1" asset="$2"

  if ! command -v gh >/dev/null 2>&1; then
    echo "Note: gh not found - build provenance not checked (checksum verified above). Install gh and re-run to also verify: gh attestation verify <file> --repo ${REPO}" >&2
    return 0
  fi

  local out
  if out="$(gh attestation verify "$file" --repo "$REPO" 2>&1)"; then
    echo "$out"
    echo "provenance verified"
    return 0
  fi

  if echo "$out" | grep -Eqi 'no attestations found|HTTP 404.*attestations/'; then
    echo "Note: no build attestations found for ${asset} (older releases predate provenance) - continuing on checksum verification alone." >&2
    return 0
  fi

  echo "$out" >&2
  echo "provenance verification FAILED for ${asset} - the release may have been tampered with. Nothing installed." >&2
  return 1
}

install_binary() {
  local platform="$1" cpu="$2" tag="$3"
  local asset="${BIN_NAME}-${platform}-${cpu}"
  local tmp sums_tmp
  tmp="$(mktemp "${TMPDIR:-/tmp}/${BIN_NAME}.XXXXXX")"
  sums_tmp="$(mktemp "${TMPDIR:-/tmp}/${BIN_NAME}-sums.XXXXXX")"

  echo "Downloading ${asset}..."
  mkdir -p "$INSTALL_DIR"
  # Download to a temp file first: a mid-transfer failure must never leave a
  # truncated (but still +x, still shadowing-the-fallback) binary in place.
  if ! download_asset "$asset" "$tmp" "$tag"; then
    rm -f "$tmp" "$sums_tmp"
    return 1
  fi

  echo "Downloading SHA256SUMS..."
  if ! download_asset "SHA256SUMS" "$sums_tmp" "$tag"; then
    echo "Could not download SHA256SUMS - aborting, nothing installed." >&2
    rm -f "$tmp" "$sums_tmp"
    return 1
  fi

  if ! verify_checksum "$tmp" "$asset" "$sums_tmp"; then
    echo "Checksum verification FAILED for ${asset} - aborting, nothing installed." >&2
    rm -f "$tmp" "$sums_tmp"
    return 1
  fi
  echo "Checksum verified."
  rm -f "$sums_tmp"

  if ! verify_provenance "$tmp" "$asset"; then
    rm -f "$tmp"
    return 1
  fi

  chmod +x "$tmp"
  mv "$tmp" "$INSTALL_DIR/$BIN_NAME"
  echo "Installed to $INSTALL_DIR/$BIN_NAME"
  case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) echo "Add it to your PATH: export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
  esac
  local installed_version
  if installed_version="$("$INSTALL_DIR/$BIN_NAME" --version 2>/dev/null)"; then
    echo "Installed version: ${installed_version}"
  fi
  clean_legacy_install_dirs
  return 0
}

# agnosgram predates this binary-release install path: an npm-era prototype
# installed itself under ~/.local/share/brainstorm-tools/agnosgram-v*. That
# path is dead (Milestone 6's TypeScript retirement) - a stale copy could
# quietly shadow the binary this script just installed. The dirs match a
# pattern only this installer ever created, so they are ours to remove.
# Called only after a verified install succeeded, never as a pre-step; set
# AGNOSGRAM_KEEP_OLD_INSTALLS=1 to keep them.
clean_legacy_install_dirs() {
  local legacy_root="$HOME/.local/share/brainstorm-tools"
  local d
  for d in "$legacy_root"/agnosgram-v*; do
    [ -d "$d" ] || continue
    if [ "${AGNOSGRAM_KEEP_OLD_INSTALLS:-}" = "1" ]; then
      echo "Note: keeping obsolete pre-Rust install dir (AGNOSGRAM_KEEP_OLD_INSTALLS=1): ${d}"
      continue
    fi
    if rm -rf "$d" 2>/dev/null; then
      echo "Removed obsolete pre-Rust agnosgram install dir: ${d}"
    else
      echo "Note: could not remove obsolete install dir ${d} - safe to delete manually." >&2
    fi
  done
  # Drop the shared root once the last tool's dir is gone; rmdir refuses a
  # non-empty dir, so a sibling tool's leftovers keep it alive.
  rmdir "$legacy_root" 2>/dev/null || true
}

print_source_fallback() {
  # agnosgram is not on npm and never will be (Milestone 6 owner decision) -
  # give honest build-from-source steps instead of a fallback that could
  # only ever 404.
  echo "No matching binary release (built for: ${BUILT_PLATFORMS})." >&2
  echo "agnosgram is not on npm; build from source instead (needs a stable Rust toolchain):" >&2
  echo "  git clone https://github.com/${REPO}.git" >&2
  echo "  cd ${BIN_NAME}" >&2
  echo "  cargo build --release --manifest-path rust/Cargo.toml" >&2
  echo "  ./rust/target/release/${BIN_NAME} --help   # run directly, or:" >&2
  echo "  install -m 755 rust/target/release/${BIN_NAME} \"\$HOME/.local/bin/${BIN_NAME}\"" >&2
}

main() {
  local platform cpu tag
  platform="$(os)"
  cpu="$(arch)"
  tag="$(resolve_tag)"

  if [ "$platform" = "unsupported" ] || [ "$cpu" = "unsupported" ]; then
    echo "Unrecognized platform ($(uname -s) $(uname -m); built for: ${BUILT_PLATFORMS})." >&2
    print_source_fallback
    exit 1
  fi

  if ! install_binary "$platform" "$cpu" "$tag"; then
    print_source_fallback
    exit 1
  fi
}

# Only run on direct execution, not when sourced (e.g. to unit-test
# verify_checksum/verify_provenance in isolation against a local fixture
# pair instead of a real download - source this file, then call either
# function directly).
# Run main on direct execution (./install.sh) and when piped (curl | bash),
# where bash has no BASH_SOURCE at all - under `set -u` that unset element
# used to abort the script before main ran. Sourcing (for tests) skips main.
if [ -z "${BASH_SOURCE[0]:-}" ] || [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  main "$@"
fi
