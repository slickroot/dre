# 047 - Nix-managed development environment

## User Story

As a maintainer, I want this project's development environment (Python and Rust toolchains) provided by Nix instead of relying on whatever's on my system PATH, so that `nix develop` alone gives me everything needed to run the tests and build `dre_rs`, and so that a fresh git worktree doesn't force a full Rust rebuild from scratch.

## Acceptance Criteria

- A `flake.nix` (+ committed `flake.lock`) at the repo root defines a single `devShells.default`, built with `flake-utils.lib.eachDefaultSystem` so it works on `x86_64-linux`, `aarch64-linux`, `x86_64-darwin`, and `aarch64-darwin`.
- `nix develop` alone puts Python 3.12, `uv`, and a full stable Rust toolchain (`rustc`, `cargo`, `clippy`, `rustfmt`, via `rust-overlay`'s `default` profile, tracking latest stable) on `PATH` — no reliance on system Python or a system Rust install.
- The `nixpkgs` input tracks `nixpkgs-unstable`.
- A minimal `pyproject.toml` declares `maturin` and `pytest` as dependencies, with a committed `uv.lock`.
- The shell's `shellHook` runs `uv sync` to create/update `.venv` against the Nix-provided Python 3.12, and exports `CARGO_TARGET_DIR=$HOME/.cache/sketch-cargo-target` so every git worktree shares one Rust build cache instead of recompiling `dre_rs`'s dependency tree from scratch.
- Building `dre_rs` (`maturin develop`) and running the Python test suite (`pytest`) both remain manual steps a developer runs inside `nix develop` — the hook does not auto-build Rust on every shell entry.
- This is dev-environment-only: no Nix package/derivation builds or distributes `dre` or `dre_rs` for end users.

## Technical Design

### Scope

This is a dev-shell-only change: `nix develop` provisions tools on `PATH`, nothing more. Packaging/building the app itself as a Nix derivation (`nix build`, via `crane`/`naersk` for `dre_rs` and a Python builder for `dre`) is explicitly out of scope and left for a future spec if ever needed.

### Toolchain versions

- **Python 3.12** — new enough to be worth using, but still within what `pyo3 = "0.22"` (pinned in `dre_rs/Cargo.toml`) supports. CPython 3.13 support only landed in PyO3 0.23, so jumping to 3.13/3.14 now would force an unrelated PyO3 upgrade.
- **Rust: latest stable**, via the [`rust-overlay`](https://github.com/oxalica/rust-overlay) flake input and `rust-bin.stable.latest.default`. Plain nixpkgs `rustc`/`cargo` lags upstream stable by weeks and is skipped in favour of the overlay. The `default` profile pulls in `clippy` and `rustfmt` alongside `rustc`/`cargo`.

### Flake inputs

```nix
inputs = {
  nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  flake-utils.url = "github:numtide/flake-utils";
  rust-overlay = {
    url = "github:oxalica/rust-overlay";
    inputs.nixpkgs.follows = "nixpkgs";
  };
};
```

`flake-utils.lib.eachDefaultSystem` wraps the `outputs` function so `devShells.default` is generated for Linux and macOS, both `x86_64` and `aarch64`, without hand-listing systems.

### Python dependency management: `uv` + `pyproject.toml`

A minimal `pyproject.toml` is added at the repo root declaring `maturin` and `pytest` as dependencies (there is currently no packaging metadata at all — the test suite just runs against `dre/` on a bare interpreter). `uv` manages a `.venv` and a committed `uv.lock` from that file, giving reproducible, pinned Python dependency versions on top of Nix's reproducible toolchain versions.

Nix's Python 3.12 package lives in the read-only `/nix/store`, so `maturin develop` (which installs the built extension into a site-packages directory) needs a writable target — `uv`'s `.venv` provides that. The `shellHook` runs `uv sync` on every `nix develop` invocation to keep `.venv` in sync with `pyproject.toml`/`uv.lock` against the Nix-provided interpreter; it does not itself invoke `maturin develop` or `pytest` — building the Rust extension and running tests stay explicit developer commands.

### Shared Cargo build cache across worktrees

`.claude/worktrees/<n>/` checkouts each currently get their own gitignored `dre_rs/target/`, so every new worktree recompiles `pyo3`, `base64`, `flate2`, and their transitive dependency trees from zero. The `shellHook` exports:

```
CARGO_TARGET_DIR=$HOME/.cache/sketch-cargo-target
```

All worktrees of this repo then build into the same target directory. Cargo's own per-crate file locking makes concurrent builds from different worktrees into one target dir safe, so only code that actually changed gets recompiled, regardless of which worktree triggered the build. (Cargo's registry/download cache at `~/.cargo/registry` is already process-global and unaffected by worktrees, so no change is needed there.)

### File layout

- `flake.nix`, `flake.lock` — repo root, committed.
- `pyproject.toml`, `uv.lock` — repo root, committed.
- `.venv/`, `dre_rs/target/` — stay gitignored (target is now largely empty/unused locally since builds redirect to `$HOME/.cache/sketch-cargo-target`, but is left in `.gitignore` in case `CARGO_TARGET_DIR` is ever unset).

