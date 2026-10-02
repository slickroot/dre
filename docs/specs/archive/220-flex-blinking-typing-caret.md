Doug is typing a label in dre-flex. As soon as he enters typing mode a `|` appears
immediately after the last character, right where the next character will land. It
does not blink — it just sits there steadily while he types, and typing another
character moves it one cell along with no other text shifting. When Doug backspaces
to empty, the `|` is still there, in the cell where the first character will land.
The moment he leaves typing mode, it is gone. It never pushes the rest of the text
around.

## Acceptance Criteria

1. Pressing the key that starts typing on a box with no text shows the `|`
   immediately, before any character is typed.
2. The `|` sits immediately after the last typed character — flush against the
   boundary of the next cell, not centred in it. Typing another character moves the
   `|` one cell further along, with no other text shifting.
3. The `|` is steady. It does not blink.
4. Backspacing to an empty label leaves the `|` visible in the first cell.
5. The `|` is drawn only in typing mode; once Doug leaves typing mode it is not
   drawn.
6. The `|` does not occupy a cell — nothing after it moves.

## Technical Design

This supersedes two earlier attempts: a terminal-driven kitty animation (frames
transmitted with `a=f`/`a=a`) and an app-side tick (a poll timeout on `read_key`, a
phase field and a repaint per tick). Both are rejected below. The caret is now an
ordinary, steady placement.

### No clock anywhere

The caret is static. `flex::view::paint` emits it on every frame while Doug is in
write mode, and the ordinary `VirtualTerminal` diff carries it: an unchanged caret
commits no ops (`virtual_terminal.rs:399`). There is no `Instant`, no poll timeout,
no phase field on `FlexState`, no `kitty::blink`, and no `Content::Animation`. So
`PartialEq for FlexState` (`src/flex/state.rs:106-110`) is untouched and the ~100
existing `assert_eq!`s on states keep passing.

This is the whole reason it is a placement and not an animation: the two failed
attempts both paid for motion. The terminal animation needed the terminal to play
frames (invisible where it does not) and could only animate a glyph, not a bar at
the cell's left edge; the tick needed a timer and a repaint loop four times a
second forever, plus an equality carve-out. A steady caret needs neither.

### The caret is a placement like any other

`flex::view::paint` (`src/flex/view.rs:248-304`) grows a `TypingCaret` arm. It emits
the node only when `state.mode == FlexMode::Write && &state.selected == path`, which
is criterion 5. `PlacementNode::TypingCaret { colour: Rgb, bold: bool }` is a 1×1
placement and is listed in `is_decoration()` (`src/view.rs:67`), so it is excluded
from the bounding-box/centre filter at `src/view.rs:337` and does not drag the
centred stack. It does not touch `text_size` or `measure`, so criterion 6 holds:
nothing reflows and no cell is reserved.

`PlacementNode::Caret` is left alone. It is the legacy editor's solid block cursor
(`draw_caret`, `src/render/terminal.rs:634`), and its tests pin that meaning;
`TypingCaret` is a different node for a different editor.

### Position: `text.x + chars().count()`

`arranged.text` is the label's `Rect`. The caret rect is

```rust
Rect { x: text.x + text.chars().count() as i64, y: text.y, width: 1, height: 1 }
```

in `FLEX_TEXT_COLOUR`, `bold: false`. Two readings of "the cell after the last
character" diverge exactly when the label is empty, because `text_size` sizes the
label with `view::interior` (`src/view.rs:114-116`), which floors an empty label's
width at 1. The caret uses `chars().count()`, not `text.width`:

- Empty text gives `text.x + 0`, the cell where the first character is actually
  drawn — criterion 1, and what survives criterion 4.
- Non-empty text gives `text.x + len`, one cell past the last glyph, and
  `draw_label` draws from `text.x`, so typing advances the caret by exactly one cell
  with nothing else moving.

Using `text.width` instead would put the caret one cell too far right on an empty
label and make it jump left on the first keystroke. There is no clamp; if the label
fills the box, the placement is still emitted and `clip_natural` decides visibility.

### The bar is at the left edge, and the offset lives in the image

The old caret was the `|` glyph, whose ink is centred in its cell, so at
`text.x + chars().count()` it read as sitting in the middle of the next cell with a
half-cell gap from the last character. Placements are cell-granular: `Desired.col`
(`virtual_terminal.rs:41-48`) is an integer cell and `clip_natural` works in cells,
so a half-cell offset cannot ride on the placement. It has to live inside the image.

`Sprites::content` (`src/render/terminal.rs:788`) for `ImageKey::TypingCaret` builds a
one-cell canvas with `Canvas::fill` and a new `CaretShape`:

```rust
struct CaretShape { bar: i64, colour: Rgba }
impl Shape for CaretShape {
    fn colour_at(&self, x: i64, _y: i64) -> Option<Rgba> {
        (x < self.bar).then_some(self.colour)
    }
}
```

`bar = (cell_width / 8).max(1)`, the whole cell height. The leftmost lit column is
x = 0, i.e. the shared boundary with the previous cell that holds the last
character — criterion 2, "exactly next to". Nothing is looked up in `glyph_source`;
the font's centred `|` is never used, and the caret stays crisp at any cell size.

### Pipeline and keys

`paint_scene` (`src/render/terminal.rs:340-380`) maps `PlacementNode::TypingCaret`
to a `Desired { image: ImageKey::TypingCaret(TypingCaretKey), col, row, z, .. }`,
clipped like any 1×1 node and z-ordered at `depth_z(placement.depth)` alongside the
label. `ImageKey` (`virtual_terminal.rs:19-26`) gains `TypingCaret(TypingCaretKey)`
and

```rust
pub(super) struct TypingCaretKey { pub(super) colour: Rgb, pub(super) bold: bool }
```

mirrors `GlyphKey`. The canvas is a pure function of `(colour, bold, cell size)`;
cell size is handled by `on_resize`, which already calls `self.vt.reset()`
(`src/render/terminal.rs:299-302`), so a font/cell-size change frees the caret image
and re-uploads it at the new size. Only one `{colour, bold}` pair is ever used, so
the cache holds at most one caret image, and a steady caret commits no ops after the
frame that first places it.

### Collaborators and responsibilities

**`flex::view::paint`** (`src/flex/view.rs:248`) — decides *whether* a caret exists
(write mode and selected) and *where* (`text.x + chars().count()`). Emits a 1×1
`TypingCaret`; knows nothing about canvases or the pipeline.

**`PlacementNode::TypingCaret`** (`src/view.rs`) — carries only colour and bold, is a
1×1 placement, and is a decoration so it never affects centre or measure.

**`TerminalRenderer::content`** (`src/render/terminal.rs:787`) — turns the key into the
left-edge bar canvas via `CaretShape`. Owns the geometry offset; the placement stays
cell-granular.

**`VirtualTerminal`** — untouched. It keys, diffs and places the caret exactly as it
does a glyph.

### Tests

Framework is inline `#[test]` with plain `assert!`/`assert_eq!`, no snapshots. The
assertion target is the placements and the built `Desired`/canvas, not escape bytes.

In `src/flex/view.rs`:
- `write_mode_puts_a_typing_caret_one_cell_past_the_label`
- `an_empty_label_puts_the_caret_in_the_first_cell`
- `typing_the_next_character_moves_the_caret_right_by_one_cell`
- `backspacing_to_empty_leaves_the_caret_in_the_first_cell`
- `move_mode_draws_no_typing_caret`
- `the_caret_placement_is_one_cell_wide_and_does_not_widen_the_label`
- `the_caret_is_a_decoration_and_stays_out_of_the_bounding_box`

In `src/render/terminal.rs`:
- `the_typing_caret_canvas_has_a_lit_column_at_its_left_edge`
- `the_typing_caret_canvas_is_transparent_past_the_bar`
- `the_typing_caret_canvas_spans_the_whole_cell_height`
- `the_typing_caret_desired_sits_at_the_label_end_cell`
- `the_typing_caret_image_is_still_and_not_an_animation`
- `an_unchanged_typing_caret_commits_no_ops`
- `the_typing_caret_is_re_placed_as_the_label_grows`

### Rejected

- **Terminal-driven kitty animation** (the first attempt) — needs the terminal to
  play `a=a` frames, so it is invisible where frames are not played, and its frames
  are glyph canvases, so it cannot hold a bar at the cell's left edge.
- **App-side tick** (the second attempt) — a `PollTimeout` on `read_key`, a phase
  field and a whole-frame repaint 4×/sec forever, plus a phase carve-out on state
  equality. Nothing here needs motion.
- **Centred `|` glyph** — the half-cell gap from the last character is the reported
  bug; the placement grid cannot offset by half a cell, so the bar is built into the
  image instead.
- **`text.x + text.width` as the caret origin** — `interior` floors an empty label at
  1, so the caret starts one cell too far right and jumps left on the first
  keystroke.
- **Reusing `PlacementNode::Caret`** — drags the legacy editor's solid block meaning
  into flex and forces every legacy `Caret` construction site and test to change.
