# 064 - Install dre with a one-line command

## Story

Bob has an Apple Silicon Mac. He runs
`curl -fsSL https://raw.githubusercontent.com/slickroot/dre/main/install.sh | bash`,
and `dre` is installed and ready to run from his terminal.

## Acceptance Criteria

- Running
  `curl -fsSL https://raw.githubusercontent.com/slickroot/dre/main/install.sh | bash`
  on an Apple Silicon Mac installs `dre` to `~/.local/bin/dre`, with no `sudo`.
- It installs the newest release, pre-releases included (today that's
  `v0.1.0-dev`).
- When the install works, the script prints a confirmation, e.g.
  `dre v0.1.0-dev installed to ~/.local/bin/dre`.
- If `~/.local/bin` isn't on Bob's PATH, the script says so and prints the
  exact line to add to his shell config.
- After that, typing `dre` in his terminal runs it.

## Technical Design

### Overview

A single POSIX shell script `install.sh` at the repo root (served from
`https://raw.githubusercontent.com/slickroot/dre/main/install.sh`). It installs
the `dre` binary to `~/.local/bin/dre` with no `sudo`.

### Download

- **Stopgap:** hardcode the exact pre-release download URL
  `https://github.com/slickroot/dre/releases/download/v0.1.0-dev/dre`.
- GitHub's "latest release" concept excludes pre-releases, and all of this
  project's releases are pre-releases, so `releases/latest/download/dre` would
  not resolve. Once a stable release exists, revisit switching to
  `releases/latest/download/dre` (no `jq`, no GitHub API parsing).
- Version for the confirmation message is also hardcoded in the script:
  `VERSION=v0.1.0-dev`.

### Responsibilities of the script

1. **Resolve target dir** — `DEST="$HOME/.local/bin/dre"`; `mkdir -p` the parent
   dir if it doesn't exist.
2. **Download** — `curl -fsSL <URL> -o <tempfile>` (use `mktemp`).
3. **Install** — `chmod +x <tempfile>`, then `mv` into `$DEST`. Download-then-move
   protects against a partial binary landing in place.
4. **Overwrite existing** — if `$DEST` already exists, replace it and print a
   message (e.g. `Replacing existing dre`).
5. **Confirm** — print `dre v0.1.0-dev installed to ~/.local/bin/dre`.
6. **PATH check** — if `$HOME/.local/bin` is not on `$PATH`, print a warning with
   the exact line to add, tailored to the user's shell:
   - zsh → `~/.zshrc`: `export PATH="$HOME/.local/bin:$PATH"`
   - bash → `~/.bash_profile`: `export PATH="$HOME/.local/bin:$PATH"`
   - fish → `~/.config/fish/config.fish`: `fish_add_path $HOME/.local/bin`

### Error handling

- Script starts with `set -euo pipefail`.
- Trust `curl`'s exit code — no extra file sanity checks.
- Any failure exits non-zero with the underlying error message.

### Collaborators

- GitHub Releases (raw.githubusercontent.com for the script, github.com for the
  binary asset).
- No runtime dependencies beyond `bash`, `curl`, `mkdir`, `mktemp`, `mv`, `chmod`.
