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

case ":$PATH:" in
  *":$HOME/.local/bin:"*) ;;
  *)
    case "${SHELL##*/}" in
      zsh)  config=~/.zshrc; line='export PATH="$HOME/.local/bin:$PATH"' ;;
      bash) config=~/.bash_profile; line='export PATH="$HOME/.local/bin:$PATH"' ;;
      fish) config=~/.config/fish/config.fish; line='fish_add_path $HOME/.local/bin' ;;
      *)    config=~/.bash_profile; line='export PATH="$HOME/.local/bin:$PATH"' ;;
    esac
    echo "Warning: ~/.local/bin is not on your PATH."
    echo "Add this line to $config:"
    echo "$line"
    ;;
esac