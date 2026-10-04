# dre-flex is four simple layers

## Problem

A keypress in `dre-flex` crosses thirteen layers and six data shapes
before a pixel changes:

`key → FlexState → Arranged → Placement → Desired → Op → bytes`

Most of that weight is borrowed from the main editor and does not pay
for itself in flex:

- `PlacementNode::Box` carries nine flags. Flex uses them to encode
  meaning: the selection outline is a `Box` with `gap: true`, the
  replace highlight is a `Box` with `opacity` and `fill`.
- The "new box" set lives in `TerminalFlexScreen::drawn`, outside
  `FlexState`, and reaches the renderer as a `grow` flag that becomes
  an animation.
- `TerminalRenderer` has three ways to draw one box (grow, tiles,
  cached sprite), a sprite cache, a tile cache and a glyph cache.
- `VirtualTerminal::commit` diffs every frame against the last one to
  keep Kitty uploads small.

This is a refactoring spec: no behaviour the user can see changes,
except where "Out of scope" below says so.

## Target

Four layers. Each layer reads only the previous layer's output.

```
FlexState ──▶ Vec<Placement> ──▶ Frame ──▶ terminal bytes
  state         placements        canvas       kitty
```

Every loop iteration redraws everything from scratch. There is no
diffing, no caching, no animation and no `VirtualTerminal`.

## Technical Design

All four layers live under `src/flex/`, one file each, plus the
existing `mod.rs` loop. Nothing under `src/flex/` imports from
`render/*` or `view.rs`.

### 1. State — `flex/state.rs`, `flex/history.rs`

`FlexState` and `reduce(state, key) -> (state, effect)` stay as they
are. The `drawn` set in `TerminalFlexScreen` and `new_boxes()` are
deleted along with the grow animation.

### 2. Placements — `flex/placements.rs`

`pub(crate) fn placements(state: &FlexState, window: Cells) -> Vec<Placement>`

`measure`, `arrange` and `paint` from `flex/view.rs` are kept as the
private steps of this one function. Output is in **cells**, with (0, 0)
at the top-left of the root canvas. A Placement says its role and
nothing about its look: no colours, no opacity, no thickness in pixels.

```rust
struct CellRect { x: i64, y: i64, width: i64, height: i64 }

enum Placement {
    Box { rect: CellRect, filled: bool },
    Text { at: (i64, i64), text: String },
    Highlight { rect: CellRect },   // replace-mode selected text
    Caret { at: (i64, i64) },       // write-mode typing position
    Outline { rect: CellRect },     // move-mode selection; rect is the
                                    // selected box grown by one cell
}
```

Placements are listed in paint order; `Outline` is last (spec 240).
`Scene`, `Area`, `PlacementNode`, the `grow` and `gap` flags are not
used.

### 3. Canvas — `flex/canvas.rs`

`pub(crate) fn draw(placements: &[Placement], window: Window) -> Frame`

```rust
struct Frame {
    pixels: Canvas,          // crate::canvas::Canvas, window-sized
    texts: Vec<TextRun>,     // { col, row, text, colour }
    caret: Option<(i64, i64)>,
}
```

Canvas is where cells become pixels. It owns the **theme** (colours,
highlight opacity, fill) and the **border thickness** (spec 239, from
the screen DPR) and the outline gap. It rasterizes `Box`, `Highlight`
and `Outline` into `pixels` through the `Shape` trait. It does not
rasterize text: each `Text` becomes a `TextRun` that already carries
its theme colour, and `Caret` becomes `caret`.

### 4. Kitty — `flex/kitty.rs`

`pub(crate) fn show(frame: &Frame, out: &mut impl Write) -> io::Result<()>`

Kitty only sees a `Frame` and knows nothing about boxes, roles or
themes. Inside one synchronized update it:

1. transmits `frame.pixels` as the single image and places it at cell
   (0, 0) with `z = -1` (above cell backgrounds, below characters),
   reusing one image id so each frame replaces the last;
2. writes each `TextRun` with a cursor-move and a truecolor SGR;
3. shows the terminal cursor at `frame.caret`, or hides it when `None`.

Text is drawn by the terminal's own font. The glyph path is gone from
flex.

### The loop — `flex/mod.rs`

`run_loop(keys, screen)` keeps its shape. `FlexScreen::render(&state)`
becomes `show(&draw(&placements(state, window), window))`, with
`window` (cols, rows, cell size in pixels) from `tty::probe` and
refreshed on resize. `resize` just re-probes; there is no
renderer to reset beyond that. `tty`, `key_source` and the
mockall-based loop tests stay.

### What flex may import from the rest of dre

Leaf primitives only: `tty`, `key_source`, `canvas::{Canvas, Shape}`
and the low-level Kitty commands (`kitty::require`, `transmit`,
`place_ext`, `delete`). Not `render/*`, `view.rs`, `font.rs` (flex no
longer rasterizes text) or `kitty::encode_ops` (tied to `Op`). The
main editor keeps working untouched.

### Migration

Build the new layers **alongside** the old ones, one layer per slice,
keeping `dre-flex` runnable after every slice:

1. `placements.rs`: the `Placement` enum and `placements()`, with the
   existing `view.rs` layout tests ported to assert geometry and role.
2. `canvas.rs`: `Frame` and `draw()`, tested on pixels at known cells.
3. `kitty.rs`: `show()`, tested on the bytes written to a `Vec<u8>`.
4. Switch `TerminalFlexScreen` to `show(draw(placements(..)))`.
5. Delete `flex/view.rs`, `new_boxes`, `drawn` and the flex-only
   renderer parts that nothing else uses.

## Acceptance Criteria

- `src/flex/` has `state`, `placements`, `canvas` and `kitty` layers,
  and no module under it imports from `render/*` or `view.rs`.
- `FlexState` has no knowledge of the screen, the renderer or pixels.
- `Placement` carries no colour, opacity or pixel size.
- A frame is drawn from scratch on every loop iteration: no previous
  frame is kept, no `VirtualTerminal`, no sprite, tile or glyph cache.
- Text and the typing caret are drawn by the terminal; the Kitty image
  is placed under them at `z = -1`.
- Every flex behaviour covered today still holds: layout, selection
  outline painted last, borders, fill, replace highlight, caret, undo.

## Out of scope

- The grow animation for new boxes is **dropped**, not ported. This is
  the one visible change; boxes simply appear.
- The replace-mode highlight is a tinted rect under the text rather
  than a blend over it, since text is no longer in the bitmap.
- Making the full-frame upload cheap. Every loop sends one
  window-sized RGBA image; if that is too slow, compress or diff in the
  Kitty layer later. Measure first.
- Any change to the main editor, `VirtualTerminal` or `render/*`.
  Spec 242 (benchmarking `VirtualTerminal::commit`) still applies to
  the editor, but no longer to flex.
