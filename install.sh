#!/usr/bin/env bash
set -e

REPO="AssafWoo/homebrew-ccr"
INSTALL_DIR="${CCR_INSTALL_DIR:-$HOME/.local/bin}"

# Detect platform
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
  Darwin)
    case "$ARCH" in
      arm64)  ASSET="ccr-macos-arm64" ;;
      x86_64) ASSET="ccr-macos-x86_64" ;;
      *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  Linux)
    case "$ARCH" in
      x86_64) ASSET="ccr-linux-x86_64" ;;
      *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

# Resolve latest release tag
echo "Fetching latest CCR release..."
TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
  | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

if [ -z "$TAG" ]; then
  echo "Could not determine latest release tag." >&2
  exit 1
fi

URL="https://github.com/$REPO/releases/download/$TAG/$ASSET"
echo "Downloading CCR $TAG for $OS/$ARCH..."

mkdir -p "$INSTALL_DIR"
curl -fsSL "$URL" -o "$INSTALL_DIR/ccr"
chmod +x "$INSTALL_DIR/ccr"

# Ensure install dir is on PATH
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
  echo ""
  echo "Add $INSTALL_DIR to your PATH:"
  echo '  export PATH="$HOME/.local/bin:$PATH"'
  echo ""
fi

echo "CCR $TAG installed to $INSTALL_DIR/ccr"
echo ""

# Register Claude Code hooks
if command -v ccr &>/dev/null || [ -x "$INSTALL_DIR/ccr" ]; then
  "$INSTALL_DIR/ccr" init && echo "Claude Code hooks registered."
else
  echo "Run 'ccr init' to register Claude Code hooks."
fi
