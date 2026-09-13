# 035 - Arrows are drawn as boldly as boxes

## Story

Bob grows a child box and the arrow that connects it is drawn with bold 4px
lines, matching the weight of his box borders.

## Acceptance Criteria

- The arrow shaft, the vertical trunk and the arrowhead strokes are all drawn
  4px thick.
- The arrowhead keeps its current size and angle — only the stroke gets
  thicker.
- Arrows connect the same boxes at the same points as they do today.

## Technical Design

- Add a module-level constant in `sketch/render.py`, `ARROW_STROKE = 4`,
  independent of `Box.border` — arrows have no border field, so their
  thickness is not derived from any box's setting.
- `Canvas` stays stroke-width-agnostic: `horizontal(y, x0, x1, width)`,
  `vertical(x, y0, y1, width)`, and `point(x, y, width)` each take a required
  `width` parameter (no default) rather than baking in a fixed thickness,
  even though today `_outline_arrow` is their only caller and always passes
  `ARROW_STROKE`.
- Thickening rule, applied consistently everywhere a stroke is centred on a
  coordinate `c`: the stroke covers `c-1 .. c+2` inclusive (i.e.
  `range(c - 1, c + 3)`), biasing the extra pixel toward higher coordinates.
  - `horizontal(y, x0, x1, width)` thickens perpendicular to the line, i.e.
    across rows `y-1..y+2`, unchanged across `x0..x1`.
  - `vertical(x, y0, y1, width)` thickens across columns `x-1..x+2`,
    unchanged across `y0..y1`.
  - `point(x, y, width)` stamps a `width`×`width` square spanning
    `x-1..x+2` and `y-1..y+2`.
- `_outline_arrow` passes `ARROW_STROKE` to every `canvas.horizontal(...)`
  and `canvas.vertical(...)` call (shaft, trunk, and each stop's horizontal
  run).
- `_arrowhead` keeps its existing geometry untouched — `ARROWHEAD_DEPTH`,
  `ARROWHEAD_SLOPE`, and the per-distance `x`/`spread` calculation are
  unchanged — but each `canvas.point(x, stop_row ± spread)` call becomes
  `canvas.point(x, stop_row ± spread, ARROW_STROKE)`, stamping a 4×4 square
  per step instead of a single pixel. This keeps the arrowhead's size and
  angle identical while thickening its strokes.
- No changes to `stop_rows`, `shaft_row`, `trunk_top`/`trunk_bottom`, or
  `midpoint` — connection points and arrow endpoints are unaffected.
