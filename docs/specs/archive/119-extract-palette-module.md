# Extract palette.rs

Step of spec 117 (one-direction architecture). This is a technical spec, not a
user story: it has no user-facing behaviour.

## Problem

`palette()`, `PALETTE`, `FOREGROUND` and `BACKGROUND` live in `diagram.rs`, but they are the app's colours, used by the renderers and the status line. The drawing does not use them: `Node.colour` is only an index.

## Acceptance Criteria

- A new `src/palette.rs` holds `PALETTE`, `FOREGROUND`, `BACKGROUND` and `palette()`, unchanged.
- `diagram.rs` no longer defines or re-exports them, and every user imports from `palette`.
- No behaviour changes. Existing tests pass unchanged apart from imports.

## Technical Design

Mechanical move. `palette.rs` is a leaf: it imports nothing, and `diagram.rs` does not use it (nothing in `diagram.rs` outside its own palette tests refers to it). See spec 117 for where it sits in the one-direction architecture.

### `src/palette.rs`

- Holds `PALETTE`, `FOREGROUND`, `BACKGROUND` and `palette(index: u8) -> Option<(u8, u8, u8)>`, moved as they are.
- `PALETTE` stays private. `palette()` is the only way to read a colour, so callers cannot index the array and skip the `Option` for an out-of-range index. `palette()`, `FOREGROUND` and `BACKGROUND` stay `pub(crate)`.
- The four palette tests move from `diagram.rs` into `palette.rs`, unchanged (index 0 exists, foreground, background, nothing past the last index).

### `lib.rs`

- Add `mod palette;` with no `cfg` gate. `state` and `render/svg` use it and both build for wasm.

### Callers

Imports change from `crate::diagram::…` to `crate::palette::…` in `state.rs`, `render/mod.rs`, `render/svg.rs`, `render/terminal.rs`, `terminal.rs`, `dre_format.rs` and the `command_mode.rs` tests. Names that stay in `diagram` (`Node`, `Document`, `Path`, `node` and so on) keep their `diagram` import. `diagram.rs` does not re-export anything from `palette`.

### Not in this spec

`PALETTE_ROWS: i64 = 7` in `render/terminal.rs` repeats the palette's length. It is left alone because this is a move with no behaviour change.

### Slice

One slice, one commit: create `palette.rs`, move the code and tests, fix the imports, and check that `cargo test` passes and `diagram.rs` has no palette left.
