#!/bin/sh
# resumake installer for Linux and macOS
# https://github.com/arvinduh/resumake

set -eu

# Color formatting helpers (disabled if not terminal or NO_COLOR set)
if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
  BOLD="\033[1m"
  GREEN="\033[1;32m"
  BLUE="\033[1;34m"
  YELLOW="\033[1;33m"
  RED="\033[1;31m"
  RESET="\033[0m"
else
  BOLD=""
  GREEN=""
  BLUE=""
  YELLOW=""
  RED=""
  RESET=""
fi

info() {
  printf "${BLUE}info:${RESET} %s\n" "$*"
}

success() {
  printf "${GREEN}success:${RESET} %s\n" "$*"
}

warn() {
  printf "${YELLOW}warning:${RESET} %s\n" "$*"
}

error() {
  printf "${RED}error:${RESET} %s\n" "$*" >&2
  exit 1
}

# Detect operating system
OS="$(uname -s)"
case "$OS" in
  Linux)
    PLATFORM="unknown-linux-gnu"
    ;;
  Darwin)
    PLATFORM="apple-darwin"
    ;;
  *)
    error "Unsupported operating system '$OS'. install.sh supports Linux and macOS. For Windows, see install.ps1."
    ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64 | amd64)
    ARCH="x86_64"
    ;;
  aarch64 | arm64)
    ARCH="aarch64"
    ;;
  *)
    error "Unsupported architecture '$ARCH'. install.sh supports x86_64 and aarch64 (ARM64)."
    ;;
esac

TARGET="${ARCH}-${PLATFORM}"
ASSET_NAME="resumake-${TARGET}.tar.gz"
VERSION="${RESUMAKE_VERSION:-latest}"
if [ "$VERSION" = "latest" ]; then
  DOWNLOAD_URL="https://github.com/arvinduh/resumake/releases/latest/download/${ASSET_NAME}"
else
  DOWNLOAD_URL="https://github.com/arvinduh/resumake/releases/download/v${VERSION#v}/${ASSET_NAME}"
fi
INSTALL_DIR="${RESUMAKE_INSTALL_DIR:-$HOME/.local/bin}"

info "Detected platform: ${TARGET}"
if [ "$VERSION" != "latest" ]; then
  info "Requested version: v${VERSION#v}"
fi
info "Installing rsmk into ${INSTALL_DIR}..."

mkdir -p "$INSTALL_DIR"

TEMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TEMP_DIR"
  rm -f "$INSTALL_DIR/.rsmk.tmp.$$"
}
trap cleanup EXIT

info "Downloading ${DOWNLOAD_URL}..."
if command -v curl > /dev/null 2>&1; then
  curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_DIR/$ASSET_NAME"
elif command -v wget > /dev/null 2>&1; then
  wget -qO "$TEMP_DIR/$ASSET_NAME" "$DOWNLOAD_URL"
else
  error "Neither curl nor wget found. Please install either tool and retry."
fi

info "Verifying checksum..."
if command -v sha256sum > /dev/null 2>&1; then
  CHECK="sha256sum --check --quiet"
elif command -v shasum > /dev/null 2>&1; then
  CHECK="shasum -a 256 --check --quiet" # macOS has no sha256sum
else
  CHECK=""
  warn "Neither sha256sum nor shasum found; skipping checksum verification."
fi

if [ -n "$CHECK" ]; then
  if command -v curl > /dev/null 2>&1; then
    curl -fsSL "$DOWNLOAD_URL.sha256" -o "$TEMP_DIR/$ASSET_NAME.sha256"
  else
    wget -qO "$TEMP_DIR/$ASSET_NAME.sha256" "$DOWNLOAD_URL.sha256"
  fi
  (cd "$TEMP_DIR" && $CHECK "$ASSET_NAME.sha256") ||
    error "Checksum verification failed for ${ASSET_NAME}. Refusing to install."
  success "Checksum verified."
fi

info "Extracting binary..."
tar -xzf "$TEMP_DIR/$ASSET_NAME" -C "$TEMP_DIR"
cp "$TEMP_DIR/rsmk" "$INSTALL_DIR/.rsmk.tmp.$$"
chmod 755 "$INSTALL_DIR/.rsmk.tmp.$$"
mv -f "$INSTALL_DIR/.rsmk.tmp.$$" "$INSTALL_DIR/rsmk"

success "rsmk installed successfully to ${INSTALL_DIR}/rsmk"

# Verify PATH
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    warn "${INSTALL_DIR} is not currently in your PATH."
    warn "Add it to your shell configuration (e.g. ~/.bashrc, ~/.zshrc):"
    warn "  export PATH=\"\$PATH:${INSTALL_DIR}\""
    ;;
esac

"$INSTALL_DIR/rsmk" --version || true
