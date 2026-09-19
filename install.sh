#!/usr/bin/env bash
set -euo pipefail

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)                asset=dre-macos-arm64 ;;
  Darwin-x86_64)               asset=dre-macos-x86_64 ;;
  Linux-x86_64)                asset=dre-linux-x86_64 ;;
  Linux-aarch64 | Linux-arm64) asset=dre-linux-arm64 ;;
  *)
    echo "dre has no build for $(uname -s) $(uname -m)." >&2
    exit 1
    ;;
esac

URL="https://github.com/slickroot/dre/releases/latest/download/$asset"

DEST="$HOME/.local/bin/dre"
mkdir -p "$(dirname "$DEST")"

tmp="$(mktemp)"
curl -fsSL "$URL" -o "$tmp"
chmod +x "$tmp"

if [ -e "$DEST" ]; then
  echo "Replacing existing dre"
fi
mv "$tmp" "$DEST"

echo "dre installed to ~/.local/bin/dre"

case ":$PATH:" in
  *":$HOME/.local/bin:"*) ;;
  *)
    case "${SHELL##*/}" in
      zsh)  config='~/.zshrc'; line='export PATH="$HOME/.local/bin:$PATH"' ;;
      bash) [ "$(uname -s)" = Darwin ] && config='~/.bash_profile' || config='~/.bashrc'; line='export PATH="$HOME/.local/bin:$PATH"' ;;
      fish) config='~/.config/fish/config.fish'; line='fish_add_path $HOME/.local/bin' ;;
      *)    config='~/.bash_profile'; line='export PATH="$HOME/.local/bin:$PATH"' ;;
    esac
    echo "Warning: ~/.local/bin is not on your PATH."
    echo "Add this line to $config:"
    echo "$line"
    ;;
esac