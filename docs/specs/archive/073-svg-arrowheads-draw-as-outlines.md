# 073: SVG Arrowheads Draw as Outlines

## User Story

I can export my diagram to SVG and see arrowheads that look exactly like they do
in the Terminal — hollow outlines, not solid filled triangles.

## Acceptance Criteria

- The SVG arrowhead is drawn as two edge strokes only, with no solid fill inside
- Those two edge strokes are the same stroke thickness as the box borders currently have in the SVG
- The hollow arrowhead applies to every arrow, pointing left or right
- I can run `dre --svg` and open the result in the browser, and the arrowheads look just like they do in the Terminal render

## Technical Design

### Approach

Keep the existing `<marker id="arrowhead">` mechanism (`src/svg.rs` `marker_defs()`) and change only the marker's inner `<path>` from a filled triangle to two hollow edge strokes. The marker tips stay at the current `refX`/`refY` attach point, `orient="auto"` is retained (covers arrows pointing left or right — AC#3), and the geometry constants (`arrowhead_depth`, `arrowhead_slope`) remain shared with `src/render.rs`.

### Marker path

Replace the filled closed triangle

```
M {tip_x} {tip_y} L {base_x} {tip_y-arm} L {base_x} {tip_y+arm} Z   fill="rgb(0,0,0)"
```

with a single `<path>` element holding two open subpaths:

```
d="M {tip_x} {tip_y} L {base_x} {tip_y-arm} M {tip_x} {tip_y} L {base_x} {tip_y+arm}"
stroke="rgb(0,0,0)" stroke-width="{ARROW_STROKE/2}" fill="none"
```

- No `Z` and `fill="none"` → no solid fill inside the arrowhead (AC#1).
- `stroke-width` = `ARROW_STROKE / 2` (= 2, identical to the box border `BORDER/2` and the arrow trunk/arm strokes — AC#2). The arrowhead is part of the arrow, so its stroke mirrors the terminal (`ARROW_STROKE`) like the other arrow paths.
- `box_width`, `box_height`, `refX`, `refY`, and the `markerWidth`/`markerHeight`/`viewBox` geometry are unchanged from the current filled triangle. The resulting 1px stroke overhang at the two base corners is clipped by the marker viewport and accepted as invisible.

### Collaborators / unchanged surface

- `SvgRenderer::render`, `arrow_paths`, and the attribute order `d` → `stroke` → `stroke-width` → `fill` pin the emitted structure.
- `orient="auto"` continues to give left/right orientation.
- Arm/trunk/shadow `<path>`s and box `<rect>`s are untouched.

### Test criteria (TDD order)

1. Red: rewrite `the_arrowhead_marker_geometry_is_computed_from_the_mirrored_constants` to assert one `<path>` with exactly two `M tip L base` subpaths, no `Z`, `fill="none"`, `stroke="rgb(0,0,0)"`, `stroke-width="{ARROW_STROKE/2}"`.
2. Red: `colourless_box_strokes_stay_white...` currently asserts an ink `fill="rgb(0,0,0)"` that only the arrowhead triangle provides; switch it to assert the ink arrowhead as a stroke.
3. Update the golden `renders_the_spec_example_diagram_as_a_whole_document` through the shared `expected_marker()` helper (two-subpath `d`, `fill="none"`, `stroke`/`stroke-width`).
4. Green after `marker_defs()` body change; `orient="auto"`, box strokes, and the no-arrows test stay green unchanged.