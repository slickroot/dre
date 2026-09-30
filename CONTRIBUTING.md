worktrees should be in directory `.claude/worktrees`

never run the app yourself to test it.

this project uses `nix`

use the `Makefile` for common tasks: `make build`, `make test`, `make fmt`, `make clippy`, and `make install` (installs a debug build to `~/.local/bin/dre`)

## adding a spec

add specs only on `main`, only via `scripts/new-spec`. never choose a spec number by hand.

`scripts/new-spec <slug> < body.md`

## refreshing the README assets

the screenshots live in `docs/assets`. keystrokes to reproduce each one:

- `first-box.png`: on an empty canvas press `b`, type `API Gateway`, press `Esc`. one selected box.
- `selection.png`: `b`, type `API Gateway`, `Enter`, type `Auth`, `Esc`; `s`, type `Orders`, `Esc`; `s`, type `Payments`, `Esc`; on `Orders` press `b`, type `Postgres`, `Esc`, then `h` so the brackets sit on `Orders`.
- `styling.png`: same tree as `selection.png`. select the root and press `c` then `r`; on the children press `C`; on `Postgres` press `c` until violet; finish with the selection on `Orders`.
