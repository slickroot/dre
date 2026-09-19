# 077: Kitty module and a thin main

Refactoring spec — no user story. Placeholder, to be designed.

## Goal

Move the Kitty graphics protocol (encoding, chunking, escapes, `KittyGraphics`, `DELETE_ALL`, and the support query now in `writer.rs`) into `src/kitty.rs`. `main.rs` keeps only `mod` declarations and `main()`.

## Technical Design

### Responsibility

`kitty` is the only module that speaks the Kitty graphics protocol. Callers
say what they want — "clear the images", "show this sprite here", "is Kitty
supported?" — and `kitty` gives back the exact commands to send to the
terminal. No escape sequence for graphics is built anywhere else.

### Components

- `src/kitty.rs` — new. Holds `Sprite`, `Command`, `clear`, `show`,
  `supported`, and the private encoding (zlib, base64, chunking, escapes).
- `src/main.rs` — only `mod` declarations and `main()`. No `use` of base64 /
  flate2, no constants, no tests.
- `TerminalRenderer` (src/render.rs) — builds sprites and asks `kitty` for the
  commands.
- `writer.rs` — calls `kitty::supported` at startup.

### Dependency direction

`render → kitty` and `writer → kitty`, never the other way. `kitty` depends on
nothing inside the crate (only `base64`, `flate2`, `nix`, `std`).

Today there is a cycle: `KittyGraphics::draw` (in `main.rs`) takes
`render::Sprite`, while `TerminalRenderer` holds a `crate::KittyGraphics`. It
exists because `main.rs` is the crate root, so `render` could reach
`crate::KittyGraphics` for free, while `Sprite` was defined next to the code
that builds it. It is broken by moving `Sprite` into `kitty`: a sprite is a
Kitty "image placed at a cell", and only Kitty ever draws it.

### The interface

```rust
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Sprite {
    pub(crate) pixels: Vec<u8>,   // RGBA
    pub(crate) width: i64,
    pub(crate) height: i64,
    pub(crate) col: i64,
    pub(crate) row: i64,
}

pub(crate) struct Command(String);      // field private: only kitty builds one
impl Display for Command { ... }        // callers write!/push_str it

pub(crate) fn clear() -> Command;                // delete all images
pub(crate) fn show(sprite: &Sprite) -> Command;  // move to (col,row) + transmit
pub(crate) fn supported<W: Write>(stream: &mut W, stdin_fd: RawFd) -> io::Result<bool>;
```

- `Command` is opaque: nobody outside `kitty` can construct one, so graphics
  bytes can't be built elsewhere. It is always referred to qualified as
  `kitty::Command` (not imported bare) to avoid confusion with `cli::Command`.
- `clear()` = today's `DELETE_ALL`.
- `show(&sprite)` = today's per-sprite body of `KittyGraphics::draw`: the
  cursor move `\x1b[{row+1};{col+1}H` followed by `transmission(..)`.
- Building commands is pure; `supported` is the only function in `kitty` that
  does I/O, because it has to read the terminal's reply.

### Private to `kitty`

`zlib`, `base64`, `encode`, `chunks`, `more`, `escape`, `transmission`,
`CHUNK_SIZE`, `DELETE_ALL`, `QUERY` (today's `KITTY_GRAPHICS_QUERY`),
`REPLY_TIMEOUT_MICROS`, and `is_supported`.

### `KittyGraphics` is removed

It is an empty struct whose only method is "clear, then show each sprite", which
`clear()` + `show()` now cover. No trait / fake is introduced:

- `kitty`'s commands are pure and deterministic, so the real functions are
  already a perfect test double — tests build expected output from them.
- `TerminalRenderer`'s testing seam is `sprites()` (what to draw, as
  `Vec<Sprite>`), not the encoding (how to say it).
- A switchable trait would force a fake to construct `kitty::Command`, which
  conflicts with keeping it opaque.

If a second graphics protocol ever appears, a trait is introduced then.

### Support query

`supports_kitty_graphics` moves from `writer.rs` into `kitty` as `supported`,
split into:

- `fn is_supported(reply: &[u8]) -> bool` — pure: the reply contains `i=1`.
- `pub(crate) fn supported(stream, stdin_fd)` — writes `QUERY`, puts stdin in
  raw mode, waits up to the timeout with `select`, reads, restores the terminal,
  returns `is_supported(&reply)`.

`NOT_SUPPORTED_MESSAGE`, `CLEAR_LINE` and the `ExitCode::FAILURE` stay in
`writer.rs` — what to do when Kitty is missing is the application's decision.

Note for 080: after this spec `supported` and `RawModeGuard` both do the
`tcgetattr` / `cfmakeraw` / `tcsetattr` dance. Accept that duplication; when
`writer` becomes `editor/`, `kitty` must **not** reuse the editor's raw-mode
helper (that would reverse the arrow). If it ever matters, extract a small
`tty` module both depend on.

### Changes to `TerminalRenderer` (src/render.rs)

- `Sprite` definition removed; uses `kitty::Sprite`.
- `graphics` field removed; `new(graphics, cell_width, cell_height)` becomes
  `new(cell_width, cell_height)`.
- In `draw`, the payload appended to the last line becomes `kitty::clear()`
  followed by `kitty::show(&sprite)` for each sprite.

### Changes to `writer.rs`

- `use crate::KittyGraphics` removed; `TerminalRenderer::new(cell_width,
  cell_height)`.
- `supports_kitty_graphics`, `KITTY_GRAPHICS_QUERY` and
  `KITTY_GRAPHICS_REPLY_TIMEOUT_MICROS` removed; `write` calls
  `kitty::supported(&mut stdout, stdin_fd)`.

### Tests

- Move to `kitty.rs`: all current `main.rs` tests (chunks, more, escape,
  encode round trip, single- and multi-chunk transmission).
- New in `kitty.rs`:
  - `clear()` displays as `\x1b_Ga=d,d=A,q=2;\x1b\\`.
  - `show(&sprite)` is the cursor move to `(row+1, col+1)` followed by the
    transmission of its pixels.
  - `is_supported` is true for a reply containing `i=1`, false for an empty or
    unrelated reply.
- Updated in `render.rs` and `writer.rs`: assertions on `crate::DELETE_ALL`
  become `kitty::clear().to_string()`; `TerminalRenderer::new` loses the
  graphics argument.
- New in `render.rs`: a frame with sprites ends its last line with
  `kitty::clear()` followed by `kitty::show(..)` for each sprite, in order.
