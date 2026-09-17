#!/usr/bin/env bash
set -euo pipefail

VERSION=v0.1.0-dev
URL="https://github.com/slickroot/dre/releases/download/$VERSION/dre"

DEST="$HOME/.local/bin/dre"
mkdir -p "$(dirname "$DEST")"

tmp="$(mktemp)"
curl -fsSL "$URL" -o "$tmp"
chmod +x "$tmp"

if [ -e "$DEST" ]; then
  echo "Replacing existing dre"
fi
mv "$tmp" "$DEST"

echo "dre $VERSION installed to ~/.local/bin/dre"