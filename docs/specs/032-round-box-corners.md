# 032 - Round box corners

## Story

Bob selects a box and presses `r` repeatedly. Its corners cycle through three
radii — 0px (square), then 10px round, then 20px round — and pressing `r`
again after 20px wraps back to 0px.

## Acceptance Criteria

- `r` cycles the selected box's corner radius through three levels in order:
  0px → 10px → 20px → back to 0px.
- A new box starts with 0px (square) corners.
- `r` only changes the selected box's corners; other boxes are untouched.
- The box's size and position stay exactly as they are today — only the
  drawn corners change.
- On a canvas with no boxes, `r` does nothing.
- In insert mode, `r` types the letter "r" into the label.

## Technical Design

### Radius

- `Box` (`state.py`) gains `radius: Literal[0, 10, 20] = 0`, the corner radius
  in pixels. It is an absolute pixel value, independent of `border`, so `t` and
  `r` each change one thing only.
- `handle_command` (`state.py`) gains an `"r"` branch mirroring `"t"`: the same
  `if not state.selected: return state` guard, then `rewrite` with
  `radius=(box.radius + 10) % 30`, giving 0 → 10 → 20 → 0. Insert mode needs no
  change — `r` falls through `handle_insert`'s printable-character branch.
- The radii are sized against the cell, not against the pixel grid in the
  abstract. A cell is on the order of 8 × 32 physical pixels, so a radius
  below about 8px clips less than a third of a cell's height and does not read
  as "rounded" at any border thickness. 10px and 20px are the smallest pair
  that are unambiguously round and clearly distinct from each other.
- `radius` names the **inner** arc. The outer arc is concentric at
  `outer = radius + border`, so the band keeps constant thickness `border`
  around the bend. `radius = 0` is special-cased to a fully square corner with
  no arc drawn at all, so a new box is square regardless of its border
  thickness.

### Coverage

- Corner geometry is the rounded-rectangle signed distance field. For a point
  `p` in box pixel coordinates, a box of size `W × H` and a corner radius `R`,
  let `half = (W/2, H/2)`, `q = abs(p - half) - (half - R)` and
  `sdf(p, R) = length(max(q, 0)) + min(max(q.x, q.y), 0) - R`, which is
  negative inside the shape and positive outside it.
- A pixel's coverage by a shape is `clamp(0.5 - sdf(centre, R), 0, 1)`, taken at
  the pixel centre `(x + 0.5, y + 0.5)`. This is the exact area for a straight
  edge and is within a fraction of a percent of it for the arcs at these radii.
- The border band is the difference of two such coverages: the outer shape is
  the box at radius `outer`, and the inner shape is the box inset by `border`
  on every side at radius `radius`. So
  `edge_coverage = outer_coverage - inner_coverage`, which is never negative
  because the inner shape is strictly contained in the outer one.
- There is **no supersampling**. An earlier draft averaged 4×4 subsamples per
  pixel, which quantises alpha to seventeen levels; along a one-pixel band that
  reads as stair-stepping in the alpha itself, independent of the geometry.
  Analytic coverage is continuous, and it costs one distance evaluation per
  pixel instead of sixteen.
- The two coverages address disjoint regions, so the pixel is their sum in
  **premultiplied** alpha, un-premultiplied before it is written:
  `a = edge_coverage * edge_a + inner_coverage * fill_a`, and
  `rgb = (edge_rgb * edge_coverage * edge_a + fill_rgb * inner_coverage *
  fill_a) / a`, or `(0, 0, 0, 0)` when `a` is zero. Straight averaging would
  drag the fringe toward black against `TRANSPARENT`.
- On the straight runs the distance field reduces exactly to today's
  `border`-inset rules and every coverage is 0 or 1, so the flat edges stay
  pixel-crisp with no fringe.

### Fill

- A filled box today is painted **twice**: `TerminalRenderer._draw_box` sets an
  SGR background on every cell of the box, and the sprite lays `PALETTE[fill]`
  at `FILL_ALPHA` on top of it. The muted fill colour is the combination of the
  two.
- That is incompatible with a rounded corner. The SGR background is a
  rectangle of whole cells and cannot be clipped to an arc, so it shows through
  exactly where the corner was cut away and the box reads as square however
  well the sprite is rasterised.
- So the fill moves entirely into the sprite. `TerminalRenderer._draw_box` no
  longer sets a background — it still claims the box's cells as blank so labels
  and cursors keep stamping over it. `_fill_colour` returns an **opaque**
  colour, `PALETTE[fill]` composited at `FILL_ALPHA` over black, which is what
  the two layers already produce on the default dark terminal. `PLAIN` stays
  fully transparent.
- The fill is therefore the same colour on every terminal instead of varying
  with the theme's ANSI palette. On a light-background terminal fills are
  darker than they used to be.

### Rasteriser

- `GraphicsRenderer._outline_box` (`render.py`) becomes row-based: it builds
  each row's profile from the radius rather than repeating two prebuilt rows.
  Coverage is evaluated only for pixels inside a corner band (`x < outer` or
  `x >= W - outer` on a row with `y < outer` or `y >= H - outer`); every other
  span is still a flat repeat of `edge` or `fill` bytes, so the cost stays
  proportional to the corners, not the box. `radius = 0` short-circuits to
  today's two-row path unchanged.
- `outer` is clamped to `min(radius + border, W // 2, H // 2)` so that the two
  corner bands in a row can never overlap. Without the clamp a box narrower
  than `2 * outer` emits more pixels than the row is wide and shears the rest
  of the sprite. The clamp is not reachable at the current `BOX_HEIGHT` and
  minimum box width, but it is one cell-size change away from being reachable
  and the failure is silent.
- Rows are computed in full-box coordinates, so the existing
  `first_x`/`last_x`/`first_y`/`last_y` screen clipping keeps working: a box
  half off-screen still gets the corners the whole box would have had.
- `_key`'s `shape` tuple (`render.py`) gains `node.radius`, so the sprite cache
  distinguishes the three radii. Each distinct shape is rasterised once and
  reused for every later frame.

### Cell size

- `cell_size` (`writer.py`) rounds to the nearest pixel rather than truncating.
  Truncation loses up to a full pixel per cell, and the sprite is sized as
  `cells * cell_width`, so the error accumulates across a box: a wide box ends
  up visibly narrower than the cells it covers and its right border drifts off
  the text grid. Rounding halves the worst-case error and centres it.
- The sprite is transmitted at its exact pixel size with no `c=`/`r=` keys
  (`kitty.py`), so the terminal blits it 1:1 and never resamples an already
  anti-aliased bitmap.
