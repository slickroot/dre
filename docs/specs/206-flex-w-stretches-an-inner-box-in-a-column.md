# Leila stretches an inner box across its parent with w

## User Story

Leila has a full-width box in dre-flex, stacked as a column, with the text "Title" and an inner box "Hi" inside it. The inner box is only as wide as "Hi". In move mode she selects the inner box and presses `w`. It stretches from the left inside edge of its parent to the right inside edge, keeping the parent's padding. Happy, she carries on drawing!

## Acceptance Criteria

- In move mode, when the parent stacks its children in a column, `w` on an inner box stretches it to the parent's inside width and keeps the parent's padding on both sides.
- Pressing `w` again shrinks the inner box back to fit its content.
- When the parent lays its children out in a row, `w` on an inner box changes nothing on screen.
- If Leila presses `w` on an inner box while its parent is a row and then switches the parent to a column with `d`, the inner box shows full width.

## Technical Design
All changes are in `src/flex/state.rs` and `src/flex/view.rs`. Move mode needs no new keys. `w` already toggles the width of any selected box, inner boxes included, and the flag survives `d` on the parent. Only the layout ignores it today.

The work comes in two parts. First a refactor: **the window is the root box**, and the separate window code (`lay_out_window`) is removed. Then the story itself: **a `Full` box stretches across its parent's cross axis**. Because the window is just a box, that one rule in `arrange` covers both full-width outer boxes (already shipped, spec 190) and Leila's inner box.

### State (`state.rs`)

- **`FlexWidth` becomes `FlexSize { Fit, Full }`**, with the same `toggle`.
- **`FlexBox`** gains two fields:
  - `height: FlexSize`: `Fit` by default. No key changes it. Only the window box uses `Full`.
  - `bare: bool`: `false` by default. A bare box has no padding and no border.
- **`Justify`** gains `Center`, which starts the children so that the free room is split evenly before and after them. `g` never reaches it. `Center.toggle()` returns `Start`.
- **`FlexBox::default()`** is unchanged: Leila's box (`Fit`, `Fit`, `Row`, `Start`, not bare, not filled). `new_box()` is unchanged.
- **`FlexBox::window()`** builds the root: `width: Full`, `height: Full`, `direction: Column`, `justify: Center`, `bare: true`.
- **`impl Default for FlexNode`** returns `FlexNode::Box(FlexBox::window())`. Every `Tree::root(...)` (in `FlexState::default()` and the test helpers) therefore gets the window as its root without changes.
- The window can't be selected. `Tree::parent` stops at the outer boxes, so `w`, `g`, `d` and `f` never touch it.

### View (`view.rs`)

- **Padding is per box.** `measure` and `Rect::inner` pad by `FLEX_SPACE`, or by nothing for a `bare` box.
- **`distribute`** handles `Justify::Center`: it uses the normal gap, and the first offset is `(room - total) / 2`, where `total` is the lengths plus the gaps.
- **The stretch rule, in `arrange`:** after measuring a child, if it's a box that is `Full` on the parent's cross axis, its cross size becomes the parent's inside cross size and its cross offset is 0:
  - In a `Column` parent, a `width: Full` child is as wide as `inner.width`, keeping the parent's padding on both sides.
  - In a `Row` parent, a `height: Full` child is as tall as `inner.height`. Only the window could use this today, and its parent isn't a row. A `width: Full` child in a row is laid out at its measured size, so `w` changes nothing on screen.
  - `measure` stays content-only. A `Full` child never makes its parent bigger. It fills whatever room the parent already has.
- **Cross centring** stays `(room - size) / 2` for every box, so odd leftovers lean left. That now includes the outer boxes in the window.
- **`scene`** arranges the root straight into the window: `arrange(&state.boxes, &[], window_rect, &mut out)`. Painting skips `bare` boxes, so the window draws nothing (and `depth` never sees the empty root path). Outer boxes still get `solid_fill` through `path.len() == 1`.
- **`lay_out_window` is deleted.** Its jobs are now ordinary box rules: column stacking comes from the root's `Column`, vertical centring from `Center`, horizontal centring from cross centring, and a full outer box spanning the window from the stretch rule with no padding.

### Visible changes from the refactor

- Outer boxes line up on the window's centre using the same lean-left rounding as inner boxes, rather than centring the stack and then lining up centres leaning right. Depending on the widths, the stack can move by 1 cell.

### Tests

Refactor (all existing view tests should pass, except one):
- `odd_widths_lean_right` becomes `odd_widths_lean_left`.
- `FlexNode::default()` is the window box: full both ways, a centred column, bare.
- No placement is produced for the root. `only_the_box_and_its_label_are_placed` already covers this.
- `distribute` with `Center` splits the free room evenly, with odd leftovers leaning left.

Story (one view test per acceptance criterion; outer box `Full`, `Column`, holding "Title" and an inner box "Hi"):
- After `w` on the inner box, it spans from `outer.x + FLEX_SPACE.width` to `outer.x + outer.width - FLEX_SPACE.width`.
- After `w` twice, it's back to its measured width.
- In a `Row` parent, `w` on the inner box gives the same placements as before the `w`.
- `w` on the inner box while the parent is a row, then `d` on the parent: the inner box is full width.
