# 069: Move centering out of layout

## Technical Design

### Why

`layout` currently conflates two concerns: computing the diagram's geometry, and positioning
that geometry within a canvas. The `cols`/`rows` parameters exist only because the terminal
view wants the diagram centred on screen. Export to SVG (spec 070) needs the identical
geometry with no canvas at all — fabricating a canvas to satisfy `layout` would invent a
constant the production code never had (today `cols`/`rows` always come from the live TTY
ioctl in `writer.rs`). Centering is a presentation concern, and each renderer already owns
its presentation surface: `TerminalRenderer` receives `cols`/`rows` and draws a full
terminal-sized grid; an SVG renderer will size its own page.

So `layout` drops its canvas entirely, and the centering moves into the one renderer that
actually uses it.

### `layout.rs`

`layout(&[Node]) -> Vec<Placement>` becomes pure, origin-based geometry, shared by both the
terminal and export:

- Column offsets start at 0 (no `left` shift).
- No `top` vertical shift.
- The boxes-first ordering is retained — `TerminalRenderer` and the SVG renderer both rely
  on it for draw order.
- An empty node list still returns `Vec::new()`.

The `cols` and `rows` parameters disappear.

### `render.rs`

`TerminalRenderer::render(&placements, cols, rows)` centres the origin-based placements
before drawing, using the identical math `layout` uses today:

- `left = (cols - span).div_euclid(2)` where `span = max(x + width) - min(x)` over
  placements — equal to the old summed column widths (in origin-based output `min(x) = 0`;
  the widest box/arrow right edge is the last column offset).
- `top = (rows - height).div_euclid(2)` where `height = max(y + height)` over placements —
  equal to the old `max_row_y + BOX_HEIGHT`: the lowest box is the lowest element, because
  labels sit within their box and arrow bottoms stop at the deepest child box's midline.
- Empty placements → no shift.

Per frame the renderer shifts a copy of the placements by `(left, top)` once, then feeds the
existing `grid` and `sprites` paths unchanged.

### `writer.rs`

`frame` calls `with_cursor(layout(&state.doc.boxes), selected)` — the `cols, rows` argument
to `layout` is gone; the renderer still receives them for centering and grid size.

### Test impact

- `layout.rs`: the `(80, 24)` fixtures become single-argument calls; anything asserting
  *centring* moves to the renderer tests.
- `render.rs`: new tests pin the centering — e.g. `render(layout(boxes), cols, rows)` output
  equals today's `layout(boxes, cols, rows)`-fed output for the same box set.
- `writer.rs`: expected frame output is unchanged.