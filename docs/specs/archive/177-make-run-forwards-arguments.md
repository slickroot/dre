# Make run forwards arguments

This is a technical spec, not a user story: it has no user-facing behaviour.

## Problem

Testing the current code means typing `nix develop --command cargo run -- diagram.dre` by hand. The Makefile has no target that runs the debug build of `dre`.

## Acceptance Criteria

- `make run` builds and runs the debug `dre`.
- `make run ARGS=diagram.dre` runs it with `diagram.dre` as the file to open.
- Everything in `ARGS` is forwarded to `dre`, so `make run ARGS=serve` works too.
- Every other target behaves as before.

## Technical Design

Decisions:

- **Variable, not extra goal.** Arguments come in through `ARGS`, the idiomatic way to pass values to Make. There is no catch-all rule and no `$(MAKECMDGOALS)` parsing.
- **Forward everything.** `ARGS` is passed to `dre` as is.
- **Debug profile.** `cargo run` builds and runs in one step and defaults to debug, so there is no separate build dependency.
- **Same wrapper as the other targets.** The recipe goes through `nix develop --command`.

Change, in `Makefile` only: add `run` to `.PHONY` and add one recipe.

```make
run:
	nix develop --command cargo run -- $(ARGS)
```
