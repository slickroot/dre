# SVG brackets match the border thickness

## User Story

Marouane selects a box in an SVG diagram. The corner brackets around it are as thick as the box's own border, so the selection looks like it does in the terminal.

## Acceptance Criteria

1. In an SVG with a selected box, the brackets are the same thickness as the box border.
2. The bracket arm length and the gap between the brackets and the box stay as they are today.

## Technical Design

### Root cause

`PlacementNode::Brackets { border }` carries the terminal thickness in device pixels (`BORDER` = 4). `svg::rect` converts it for the box stroke (`BORDER / 2` = 2), but `svg::brackets_path` uses it raw, so the SVG brackets are twice as thick as the SVG border. It also reuses the terminal's `BRACKET_OFFSET` (8), which was tuned for a 4-pixel bracket.

### The rule to mirror

In the terminal the gap between the box border and the bracket equals the bracket thickness (offset 8 - thickness 4 = 4). The SVG keeps that relationship at its own scale.

### Change (confined to `brackets_path` in `src/render/svg.rs`)

- `stroke = border / 2`, the same conversion `rect` applies to the box border. This is the bracket thickness.
- The gap between the visible border and the bracket equals `stroke`.
- The SVG box stroke is centred on the rect edge, so the visible border reaches `stroke / 2` outside it. The bracket's outer corner therefore sits `stroke / 2 + stroke + stroke` from the rect edge (1 + 2 + 2 = 5 for `BORDER` = 4).
- The arm still measures `BRACKET_ARM` (14) from the outer corner. Arm length is unchanged.
- The polygon (`h arm v stroke h -inner_arm v inner_arm h -stroke z`) keeps its shape and uses `stroke` where it used `border`.
- `svg.rs` no longer uses `BRACKET_OFFSET`. It stays in `render/mod.rs` for the terminal renderer.
- The terminal renderer, `PlacementNode::Brackets`, and the layout are untouched.

### Tests (`src/render/svg.rs`)

- Update `the_brackets_sit_at_the_offset_from_the_box_corners_with_the_arm_and_border`: thickness is `BORDER / 2`, and the outer corner is `stroke / 2 + 2 * stroke` from the box edge.
- Add a test that the gap (bracket inner edge minus the border's visible outer edge) equals the bracket thickness, and that the thickness equals the `stroke-width` on the box `rect`.

### Out of scope (follow-up refactoring specs)

- A `Thickness` newtype, so a renderer can't use a terminal thickness without converting it. Today the 1/2 ratio is repeated by hand at each call site.
- Deriving the terminal thickness from the real cell size, so the terminal and SVG match on non-Retina displays. Spec 147 already declined to add a scale factor.
