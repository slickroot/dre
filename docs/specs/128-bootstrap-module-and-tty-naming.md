# Bootstrap module and tty naming

This is a technical spec, not a user story: it has no user-facing behaviour.

## Problem

The editor module is almost finished, but two problems are left over from spec 127.

`editor::open` does all the setup work: it checks for kitty, measures the window, builds the glyph cache and the `TerminalRenderer`, enters raw mode, installs the resize pipe, builds every collaborator and runs the `Editor`. So the editor module imports `kitty`, `terminal` and `render`, even though `Editor` itself only needs its traits.

The word "terminal" means four different things:

- `crate::terminal`: OS tty plumbing (termios, `poll_read`, the resize pipe, `probe`).
- `terminal::Terminal`: not a terminal at all, but `cols`, `rows`, `cell_width` and `cell_height`. The renderer's own tests already call it `window`.
- `crate::render::terminal` / `TerminalRenderer`: the renderer that sits alongside `SvgRenderer`.
- `TerminalKeySource` / `TerminalScreen`: the editor's real implementations.

The word "screen" means three different things: the editor's `Screen` trait, the `terminal::RawScreen` raw-mode guard, and a private `Screen` frame buffer in `render/terminal.rs`.

## Acceptance Criteria

- A new `bootstrap` module owns the setup. `lib::run` calls `bootstrap::run(file)`. `editor::open` no longer exists.
- `editor/mod.rs` contains only `Editor` and its tests. The editor module no longer imports `kitty`, `tty` or `render::TerminalRenderer` outside `controller/screen.rs` and `controller/key_source.rs`.
- `crate::terminal` is renamed to `crate::tty`.
- `Terminal` is renamed to `Window`.
- `RawScreen` is renamed to `tty::RawMode`, and `RawScreen::open` to `RawMode::enter`.
- The private `Screen` in `render/terminal.rs` is renamed to `Frame`.
- `TerminalKeySource` is renamed to `TtyKeySource`. `TerminalScreen`, `TerminalRenderer` and the editor's `Screen` trait keep their names.
- After the change, "Terminal" appears only on the renderer side (`TerminalRenderer`, `TerminalScreen`) and "Tty" only on the OS side.
- Behaviour is unchanged. The setup steps run in the same order as today.

## Technical Design

Decided in the design session.

### `bootstrap::run`

```rust
// src/bootstrap.rs
pub(crate) fn run(file: Option<String>) -> io::Result<ExitCode> {
    let mut stdout = io::stdout();
    let fd = io::stdin().as_raw_fd();
    kitty::require(&mut stdout, fd)?;
    let window = tty::probe()?;
    let glyph_source = Box::new(GlyphCache::new(window.cell_width, window.cell_height));
    let renderer = TerminalRenderer::new(window, glyph_source, CACHE_LIMIT);
    let _raw = tty::RawMode::enter(fd)?;
    let resize_fd = tty::install_resize_pipe()?;

    let store = FileStateStore::new(Box::new(DiskFiles));
    let controller = DreController::new(
        Box::new(TtyKeySource { fd, resize_fd }),
        Box::new(TerminalScreen { renderer, out: stdout }),
        Box::new(StateReducer),
    );
    Editor::new(Box::new(store), Box::new(controller)).run(file.as_deref())?;
    Ok(ExitCode::SUCCESS)
}
```

- Bootstrap is the composition root. It is the only place that builds real implementations.
- It runs the editor itself instead of returning one. The `RawMode` guard has to live until `Editor::run` returns, because dropping it restores the tty, and keeping it inside `run` means no caller has to know about that lifetime.
- Bootstrap is untested wiring, just as `editor::open` was. It has no branches.
- `lib.rs` declares `mod bootstrap` behind `#[cfg(not(target_arch = "wasm32"))]`, the same as `editor`, and matches `cli::Command::Edit(file) => bootstrap::run(file)`.
- Bootstrap may import `editor`'s items, so the editor module makes these visible to the crate: `Editor`, `DreController`, `FileStateStore`, `DiskFiles`, `TtyKeySource`, `TerminalScreen` and `StateReducer`. Dependencies still go one way: `bootstrap` → `editor`, `tty`, `kitty`, `render`. Nothing imports `bootstrap`.

### Renames

| Before | After |
| --- | --- |
| `src/terminal.rs` (`crate::terminal`) | `src/tty.rs` (`crate::tty`) |
| `terminal::Terminal` | `tty::Window` |
| `terminal::RawScreen`, `RawScreen::open` | `tty::RawMode`, `RawMode::enter` |
| `terminal::RESIZE`, `probe`, `poll_read`, `install_resize_pipe` | same names under `tty::` |
| private `Screen` in `render/terminal.rs` | `Frame` |
| `TerminalKeySource` (`editor/controller/key_source.rs`) | `TtyKeySource` |

These stay the same: `render::terminal`, `TerminalRenderer` (paired with `SvgRenderer`), `TerminalScreen` (a `Screen` backed by `TerminalRenderer`) and the editor's `Screen` trait (the only screen left once the other two are renamed).

Fields and parameters follow their types: `terminal: Terminal` becomes `window: Window` in `TerminalRenderer`, `Frame` and `on_resize`, and the `terminal(cols, rows, …)` test helper becomes `window(…)`.

### Steps

1. Pure renames (`tty`, `Window`, `RawMode::enter`, `Frame`, `TtyKeySource`). The build and tests stay green, and no code moves.
2. Move `editor::open` to `bootstrap::run` and point `lib::run` at it. `editor/mod.rs` drops its `kitty`, `tty`, `render` and `std::os::fd` imports.
