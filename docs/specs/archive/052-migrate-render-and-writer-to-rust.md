# Migrate render.py and writer.py fully to Rust

## User Story

As a maintainer learning Rust, I want `dre/render.py` and `dre/writer.py` fully replaced by `dre_rs`, so that the sprite-rendering pipeline and the terminal/process wiring live in Rust the same way the document model, layout, and Kitty graphics protocol already do, and `dre/render.py` and `dre/writer.py` can be deleted.

## Acceptance Criteria

- `dre/render.py` and `dre/writer.py` are deleted.
- `dre/__main__.py` becomes a thin entrypoint: `from dre_rs import main` followed by `main()` under `if __name__ == "__main__":` — matching the existing pattern used for every other Rust-side entry point.
- `tests/test_render.py`, `tests/test_writer.py`, and `tests/test_kitty.py` are deleted. `dre_rs`'s own `#[test]` suite is the only test coverage for this code going forward, leaving `tests/` with no test files.
- `dre_rs` exposes exactly one `#[pyfunction]`: `main`. Every other type and function this migration touches — `State`, `Box`, `Label`, `Arrow`, `Cursor`, `Placement`, `handle_key`, `layout`, `with_cursor`, `KittyGraphics`, and the new `Sprite`/`TerminalRenderer` — loses its `#[pyclass]`/`#[pyfunction]` status and becomes a plain internal Rust type or function, since nothing outside `main` ever crosses the PyO3 boundary anymore.
- The `Renderer` and `GraphicsProtocol` `typing.Protocol`s disappear entirely; nothing else implemented them.
- `TerminalRenderer` holds its `KittyGraphics` collaborator as a plain native Rust field (real composition, not a trait object) — there is only ever one implementation and no Python-side substitution point anymore.
- `KittyGraphics::draw` takes `Vec<Sprite>` (a concrete internal type) directly and returns a plain `String`, dropping the `getattr`/`extract`-based duck typing and the `PyResult` error handling that existed only because `Sprite` used to cross the FFI boundary as a Python dataclass.
- All of `writer.py`'s terminal control (`termios`/`tty`/`fcntl`/`select`/`struct`) is rebuilt using the `nix` crate (added as a new dependency in `Cargo.toml`) instead of raw `libc` calls, since `nix` covers termios, `ioctl(TIOCGWINSZ)`, and `select` in one safe, typed dependency.
- `terminal_session`'s save/set-raw/restore behavior is rebuilt as an RAII guard (`RawModeGuard`) whose `Drop` impl restores the saved termios state, replacing the Python context-manager/`finally` pattern with Rust's ownership-based cleanup.
- `paint`, `frame`, and any equivalent helpers stay generic over `io::Write`/`io::Read` (e.g. `fn paint<W: Write>(stream: &mut W, lines: &[String])`) so Rust tests can substitute an in-memory buffer, mirroring the role the Python fakes (`Stream`, `StubRenderer`) used to play. `run`/`main` themselves construct the real stdio handles and are not required to be generic.
- `main`'s unsupported-terminal path calls `std::process::exit(1)` directly after writing the same clear-line-and-message output as the Python version, matching Python's `sys.exit(1)` behavior exactly.
- The new code lives in two new submodules, declared via `mod render;` and `mod writer;` in `lib.rs`: `dre_rs/src/render.rs` (`Sprite`, `TerminalRenderer`, `RoundedBox`, `Canvas`, and all pixel/colour math) and `dre_rs/src/writer.rs` (`RawModeGuard`, `cell_size`, `supports_kitty_graphics`, `paint`, `frame`, `run`, `main`). Existing `State`/`Box`/`KittyGraphics`/`handle_key` code already in `lib.rs` is left wherever it currently sits, only losing its `#[pyclass]`/`#[pyfunction]` attributes and its `#[pymodule]` registration.

## Technical Design

### Scope

This migrates all of `dre/render.py` (the `Sprite`/`TerminalRenderer`/`RoundedBox`/`Canvas` pipeline and its module-level pixel/colour helpers) and all of `dre/writer.py` (terminal control, `frame`, `run`, `main`) into `dre_rs`, deleting both Python files. Because nothing in Python calls into `dre_rs` afterward except the one-line `dre/__main__.py` entrypoint (`from dre_rs import main; main()`), this migration also strips `#[pyclass]`/`#[pyfunction]` status from every type and function introduced by specs 043–051 (`State`, `Box`, `Label`, `Arrow`, `Cursor`, `Placement`, `handle_key`, `layout`, `with_cursor`, `KittyGraphics`) as well as the new code — `main` becomes the sole symbol the `#[pymodule]` registration function exports.

### Module layout

Two new submodules, following spec 051's precedent of splitting `lib.rs` rather than letting it keep growing:

- `dre_rs/src/render.rs`: `Sprite`, `TerminalRenderer`, `RoundedBox`, `Canvas`, and the module-level helpers (`_square_pixels`, `_body_row`, `_key`, `_colour`, `_fill_colour`, `_cell` equivalents). `TerminalRenderer::render` matches on the `PlacementNode` enum (already defined in `layout.rs`) directly instead of Python's `isinstance` checks.
- `dre_rs/src/writer.rs`: `RawModeGuard`, `cell_size`, `supports_kitty_graphics`, `paint`, `frame`, `run`, `main`.

`lib.rs` gains `mod render;` and `mod writer;`, drops its `#[pyclass]`/`#[pyfunction]` attributes on existing types, and its `#[pymodule]` function shrinks to registering only `main`.

### `KittyGraphics::draw`

Changes from:

```rust
fn draw(&self, sprites: Vec<Bound<'_, PyAny>>) -> PyResult<String>
```

to:

```rust
fn draw(&self, sprites: Vec<Sprite>) -> String
```

since `Sprite` is now a concrete Rust type on both sides of the call, not a Python object reached via `getattr`.

### `TerminalRenderer` composition

`TerminalRenderer` owns its `KittyGraphics` collaborator as a plain field:

```rust
struct TerminalRenderer {
    graphics: KittyGraphics,
    cell_width: i64,
    cell_height: i64,
    cache: HashMap<Key, Sprite>,
}
```

No trait/dynamic dispatch — there is exactly one `GraphicsProtocol` implementation and no Python-side substitution point to preserve.

### Terminal control via `nix`

`Cargo.toml` gains `nix` (with the `term`/`ioctl`/`fs` feature set needed for `termios`, `TIOCGWINSZ`, and `select`). `writer.rs` rebuilds:

- `terminal_session` → `RawModeGuard::new(fd)` saves `Termios::from_fd`, calls `cfmakeraw`+`tcsetattr` to enter raw mode; `impl Drop for RawModeGuard` restores the saved `Termios` via `tcsetattr`.
- `cell_size` → `nix`'s typed `ioctl!` / `Winsize` equivalent for `TIOCGWINSZ`, replacing `struct.pack`/`unpack`.
- `supports_kitty_graphics` → `nix::sys::select::select` with a timeout matching `KITTY_GRAPHICS_REPLY_TIMEOUT`, then `nix::unistd::read`.

### Testability without Python fakes

```rust
fn paint<W: Write>(stream: &mut W, lines: &[String]) -> io::Result<()>
fn frame<W: Write>(state: &State, renderer: &mut TerminalRenderer, stream: &mut W, cols: i64, rows: i64) -> io::Result<()>
```

Rust `#[test]`s pass a `Vec<u8>` (or `Cursor<Vec<u8>>`) in place of the Python `Stream` fake to assert on written bytes, and a stub/minimal `TerminalRenderer` setup in place of `StubRenderer`. `run`/`main` are the concrete wiring layer — they construct real stdio and a real `TerminalRenderer`/`KittyGraphics`, and are not themselves required to be generic or unit-tested beyond the process-exit behavior on unsupported terminals.

### Exit behavior

`main` matches Python's `sys.exit(1)` path exactly: on unsupported Kitty graphics, it writes `CLEAR_LINE` and the not-supported message to real stdout, then calls `std::process::exit(1)` directly rather than returning an error through PyO3.
