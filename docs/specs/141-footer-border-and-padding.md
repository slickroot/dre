# Footer border and padding

Doug is using dre in a Kitty-graphics-capable terminal. He glances at the footer in the bottom-right, and now sees it fully boxed in with a border on all four sides (matching the existing top/left border style), with the text sitting 8px away from each edge instead of flush against it. Satisfied with the clearer, more polished look, he keeps working.

## Acceptance Criteria

- The footer has 1px borders on all four sides (top, right, bottom, left), using the same style/color as the existing top/left border.
- The footer text has 8px of padding between it and each of the four borders.
- This applies only to the interactive terminal rendering (via Kitty graphics); SVG export/rendering is unchanged.

## Technical Design

The footer's shape is built by the shared `layout::footer()` (`src/layout.rs:226-248`), consumed by both the terminal (Kitty) and SVG rendering paths via `render::editor()`. We will edit it in place; SVG's footer rendering is allowed to change as a side effect since it shares the same layout data.

Changes to `layout::footer()`:
- The footer's `PlacementNode::Box` gets `sides: (true, true, true, true)` (all four sides) instead of `(true, false, false, true)`. `border` stays `1`, `colour: None` (foreground) — both already match the existing top/left style, so no separate styling work is needed for the new sides.
- Reserve a 1-cell margin around the text on all sides, mirroring the existing pattern used for diagram-node boxes (which reserve cells around their label via `BORDERS`, `src/layout.rs:8-9`). Add a new constant, e.g. `FOOTER_MARGIN: i64 = 1`, distinct from `BORDERS` (which is `2`, sized for diagram nodes).
- Box `width` becomes `text.chars().count() + 2 * FOOTER_MARGIN`; box `height` becomes `1 + 2 * FOOTER_MARGIN` (replacing the current `FOOTER_ROWS: i64 = 1`, which becomes `3`).
- The `Label` placement is offset by `FOOTER_MARGIN` in both x and y relative to the box's origin, so it sits one cell in from each border instead of flush against it.

No pixel-level offset math is introduced: the reserved 1-cell margin is treated as "close enough" to the 8px padding requirement, the same way the existing border rendering already accepts imprecision between logical cell reservations and actual per-terminal pixel dimensions. Both `Placement` and `PlacementNode::Box` keep their existing fields — no new padding/inset field is added; the margin is expressed purely as extra reserved cells, consistent with how diagram-node borders already work.

Downstream effects (already acceptable per the above):
- `render::mod.rs`'s `composer::stack([None, Some(FOOTER_ROWS)], window)` (`src/render/mod.rs:25-38`) will reserve 3 rows for the footer instead of 1, shrinking the body area by 2 rows.
- SVG rendering (`src/render/svg.rs`) will render the larger, fully-bordered, padded footer box too, since it consumes the same `layout::footer()` output.
- The existing terminal test asserting the footer box is a "one-pixel line along its top and left" (`src/render/terminal.rs:1216`) will need updating to reflect all four sides.
