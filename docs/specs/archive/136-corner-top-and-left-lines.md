## Story

Doug is editing `plans.dre` in the terminal. The bottom-right corner reads `plans • dre`. A thin line runs along its top and another along its left side, in the same colour as the text, so the corner looks like a small tab. Happy, he sees the name as a clear, separate label. With a brand-new diagram, the corner reads `[no name] • dre` and has the same two lines.

## Acceptance Criteria

- In the live editor, the bottom-right corner has a 1px line on its top edge and a 1px line on its left edge.
- The lines are drawn as graphics, like box borders, and in the foreground colour.
- The lines show for `<name> • dre` and for `[no name] • dre`.
- The SVG export is unchanged. It draws only the diagram.

## Technical Design

### Decisions

- The lines are a customised `Box`, not a new placement kind. `PlacementNode::Box` gets two more fields, `sides: Sides` and `border: i64`.
  - `type Sides = (bool, bool, bool, bool)`, in CSS order: `(top, right, bottom, left)`.
  - `const ALL_SIDES: Sides = (true, true, true, true)`. Every box the diagram layout produces uses it, so nothing changes for them.
  - `border` is the line thickness in pixels. The diagram layout sets it to `BORDER` (4), which moves from `render/mod.rs` to `layout.rs`. The footer box sets it to 1.
- `layout::footer(text)` returns two placements, both at `(0, 0)` and both as wide as the text and one row high:
  1. `Box { colour: None, fill: None, rounded: false, sides: (true, false, false, true), border: 1 }`. `colour: None` is the foreground colour.
  2. The `Label` with the text.
- The box covers exactly the label's cells. Its lines are the outer edge: the top pixel row and the left pixel column of the corner. The text is not inset.
- `render::editor` is unchanged. `align_right` moves both placements together. The lines show for `<name> • dre` and for `[no name] • dre`, because both go through `layout::footer`.
- The border sits on the outer edge of the box's cells. That is how `BoxShape` already works, and it does not change.
- `BoxShape` already takes `border`. It also gets `sides: Sides`. It draws the border only on the sides that are on. The other edges get no line, and the fill is unaffected.
  - With `ALL_SIDES` its output is identical to today, including rounded corners.
  - `rounded` with a partial `sides` is not used, so it is not defined. It behaves as square.
- Terminal renderer:
  - `SpriteKey::Box` gets `sides` and `border`, so boxes that differ in either do not share a cached sprite.
  - `outline_box` passes `sides` and the placement's `border` to `BoxShape`, instead of the `BORDER` constant.
- SVG renderer: `paint` draws only boxes with `ALL_SIDES`. A box with partial sides is skipped, so the SVG never draws a full rectangle around the corner. The export has no footer anyway (spec 135), and it stays unchanged.

### Call sites

- Every `PlacementNode::Box { .. }` construction gains `sides` and `border`: `layout.rs` (the diagram box and the tests), `render/mod.rs`, `render/svg.rs` and `render/terminal.rs` (test helpers).
- Patterns with `..` need no change.

### Tests

- `layout`: `footer("plans • dre")` is a box then a label. Both are 11 wide and 1 high. The box has `sides == (true, false, false, true)`, `border == 1`, no fill and no colour.
- `layout`: diagram boxes still have `sides == ALL_SIDES` and `border == 4`.
- `render`: `editor` ends with the footer box and label in the bottom-right corner, for a path and for no path (`[no name] • dre`). The box is aligned with the label.
- `render/shapes`: `BoxShape` with `(true, false, false, true)` has an opaque pixel at the top row and the left column. It has none at the bottom row or the right column, and none inside. With `ALL_SIDES` the output equals the current output.
- `render/terminal`: the footer box is drawn as an image with pixels on its top row and left column in the foreground colour. The line is 1px thick. Two boxes of the same size with different `sides` or `border` are two sprites in the cache.
- `render/svg`: a box with partial sides adds no `<rect>`. The existing box tests stay green.
- `cli`: `export` output is unchanged, with no footer and no corner lines.
