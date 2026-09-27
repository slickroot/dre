# Box fill drops to 12% opacity

Priya draws a diagram with a filled box in dre. Whether she views it in the terminal or exports it to SVG, the box's fill now shows at 12% opacity instead of 30%, giving it the same lighter, subtle look the footer already has. She's happy with how much cleaner it looks and keeps working.

## Acceptance Criteria

- Regular box fill opacity is 12% (down from 30%) in the terminal renderer.
- Regular box fill opacity is 12% (down from 30%) in SVG export.
- Footer fill opacity is unaffected (stays at 12%, same as before).

## Technical Design

Box fill opacity is a single shared constant, `BOX_FILL_OPACITY`, defined in `src/render/mod.rs:115`. It's consumed by `src/layout.rs` when constructing filled box shapes, and both the terminal renderer (`src/render/terminal.rs`) and the SVG renderer (`src/render/svg.rs`) render whatever opacity value is attached to a shape by layout — neither renderer has its own separate box-opacity logic. So a single-line change to this constant (`0.3` → `0.12`) propagates to both output formats.

`FOOTER_FILL_OPACITY` (already `0.12`, `src/render/mod.rs:116`) stays a separate constant — `BOX_FILL_OPACITY` and `FOOTER_FILL_OPACITY` are not aliased to each other, they just happen to hold the same value now.

Tests to update:
- `src/layout.rs` — `a_filled_coloured_box_has_the_box_fill_opacity` (still valid, just now asserts `0.12`)
- `src/render/terminal.rs` — `fill_colour_of_the_footer_opacity_is_dimmer_than_a_regular_box_fill`: this test's premise (box opacity > footer opacity) is no longer true and should be removed or rewritten to assert they're now equal
- `src/render/svg.rs` — `a_filled_all_sides_box_uses_the_box_fill_opacity` derives its expected value from the constant, so it should keep passing unchanged
