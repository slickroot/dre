## User Story

Doug draws a box, types "Login" into it, selects it, and presses `f` to fill it with colour. The fill appears behind his label — "Login" is still fully readable sitting on top of the coloured background. Satisfied, he keeps drawing.

## Acceptance Criteria

- In the terminal view, filling a labelled box leaves its label fully visible on top of the fill colour.
- This holds for every fill colour, not just some.

## Technical Design

**Root cause.** The terminal renderer doesn't composite pixels into one shared buffer — every box and every label glyph is sent to the terminal as its own Kitty graphics protocol image, positioned independently by cell (`Frame::place`, `src/render/terminal.rs:170`; `kitty::show`, `src/kitty.rs:25`). Stacking between overlapping images is decided by Kitty's `z` key, but `transmission()` (`src/kitty.rs:124`) hardcodes `z=-1` for every image regardless of what it's drawing. Box fills and label glyphs therefore share the same z-index, and Kitty is free to draw the fill on top of the text — which is what Doug sees.

**Fix.** Make `z` an explicit parameter instead of a hardcoded literal, threaded down the same path the canvas already travels:

- `kitty::transmission` and `kitty::show` take a `z: i32` argument and interpolate it into the header instead of the literal `-1`.
- `Placed` (`src/render/terminal.rs:128`) gains a `z: i32` field; `Frame::place` takes `z` as a parameter and stores it on the `Placed` it pushes; `Frame::into_bytes` passes `image.z` through to `kitty::show`.
- Each `draw_*` method on `TerminalRenderer` owns its own z-index literal at its `frame.place(...)` call site, mirroring how it already owns its shape/colour details:
  - `draw_box` passes `z: -2`.
  - `draw_label`, `draw_arrow`, and `draw_cursor` keep `z: -1` (unchanged from today).

This guarantees fills sit strictly behind labels for every fill colour, since the ordering is now a fixed property of the layer being drawn rather than an accident of transmission order. Arrow/cursor/label relative order is untouched — the story only requires text over fill.

No new tests: this is a constant threaded through existing plumbing, not new behaviour to characterize.
