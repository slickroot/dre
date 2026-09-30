worktrees should be in directory `.claude/worktrees`

never run the app yourself to test it.

this project uses `nix`

use the `Makefile` for common tasks: `make build`, `make test`, `make fmt`, `make clippy`, and `make install` (installs a debug build to `~/.local/bin/dre`)

## adding a spec

add specs only on `main`, only via `scripts/new-spec`. never choose a spec number by hand.

`scripts/new-spec <slug> < body.md`
