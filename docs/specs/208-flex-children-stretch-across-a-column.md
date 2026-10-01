# Leila's boxes stretch across their column

## User Story

Leila opens dre-flex. Her first box already spans the full width of the window. She presses `a` to add a box below it, and that box spans the window too. Happy, she carries on drawing!

## Acceptance Criteria

- Every outer box spans the full width of the window, with no key pressed.
- In a box set as a column, every inner box stretches to the parent's inside width.
- In a box set as a column, every text stretches to the parent's inside width, and its words sit on the left.
- In move mode, `w` does nothing.

## Technical Design
All changes are in `src/flex/state.rs`, `src/flex/history.rs` and `src/flex/view.rs`. Stretching becomes a **layout rule of `Column`**, not a flag on a box. Every child of a column, box or text, gets the column's full inside width. The window is already the root `Column` (spec 206), so outer boxes span the window with no special case. The per-box width flag and the `w` key go away.

### State (`state.rs`)

- **`FlexSize` is removed**, together with `FlexBox`'s `width` and `height` fields. The window's `height: Full` was never read, because the root is never laid out as a child.
- **`FlexBox::window()`** keeps `direction: Column`, `justify: Center` and `bare: true`.
- **The `"w"` arm of `move_key` is removed.** `w` falls through to `_ => {}`, so in move mode it does nothing. In write mode it still types a "w".

### History (`history.rs`)

- `"w"` leaves the key list in `undoable`, so a stray `w` doesn't push a snapshot. In the tests, it moves from the undoable list to the not-undoable list.

### View (`view.rs`)

- **`is_full_across` is removed.** In `arrange`, the cross size and offset of each child depend only on the parent's direction:

  ```rust
  let (cross, cross_offset) = match direction {
      Direction::Column => (inner_cross, 0),
      Direction::Row => {
          let cross = size.cross(direction);
          (cross, (inner_cross - cross) / 2)
      }
  };
  ```

- **Rows are unchanged.** Their children keep their measured height and are centred on the cross axis.
- **`measure` is unchanged.** Stretching happens only in `arrange`, so a box still measures to fit its content, and its parent doesn't grow because of it.
- **Text stays on the left.** The terminal draws a label from `rect.x`, so a stretched text rect keeps its words at the inside left edge with no extra work.
- **Padding (spec 207)** still works through `rect.inner(padding(..))`. A stretched box keeps its stretched width, and extra side padding takes room from its content.

### Tests

State:
- Delete the tests about `w` toggling `width` (default `Fit`, `w` sets `Full`, `w w` returns to `Fit`, `w` in write mode, `w` then `u`).
- `w` in move mode leaves `boxes` and `selected` unchanged.
- `"w"` is in the not-undoable keys for move mode (`history.rs`).

View:
- Rewrite the spec-190, spec-206 and spec-207 tests that set `FlexSize::Full` or press `w`, so they rely on the column rule instead.
- With no key pressed, the first outer box spans the window's full width, and so does a second box added with `a`.
- In a column box, an inner box's width equals the parent's inside width (parent width minus `2 * FLEX_SPACE.width`).
- In a column box, a text's rect has the parent's inside width, and its `x` is the inside left edge.
- Regression: in a row box, children keep their measured width and are centred vertically.
