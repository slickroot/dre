# 053 - Drop Python packaging entirely

## User Story

As a maintainer who has finished porting the document model, layout, rendering, and terminal writer to Rust, I want `dre_rs` to become the whole project — a native Rust binary with no PyO3 boundary, no `dre/` Python package, and no maturin/pyproject.toml build — so that the project is completely in Rust with nothing left in Python.

## Acceptance Criteria

- The repository root gains its own `Cargo.toml` (package renamed from `dre_rs` to `dre`), `Cargo.lock`, and `src/` (moved from `dre_rs/src/`); the `dre_rs/` directory no longer exists.
- `Cargo.toml` has no `[lib]` section (the crate builds as a plain binary via `src/main.rs`), no `pyo3` dependency (normal or dev-dependency), and no `extension-module`/`default` feature block. `base64` and `flate2` remain, since the Kitty graphics encoding still needs them.
- `src/lib.rs` and its `#[pymodule]` registration function are gone. `src/main.rs` declares `mod state; mod layout; mod render; mod writer;` (moved verbatim from `lib.rs`) and defines `fn main() -> ExitCode`.
- Nothing in the crate carries `#[pyclass]`, `#[pyfunction]`, or `#[pymodule]` anymore — the PyO3 boundary is fully gone.
- `writer.rs`'s former `main` function is renamed `write` and returns `io::Result<ExitCode>`: the normal path returns `Ok(ExitCode::SUCCESS)`, the unsupported-terminal path returns `Ok(ExitCode::FAILURE)` (replacing the old `std::process::exit(1)` call), and real I/O errors propagate as `Err` via `?`.
- `main.rs`'s `fn main() -> ExitCode` calls `writer::write()` and matches its result: `Ok(code) => code`, `Err(e) => { eprintln!("{e}"); ExitCode::FAILURE }`.
- `dre/` (the Python package), `tests/` (the now-empty Python test directory), `pyproject.toml`, `uv.lock`, `.venv/`, and `.pytest_cache/` are all deleted.
- `.gitignore` drops `__pycache__/`, `*.pyc`, and `.venv/`, and changes `dre_rs/target/` to `target/`.
- `flake.nix`'s `devShells.default` drops `pkgs.python312` and `pkgs.uv` from `buildInputs`; `shellHook` no longer runs `uv sync` and only sets `CARGO_TARGET_DIR`.
- `cargo build` from the repository root produces a `dre` binary, and `cargo run` launches the tool directly — nothing in the project involves Python anymore.

## Technical Design

### Scope

This is the final step of the Python-to-Rust migration (specs 043–052). Spec 052 left one PyO3 seam standing: `dre/__main__.py` calling `dre_rs.main()` through a compiled extension module, with `dre_rs/` living as a `maturin`-built sub-crate next to the Python package. This spec removes that seam entirely — `dre_rs` stops being a Python extension and becomes the project itself: a normal Rust binary crate at the repository root, with no Python files, no PyO3 dependency, and no maturin/uv build step anywhere in the tree.

### Crate restructuring

The crate moves from `dre_rs/` (a subdirectory) to the repository root, and from a dual `cdylib`/`rlib` library crate to a plain binary crate:

- `dre_rs/src/*.rs` → `src/*.rs`, `dre_rs/Cargo.toml` → `Cargo.toml`, `dre_rs/Cargo.lock` → `Cargo.lock`. The `dre_rs/` directory is deleted once the move is done.
- `Cargo.toml`'s `[package] name` changes from `dre_rs` to `dre`. The `[lib]` table (`crate-type = ["cdylib", "rlib"]`) is removed — Cargo's default binary convention (`src/main.rs` as the entry point) applies instead. The `pyo3` dependency and dev-dependency, and the `extension-module`/`default` feature declarations, are deleted. `base64` and `flate2` are untouched.
- `src/lib.rs` is deleted. Its module declarations (`mod state; mod layout; mod render; mod writer;`) move into a new `src/main.rs`, and its `#[pymodule]` registration function is deleted outright rather than moved — there is no module to register anymore.

### Entry point and exit codes

`writer::main` (the sole surviving `#[pyfunction]` from spec 052) is renamed `writer::write` and changes signature from an implicit PyO3-wrapped `()` return to:

```rust
pub fn write() -> io::Result<ExitCode>
```

Its unsupported-Kitty-graphics path, which previously wrote the clear-line/message output and called `std::process::exit(1)` directly, instead returns `Ok(ExitCode::FAILURE)` after writing that same output. Its normal path returns `Ok(ExitCode::SUCCESS)`. Any genuine I/O failure (e.g. a broken stdout) propagates as `Err` via `?`, rather than being swallowed or force-exited.

`src/main.rs` becomes the real process entry point:

```rust
fn main() -> ExitCode {
    match writer::write() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
```

This keeps `write()` itself free of direct process-exit side effects, so it stays testable the same way spec 052 already made `paint`/`frame` testable (generic over `io::Write`, no hidden `exit()` calls) — `#[test]`s can call `write()` and assert on its returned `ExitCode`/`Err` instead of needing to fork a subprocess.

### Deletions

Once the crate move is complete and building, the following are deleted in one pass: `dre/` (the Python package — `__init__.py` was already empty, `__main__.py` was the last PyO3 call site), `tests/` (already emptied of test files by spec 052, now removed as a directory), `pyproject.toml`, `uv.lock`, `.venv/`, and `.pytest_cache/`.

### Dev environment

`flake.nix`'s `devShells.default.buildInputs` drops `pkgs.python312` and `pkgs.uv`, leaving only the Rust toolchain (`rustToolchain`). `shellHook` drops the `uv sync` line, keeping only the `CARGO_TARGET_DIR` export (updated implicitly since the crate no longer lives under `dre_rs/`, but the export mechanism itself is unchanged).

### `.gitignore`

`__pycache__/`, `*.pyc`, and `.venv/` are removed (nothing in the tree produces them anymore). `dre_rs/target/` becomes `target/`, matching the crate's new root-level location.

