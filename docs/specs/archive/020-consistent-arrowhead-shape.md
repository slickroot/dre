# 020 - Consistent arrowhead shape

As a user viewing a diagram with arrows connecting boxes, I want every
arrowhead to have the same size and shape regardless of whether the arrow
points up, down, left, or right, so that arrows look visually consistent no
matter their direction.

## Acceptance Criteria

- An arrowhead's edges are drawn at 30° from the shaft, the same angle for
  up, down, left, and right arrows.
- An arrowhead's edges are 15px long, the same length for up, down, left, and
  right arrows.
- The shaft (the line from the arrow's start to the base of the head) is
  unaffected by this change.

## Technical Design

Scope: `PixelRenderer._outline_arrow` / `_on_arrow` in `sketch/render.py` only.
`TerminalRenderer` draws arrows as single Unicode glyphs (↓↑←→) and is
unaffected.

Add two module-level constants next to `ARROW_GLYPHS`:

- `ARROWHEAD_ANGLE_DEG = 30`
- `ARROWHEAD_EDGE_LENGTH = 15` (px)

`_on_arrow` currently computes the head shape as a ratio of `reach`
(space to the box edge) over `thickness` (the cell size, `cell_width` for
vertical arrows / `cell_height` for horizontal arrows). Because
`cell_width != cell_height`, this makes vertical and horizontal arrowheads
different shapes — the bug this story fixes.

Replace the ratio with fixed trigonometry derived from the two constants:

- `depth = ARROWHEAD_EDGE_LENGTH * cos(radians(ARROWHEAD_ANGLE_DEG))` — how
  far back from the tip the head extends along the shaft.
- `spread(distance) = round(distance * tan(radians(ARROWHEAD_ANGLE_DEG)))`
  — how far the edge sits from the centreline at a given distance from the
  tip.

`in_head` becomes `along`-distance-from-tip `< depth` (replacing
`thickness`). `reach` and `thickness` are no longer used to shape the head;
`thickness` is still used to size/centre the shaft. No explicit clamping
for narrow boxes is added — the existing pixel-loop bounds clip any
overflow if the head's ~7.5px half-width exceeds the shaft's one-cell
thickness.

The shaft line (`across == centre`) is drawn unconditionally, unchanged.

The head remains an outline (the two edge lines), not a filled triangle,
matching the current `_outline_arrow` behaviour.
