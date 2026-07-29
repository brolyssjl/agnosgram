#!/usr/bin/env bash
# Install `agnosgram` without a local npm install.
#
# Primary path: fetch the single-file binary for this platform from the
# latest GitHub release and place it on PATH. Falls back to `npm install -g`
# when Node is available and no matching binary asset exists yet.
#
# Built platforms (keep in sync with .github/workflows/release.yml's
# build-binaries matrix - update both together when adding a platform):
# linux-x64, darwin-arm64.
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

install_binary() {
  local platform="$1" cpu="$2"
  local asset="${BIN_NAME}-${platform}-${cpu}"
  # /releases/latest/download/<asset> redirects straight to the current
  # release's asset - no need to resolve the tag via the (rate-limited)
  # api.github.com first.
  local url="https://github.com/${REPO}/releases/latest/download/${asset}"
  local tmp
  tmp="$(mktemp "${TMPDIR:-/tmp}/${BIN_NAME}.XXXXXX")"

  echo "Downloading ${asset}..."
  mkdir -p "$INSTALL_DIR"
  # Download to a temp file first: a mid-transfer failure must never leave a
  # truncated (but still +x, still shadowing-the-fallback) binary in place.
  if ! curl -fsSL "$url" -o "$tmp"; then
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
  return 0
}

install_via_npm() {
  if ! command -v npm >/dev/null 2>&1; then
    echo "No matching binary release (built for: ${BUILT_PLATFORMS}) and no npm found." >&2
    echo "Install Node >= 20, then: npm install -g agnosgram" >&2
    exit 1
  fi
  echo "No matching binary release yet (built for: ${BUILT_PLATFORMS}) - trying npm instead." >&2
  echo "Note: agnosgram is not published to npm yet - this will fail (404) until it is." >&2
  npm install -g agnosgram
}

main() {
  local platform cpu
  platform="$(os)"
  cpu="$(arch)"

  if [ "$platform" = "unsupported" ] || [ "$cpu" = "unsupported" ]; then
    echo "Unrecognized platform ($(uname -s) $(uname -m); built for: ${BUILT_PLATFORMS}) - falling back to npm." >&2
    install_via_npm
    return
  fi

  if ! install_binary "$platform" "$cpu"; then
    install_via_npm
  fi
}

main "$@"
