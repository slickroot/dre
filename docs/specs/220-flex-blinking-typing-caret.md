Doug is typing a label in dre-flex. As soon as he enters typing mode a `|` blinks at the end of his text, right where the next character would land. It blinks once per second and sits in the empty cell after the text, so nothing moves. When Doug backspaces to empty, the `|` is still there, blinking. The moment he leaves typing mode, it's gone.

## Acceptance Criteria

1. Pressing the key that starts typing on a box with no text shows the `|` immediately, before any character is typed.
2. The `|` sits in the cell immediately after the last typed character, and typing another character leaves the `|` one cell further along, with no other text shifting.
3. The `|` is visible for roughly half a second and hidden for roughly half a second, on that repeating cycle.
4. Backspacing to an empty label leaves the `|` visible in that position, still blinking.
5. The `|` blinks only in typing mode; once Doug leaves typing mode it is not drawn.
6. The `|` does not occupy a cell — nothing after it moves.

## Technical Design

### The clock belongs to the terminal

There is no timer anywhere in dre. `flex::run_loop` (`src/flex/mod.rs:59-74`) blocks on
`tty::read_key`, which polls with `PollTimeout::NONE` (`src/tty.rs:124-150`), and the renderer
redraws once per keystroke. A caret that blinks once a second needs something to decide twice a
second whether it is lit.

The terminal decides. Kitty's graphics protocol has terminal-driven animations: frames are
transmitted with `a=f` and millisecond gaps via `z`, and `a=a,i=<id>,s=3,v=1` starts an
animation that loops forever. The app transmits the two frames once and never speaks about the
caret again. There is no tick, no `Instant`, and no new field on `FlexState` — which also means
`PartialEq for FlexState` (`src/flex/state.rs:106-110`) is untouched and the ~100 existing
`assert_eq!`s on states keep passing.

The alternative, a finite poll timeout on `read_key` plus a phase field, was rejected: it repaints
the whole frame four times a second forever, and it needs the blink phase excluded from state
equality.

### The blink is two frames, and the root is the dark one

Kitty composes each frame onto the root using a full alpha blend by default. An all-transparent
"off" frame blended onto a root that *is* the `|` leaves the `|` fully visible — transparent over
opaque is a no-op, not an erase. So a naive two-frame blink renders as a steady caret.

Two escapes were considered. The first was `X=1`, which makes a frame a replacement of the
background rather than a blend, letting a transparent frame erase a lit root. It was rejected
because this repo has already been bitten by terminal divergence (`src/kitty.rs:79`,
`src/kitty.rs:115`, the Ghostty note at `src/render/terminal.rs:141`) and `X` is exactly the kind
of key WezTerm's implementation history here is unverified.

Instead the **root frame is transparent and the lit `|` is frame 2**. Blending an opaque glyph
over a transparent root works with the default alpha, so no exotic key is needed. This costs one
thing: a terminal that never plays frames shows only the root, hence an invisible caret. That
trade is accepted — the alternative trades "invisible on old Ghostty" for "solid on WezTerm", and
Doug is in WezTerm.

The root frame is created gapless, so the dark half needs its own gap: the command ends
`a=a,i=<id>,r=1,z=<gap>,s=3,v=1`. Without `r=1,z=` the cycle would be "500ms lit, 0ms dark".

```
a=T  f=32  s=<cw>  v=<ch>  o=z  i=<id>   transparent root
a=f  f=32  s=<cw>  v=<ch>  o=z  i=<id>  z=500   the `|` glyph
a=a  i=<id>  r=1  z=500  s=3  v=1      loop forever, dark half included
```

### A new node, because `Caret` means something else

`PlacementNode::Caret(Caret)` exists and is a unit struct drawn by `draw_caret`
(`src/render/terminal.rs:824-836`) as a **solid filled block**; it is the legacy editor's
insert-mode caret, and its tests (`the_caret_is_drawn_last_as_a_solid_sprite`,
`src/render/terminal.rs:2340`) pin that meaning.

The flex caret is a different thing: a glyph, in the label's colour, at the label's depth z, as a
two-frame animation rather than a fresh solid canvas per frame. So `PlacementNode` gains

```rust
TypingCaret { colour: Rgb, bold: bool }
```

`Caret` is untouched, which keeps the legacy editor and its tests out of this change entirely.

### Collaborators and responsibilities

`PlacementNode::TypingCaret` (`src/view.rs`) — knows only colour and bold. Carries no position
logic; it is a 1×1 placement like any other. `is_decoration()` includes it, so it never affects
measure.

`flex::view::paint` (`src/flex/view.rs:248-304`) — decides *whether* a caret exists. Emits the
node only when `state.mode == FlexMode::Write` and `path == state.selected`, which is what
satisfies criterion 5. It does not touch `text_size` or `measure`, so criterion 6 holds: nothing
reflows, and no cell is reserved.

`TerminalRenderer::draw_typing_caret` (`src/render/terminal.rs`) — owns the image id and the
transmission. Looks the id up in a new `HashMap<CaretKey, ImageId>` where `CaretKey` is
`{colour, bold}`, mirroring `GlyphKey` and the existing `glyph_images` cache
(`src/render/terminal.rs:813-820`). First sight transmits the three-command animation above via
`place_fresh`; every later frame is an `a=p` placement only, via `place_cached`. Moving the caret
one cell right as Doug types therefore re-places a live animation rather than rebuilding it.

`kitty` (`src/kitty.rs`) — gains a `blink` command builder next to `grow`, emitting the `a=T`,
`a=f` and `a=a` sequence with a `gap_ms` parameter. Reuses the existing `chunked`, `transmission`
and `wezterm`-aware `Z=` handling (`src/kitty.rs:201-216`).

The glyph canvas comes from `glyph_source.glyph('|', colour, bold)`, exactly as `draw_label`
obtains each character, so the caret is the same font and colour as the text it follows.

### Position: `text.x + chars().count()`

`text_size` (`src/flex/view.rs:69-74`) sizes the label rect with `view::interior(text)`, and
`interior` is `(chars().count() as i64).max(1)` (`src/view.rs:114-116`). An **empty** label
therefore still reserves one cell, so "the cell after the last typed character" has two readings
that diverge exactly when the text is empty.

The caret uses `text.x + chars().count()`, not `text.x + text.width`. Consequences:

- Empty text gives `x + 0`, the cell where the first character will actually be drawn, which is
  what criterion 1 asks for ("before any character is typed") and what survives criterion 4
  (backspace to empty).
- Non-empty text gives `text.x + len`, one cell past the last glyph — and `draw_label` draws from
  `text.x`, so typing advances the caret by exactly one cell with nothing else shifting.

The `.max(1)` in `interior` stays a layout concern and is deliberately not reused for the caret;
using `text.width` instead would put the caret one cell right of where typing lands, so it would
jump left on the first keystroke.

There is no clamp. If the box is full and the caret lands on or past the border, the placement is
still emitted and `Frame::crop` (`src/render/terminal.rs:358-380`) decides visibility.

### Lifetime: one image, for the session

`soft_clear` is `a=d,d=a`, which deletes placements *without* freeing image data
(`src/kitty.rs:48-50`). So once the caret image exists, the terminal keeps ticking its frames for
the rest of the session regardless of what is placed — `soft_clear` runs at the top of every frame
(`src/render/terminal.rs:447`).

Rather than have the renderer notice the caret disappearing and emit a `delete`, the image id is
**permanent**: transmitted once on first sight, cached for the session, never freed. Only one
`{colour, bold}` pair is ever used, so the leak is bounded to a single image id — and skipping it
means no lifecycle flag, no renderer-side diff against the previous frame, and nothing to
re-transmit when Doug re-enters typing mode.

The cost is honest and worth recording: a 2fps animation keeps running in the terminal after Doug
leaves Write mode for the first time, until he quits. It is not visible, because the placement is
simply not emitted.

### SVG export is unreachable here

`SvgRenderer` is called from exactly one place, `cli::export`, and it renders
`view::body(&state, window)` for a **legacy** `State` (`src/cli.rs:86-96`). `flex::view::scene` is
never handed to it. So the `match` arm for `TypingCaret` in `src/render/svg.rs:103-127` exists
only for exhaustiveness: it emits nothing, with a comment saying flex scenes are never exported.
A blinking cursor has no meaning frozen in time, and emitting a static `|` would be
indistinguishable from a literal pipe character in the label.

### Tests

Framework is inline `#[test]` with plain `assert!`/`assert_eq!`, no snapshots; existing tests use
the private `composed` / `sprites` / `grid` helpers (`src/render/terminal.rs:1616-1715`) and
`kitty.rs`'s string assertions on escape bytes (`src/kitty.rs:432-521`).

The assertion target is the **frames plus the control bytes**, not the escape sequence alone.

`FakeGlyphSource::glyph` (`src/render/font.rs:199-222`) returns fully transparent for
`style::rgb(None)` = `palette(FOREGROUND)` = `(232, 234, 237)` (`src/style.rs:63`), and an opaque
solid for any other colour. `FLEX_TEXT_COLOUR` is `(201, 201, 207)` (`src/flex/view.rs:17`) —
different, so **the real flex colour is already non-default** and the fake yields an opaque lit
frame against a transparent root. No test-only colour is needed, and neither `GlyphSource` nor the
fake changes.

In `src/flex/view.rs`:
- `write_mode_puts_a_typing_caret_one_cell_past_the_label`
- `an_empty_label_puts_the_caret_in_the_first_cell`
- `typing_the_next_character_moves_the_caret_right_by_one_cell`
- `backspacing_to_empty_leaves_the_caret_in_the_first_cell`
- `move_mode_draws_no_typing_caret`
- `the_caret_placement_is_one_cell_wide_and_does_not_widen_the_label`

In `src/render/terminal.rs`:
- `the_typing_caret_root_frame_is_transparent` — root canvas alpha is all zero
- `the_typing_caret_lit_frame_is_the_bar_glyph` — frame 2 is opaque in the caret colour
- `the_typing_caret_is_transmitted_once` — the first frame's bytes contain exactly one `a=T`, one
  `a=f`, one `a=a`, with `z=500`, `r=1`, `s=3`, `v=1`
- `the_typing_caret_is_re_placed_and_not_re_transmitted` — a second render at a new column emits
  `a=p` and no `a=T`
- `the_typing_caret_is_the_same_image_for_the_same_colour` — two placements in one frame share
  one id
- `the_legacy_caret_is_still_a_solid_block` — regression guard for
  `draw_caret`, which this change must not disturb

In `src/kitty.rs`:
- `blink_transmits_a_transparent_root_then_the_lit_frame`
- `blink_holds_the_root_for_its_gap_and_loops_forever`
- `blink_frames_carry_capital_z_in_wezterm`

### Rejected

- **App-side tick** (`PollTimeout` on `read_key`, phase on `FlexState`) — repaints the whole frame
  4×/sec forever, and the phase field would have to be excluded from state equality.
- **`X=1` replacement frames** — correct per spec, but unverified on WezTerm, which is the
  terminal this change is for.
- **Reusing `PlacementNode::Caret`** with extra fields — drags the legacy solid block's meaning
  into flex and forces every legacy `Caret` construction site to change.
- **Deleting the caret image when Write mode ends** — needs the renderer to diff against its own
  previous frame; not worth it for one bounded image id.
- **`text.x + text.width` as the caret origin** — puts the caret one cell right of where typing
  lands when the label is empty, so it jumps left on the first keystroke.
