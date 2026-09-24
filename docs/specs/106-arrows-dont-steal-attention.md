# 106: Arrows don't steal attention

## User Story

Doug opens a diagram in dre. The arrows are drawn as soft, thin lines instead
of bright, thick ones, so the boxes and their labels are the first things Doug
sees. Doug can follow the arrows from box to box without them pulling the eye
away, and gets on with the diagram.

## Acceptance Criteria

- Arrows are drawn in the foreground colour at 50% opacity, both in the
  terminal and in the exported SVG.
- Arrows are thinner than box borders: 3px in the terminal, down from 4px, and
  1px in the exported SVG, down from 2px (half the SVG box border).
- The colour and thickness apply to the whole arrow: the shaft, the branching
  lines to several children, and the arrowhead.

## Technical Design

### Shared arrow opacity

- `render/mod.rs` gains `ARROW_OPACITY: f64 = 0.5`, next to the other shared
  render constants. The arrow colour stays `colour(None)`, the foreground.
- Thickness is not shared. Each renderer has its own `ARROW_STROKE`.

### Terminal renderer (`render/terminal.rs`, `render/shapes.rs`)

- `ARROW_STROKE` becomes 3px, down from 4px (`BORDER`).
- `ArrowShape.ink` gets an alpha of `(ARROW_OPACITY * OPAQUE as f64).round()`,
  which is 128, instead of `OPAQUE`.
- `ArrowShape::colour_at` already returns one colour per pixel, so the shaft,
  trunk, stop arms and arrowhead overlap without darkening. It needs no change.
  The stroke already drives every part of the arrow, including the arrowhead.

### SVG renderer (`render/svg.rs`)

- `ARROW_STROKE` becomes `BORDER / 4` (1px). The SVG box stroke is `BORDER / 2`,
  so this is half of it.
- `ARROW_JOIN_OVERLAP` stays `ARROW_STROKE / 2`, which is now 0. The group
  opacity replaces it for hiding seams. Butt caps at 1px still land inside the
  perpendicular line, so there are no gaps at the joints.
- `arrow_paths` wraps one arrow's paths in `<g opacity="0.5">...</g>`, using
  `ARROW_OPACITY`. Group opacity composites the arrow as a single shape, so
  overlapping paths and the arrowhead marker are not darker at the joints.
- Per-path stroke and marker styling in `marker_defs` stay as they are. The
  marker is rendered as part of its path, so the group covers it.
- One group per arrow, not one for the whole diagram. Different arrows don't
  overlap.

### Tests

- Terminal: an arrow pixel has alpha 128 and the foreground RGB. A pixel where
  the shaft meets the trunk has the same colour as a pixel on the shaft alone.
  The arrow is `ARROW_STROKE` thick, thinner than a box border and thicker
  than 1px.
- SVG: every arrow's paths sit inside a `<g opacity="0.5">`. Arrow paths use
  `stroke-width="1"`. Box strokes are unchanged.
- Update the existing SVG arrow tests that build expected paths from
  `ARROW_STROKE` and `ARROW_JOIN_OVERLAP`.
- Regenerate `docs/example.svg` and `docs/architecture.svg` so they show the
  new arrows.
