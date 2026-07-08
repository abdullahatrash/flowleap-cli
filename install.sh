#!/bin/sh
set -e

REPO="abdullahatrash/flowleap-cli"
INSTALL_DIR="/usr/local/bin"
BINARY="flowleap"

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)  OS_NAME="darwin" ;;
  Linux)   OS_NAME="linux" ;;
  *)       echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64)  ARCH_NAME="x86_64" ;;
  arm64|aarch64)  ARCH_NAME="aarch64" ;;
  *)              echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

ASSET_NAME="${BINARY}-${OS_NAME}-${ARCH_NAME}"

# Get latest release tag
LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)

if [ -z "$LATEST" ]; then
  echo "Error: Could not determine latest release"
  exit 1
fi

DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${LATEST}/${ASSET_NAME}"
CHECKSUMS_URL="https://github.com/${REPO}/releases/download/${LATEST}/checksums.txt"

echo "Installing ${BINARY} ${LATEST} (${OS_NAME}/${ARCH_NAME})..."

# Download (show a progress bar on TTY, stay quiet when piped/non-interactive)
TMPFILE=$(mktemp)
CHECKSUMS_FILE=$(mktemp)
trap 'rm -f "$TMPFILE" "$CHECKSUMS_FILE"' EXIT
if [ -t 2 ]; then
  CURL_PROGRESS="--progress-bar"
else
  CURL_PROGRESS="-s"
fi
curl -fL -S $CURL_PROGRESS "$DOWNLOAD_URL" -o "$TMPFILE"

# Verify sha256 against the release's published checksums.txt
curl -fsSL "$CHECKSUMS_URL" -o "$CHECKSUMS_FILE"

if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL=$(sha256sum "$TMPFILE" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
  ACTUAL=$(shasum -a 256 "$TMPFILE" | awk '{print $1}')
else
  echo "Error: Neither sha256sum nor shasum found; cannot verify download"
  exit 1
fi

# checksums.txt lines look like "<sha256>  <asset-dir>/<asset>" (or just "<asset>").
EXPECTED=$(awk -v asset="$ASSET_NAME" \
  '$2 == asset || $2 == asset "/" asset { print $1; exit }' "$CHECKSUMS_FILE")

if [ -z "$EXPECTED" ]; then
  echo "Error: No checksum entry for ${ASSET_NAME} in checksums.txt"
  exit 1
fi

if [ "$ACTUAL" != "$EXPECTED" ]; then
  echo "Error: sha256 mismatch for ${ASSET_NAME}"
  echo "  expected: $EXPECTED"
  echo "  actual:   $ACTUAL"
  echo "Refusing to install."
  exit 1
fi

echo "sha256 verified."
chmod +x "$TMPFILE"

# Install
if [ -w "$INSTALL_DIR" ]; then
  mv "$TMPFILE" "${INSTALL_DIR}/${BINARY}"
else
  echo "Need sudo to install to ${INSTALL_DIR}"
  sudo mv "$TMPFILE" "${INSTALL_DIR}/${BINARY}"
fi

echo "Installed ${BINARY} to ${INSTALL_DIR}/${BINARY}"
${BINARY} --version
