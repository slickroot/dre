# Flex box has a thin border

## User Story

Lina runs `dre-flex`. She sees one empty box on the canvas with a thin 1-pixel outline. She types "Hello", and the box grows while the outline stays thin. She presses Backspace, and the box shrinks with the same thin outline. Happy with how light it looks, she presses Ctrl-C to leave.

## Acceptance Criteria

- When `dre-flex` starts, the empty box has a 1-pixel border on all four sides.
- The border stays 1 pixel as the box grows while Lina types and shrinks when she presses Backspace.
- Boxes in the main `dre` editor keep their current thickness.

## Technical Design

Decisions:

- **Only the view changes.** `border` in `PlacementNode::Box` is already a pixel inset. The terminal renderer copies it straight into `BoxShape` (`render/terminal.rs` `box_shape`), and the footer's naming box already draws with `border: 1`. The flex box just passes `1` instead of `view::BORDER` (4). `render`, `FlexState`, `reduce` and `run_loop` are unchanged.
- **A local constant, `FLEX_BORDER`, in `flex::view`.** It's not a shared `view::THIN_BORDER`. The flex box's look (this border, and the colours from 185) lives in `flex::view`, so nothing in the main `dre` editor can change. `view::BORDER` and the footer's literal `border: 1` are left alone. Refactoring those is out of scope.
- **The thickness doesn't depend on the text.** The box width comes from `interior(text) + 2` cells, and `FLEX_BORDER` is a constant, so the border stays 1 pixel as the box grows and shrinks.

### `flex::view`: what changes

```rust
const FLEX_BORDER: i64 = 1;

pub(crate) fn scene(state: &FlexState, window: Area) -> Scene<'_>
```

- The box placement uses `border: FLEX_BORDER`, with `ALL_SIDES`, not rounded, and the same width and height as before. `BORDER` is dropped from the `crate::view` import.

### Tests

`scene` unit tests in `flex::view`:

- An empty box (`""`) is a `Box` with `sides == ALL_SIDES` and `border == 1`.
- The box for `"Hello"` (width 7) and the box after Backspace from `"Hellp"` both have `border == 1`.

Main editor guard:

- The existing `layout::tree` test `diagram_boxes_have_all_sides_and_the_border_width` still asserts `(ALL_SIDES, BORDER)` for diagram boxes. That covers "boxes in the main `dre` editor keep their current thickness", so no new test is needed there.
