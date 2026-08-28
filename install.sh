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
DOWNLOAD_URL="https://github.com/arvinduh/resumake/releases/latest/download/${ASSET_NAME}"
INSTALL_DIR="${RESUMAKE_INSTALL_DIR:-$HOME/.local/bin}"

info "Detected platform: ${TARGET}"
info "Installing resumake into ${INSTALL_DIR}..."

mkdir -p "$INSTALL_DIR"

TEMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TEMP_DIR"
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

info "Extracting binary..."
tar -xzf "$TEMP_DIR/$ASSET_NAME" -C "$TEMP_DIR"
mv "$TEMP_DIR/resumake" "$INSTALL_DIR/resumake"
chmod +x "$INSTALL_DIR/resumake"

success "resumake installed successfully to ${INSTALL_DIR}/resumake"

# Verify PATH
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    warn "${INSTALL_DIR} is not currently in your PATH."
    warn "Add it to your shell configuration (e.g. ~/.bashrc, ~/.zshrc):"
    warn "  export PATH=\"\$PATH:${INSTALL_DIR}\""
    ;;
esac

"$INSTALL_DIR/resumake" --version || true
