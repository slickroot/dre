# dre-flex starts over as an empty canvas

## Problem

`src/flex/state.rs`, `history.rs` and `view.rs` (5650 lines) route a
keypress through `FlexState → Arranged → Placement → Desired → Op →
bytes` before a pixel changes, and the layer that owns pixels is
shared with the main editor's renderer (sprite/tile/glyph caches,
`VirtualTerminal` diffing). None of this has been measured, and it is
too large and too coupled to the main editor's renderer to reason
about or trust on performance. Spec 243 proposed slicing this into
four layers, but that still carries the model forward and migrates
the old renderer piece by piece.

This spec throws all of it away instead: the editor starts, draws a
canvas once, and does nothing else. Every future flex behaviour
(boxes, selection, typing, layout) is rebuilt from here, one spec at a
time, each free to shape the architecture it actually needs.

## Target

`dre-flex` starts, fills the full terminal window with a solid colour
through Kitty's graphics protocol, and then does nothing: no key
changes anything on screen, no resize handling, no state, no layout.
Ctrl-C is the one key that is special-cased, to leave the terminal the
way `dre-flex` found it.

## Technical Design

### What is deleted

`src/flex/state.rs`, `src/flex/history.rs` and `src/flex/view.rs` are
deleted entirely, along with everything only they used: `FlexState`,
`FlexBox`, `reduce`, undo history, `Scene`/`Area`/`PlacementNode`,
`new_boxes`/`drawn`, and `TerminalFlexScreen`/`FlexScreen`. `dre-flex`
is its own binary, so nothing else in the repo depends on them; the
main editor (`render/*`, `kitty::encode_ops`) is untouched.

### What stays, shared with the rest of `dre`

Only `crate::tty::{probe, RawMode}` is imported — `probe` for the
window size in pixels at startup, `RawMode` for the terminal. The
resize pipe is not installed, since resize is not handled.
`crate::key_source` is not used either; `mod.rs` reads stdin directly
(see below).

Neither `crate::canvas` nor `crate::kitty` is imported. Both also
serve the main editor (`canvas::Canvas` is used by `render/shapes.rs`,
`render/brackets.rs`, `render/font.rs`, `render/terminal.rs`,
`render/virtual_terminal.rs`, `render/tiles.rs`; `kitty.rs` by
`render/virtual_terminal.rs`, `render/terminal.rs`, `editor/
bootstrap.rs`), and this spec does not touch either file or their
callers. Instead, the small pixel buffer flex needs and the handful
of Kitty escape sequences to put it on screen — requiring the
graphics protocol, transmitting the image, placing it — are copied as
private code straight into `src/flex/mod.rs`. Flex ends up depending
on nothing but `crate::tty`.

### `src/flex/mod.rs` — everything, flat

No submodules. One file, exposing `pub fn run() -> ExitCode`.

Startup, in order: the copied `require`, `tty::probe` for the window
size in pixels, `tty::RawMode::enter`, then one solid-colour pixel
buffer filled at that size (a copied, minimal stand-in for
`Canvas::fill` — just a flat `Vec<u8>` of one repeated RGBA colour,
no `Shape` trait needed since there is only ever one colour),
transmitted and placed once via the copied `transmit` + `place_ext`.

The loop after that reads one byte at a time from stdin directly (no
`KeySource` trait, no mock — this is deliberately the simplest form)
and does nothing with what it reads, except: if it reads `0x03`
(Ctrl-C), it returns, which drops `RawMode` and restores the terminal.

No test seam is added for the loop, and no tests are ported from
`canvas.rs` or `kitty.rs` — the copied code here is a throwaway
stand-in sized for one solid-colour fill, not a reusable module;
this slice adds no logic worth a unit test beyond "it compiles and
starts."

### Migration

No incremental migration: this is one slice. Delete the three files,
replace `mod.rs`, confirm `dre-flex` starts, paints, and Ctrl-C exits
cleanly.

## Acceptance Criteria

- `src/flex/` contains exactly one file, `mod.rs`.
- Running `dre-flex` fills the terminal with a solid colour and then
  responds to nothing except Ctrl-C.
- Ctrl-C restores the terminal (raw mode is left cleanly, same as
  today).
- No resize handling, no state, no undo, no layout, no caching, no
  diffing.

## Out of scope

- Everything dre-flex used to do (boxes, typing, selection, undo,
  move mode) — this spec removes it; later specs rebuild what's
  needed, each free to choose its own architecture.
- Resize handling — revisited only when a future spec needs the
  window size to change after startup.
- Performance measurement itself — this spec only gets the editor
  back to a state simple enough to measure from.
