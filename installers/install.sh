#!/bin/sh
# Thin wrapper redirecting to cargo-dist generated resumake-installer.sh
# https://github.com/arvinduh/resumake
set -eu

VERSION="${RESUMAKE_VERSION:-latest}"
if [ "$VERSION" = "latest" ]; then
  URL="https://github.com/arvinduh/resumake/releases/latest/download/resumake-installer.sh"
else
  URL="https://github.com/arvinduh/resumake/releases/download/v${VERSION#v}/resumake-installer.sh"
fi

curl --proto '=https' --tlsv1.2 -LsSf "$URL" | sh

