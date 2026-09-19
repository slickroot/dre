# 074: Arrow Elbow Meets Flush

## User Story

When I view an SVG diagram with an arrow that turns a corner at the elbow, the
two line segments meet flush so the corner looks like one clean, continuous
bend.

## Acceptance Criteria

- At the elbow, the vertical stroke no longer sticks out past the horizontal line on the outer side of the corner — the outer edge is flush
- There's no visible gap between the two lines at the corner — they clearly connect

## Technical Design

The defect: `SvgRenderer::arrow_paths` (src/svg.rs:89-120) draws the arrow as
separate `<path>`s with the default `stroke-linecap="butt"`. Where a horizontal
stroke (stop arm, or the shaft) ends flush against the vertical trunk, neither
stroke covers the diagonal 1px corner square, so the outer corner is stepped
and a tiny gap appears. The terminal renderer is unaffected (its canvas fills
solid blocks) and is out of scope.

Decision: stroke overlap. Only `src/svg.rs` changes; `src/render.rs` is untouched.

### Components

- `SvgRenderer::arrow_paths` (src/svg.rs:89-120) — the only behavior change.

### Responsibilities / State

svg.rs owns its rendering constants, decoupled from render.rs:

- `const ARROW_STROKE: i64 = 2;` — SVG's own arrow weight, matching the SVG box
  border weight `BORDER / 2`. Dropped from the `crate::render` import; all
  `stroke-width="{ARROW_STROKE / 2}"` attributes (shaft, trunk, arms, arrowhead
  marker outline) become `stroke-width="{ARROW_STROKE}"`. SVG output is
  otherwise byte-identical.
- `const ARROW_JOIN_OVERLAP: i64 = ARROW_STROKE / 2;` — the flush seam (= 1),
  half the drawn stroke width.

### Behavior

In `arrow_paths`, extend only the trunk-adjacent end of each horizontal stroke
by the seam, so its butt edge reaches the trunk's far edge and fills the corner:

- each stop arm's left end: `trunk_x` → `trunk_x - ARROW_JOIN_OVERLAP`
- the shaft's right end: `trunk_x` → `trunk_x + ARROW_JOIN_OVERLAP`
- the trunk stays `trunk_top..trunk_bottom` (never extends past the outer
  stop-arm edges, so the vertical stroke cannot stick out past the horizontal)
- non-adjacent ends unchanged: the shaft's left end (hidden behind the parent
  box, drawn after arrows) and each arm's right end (covered by the arrowhead
  marker)

### Dependencies / Collaborators

- `arrowhead_depth`, `arrowhead_slope`, `colour`, `CELL_WIDTH`, `CELL_HEIGHT`
  still come from `crate::render`; only `ARROW_STROKE` moves to svg.rs.
- `marker_defs` uses the same local `ARROW_STROKE` for the arrowhead outline.

### Tests

- Update the existing exact-string tests in svg.rs (the `arrow_at` helper, the
  two-stop arrow test, the specimen golden test) to the seam coordinates, and
  use the local constants in the test module.
- Add one focused test asserting the flush contract: shaft right end ==
  `trunk_x + ARROW_JOIN_OVERLAP`, each stop arm left end ==
  `trunk_x - ARROW_JOIN_OVERLAP`, and the trunk still spans
  `trunk_top..trunk_bottom`, proving flush corners with no trunk protrusion.