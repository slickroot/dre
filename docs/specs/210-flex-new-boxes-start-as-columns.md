# Flex: new boxes start as columns

## User Story

Lina opens `dre-flex` and presses `A` twice. The two new inner boxes stack top to bottom, one under the other. Her boxes are columns from the start, and she carries on drawing!

## Acceptance Criteria

- Every new box starts as a column. That includes the box shown on launch, boxes added with `a` and boxes added with `A`.
- When `dre-flex` opens, pressing `A` twice stacks the two inner boxes top to bottom.

## Technical Design

This changes spec 197, where new boxes started as `Row`. The layout from specs 197 and 208 already handles columns, so only the default changes.

### State (`src/flex/state.rs`)

- Move `#[default]` on `Direction` from `Row` to `Column`. A new box is a column because the type's default says so.
- `new_box()` doesn't change. It still builds `FlexBox::default()`. The launch box (`FlexState::default()`), `a` and `A` all go through it, so all three now start as columns.
- `FlexBox::window()` drops its explicit `direction: Direction::Column`, because the default already gives it.

### Layout (`src/flex/view.rs`)

- No production change.

### Test fixtures

- Each test module (`state.rs`, `view.rs`) gets a test-only helper: `fn row() -> FlexBox { FlexBox { direction: Direction::Row, ..FlexBox::default() } }`.
- Use `row()` only at the call sites whose tests break because of the flip, so each of those tests says "row" out loud.
- Fixtures that don't care about direction keep `FlexBox::default()` and now build columns, which is the new truth.

### Collaborators

- `types::Tree<FlexNode>`: no change.
- Only `src/flex/state.rs` changes in production. `src/flex/view.rs` only changes in its tests.

### Testing plan (TDD, state only)

1. `starts_with_a_row_direction` becomes `starts_with_a_column_direction`. The launch box is `Direction::Column`.
2. `new_outer_and_inner_boxes_start_as_rows` becomes `new_outer_and_inner_boxes_start_as_columns`. It covers `a` and `A` and asserts `Direction::Column`. Drop the leading `d` from its key sequence so it starts from the real default.
3. Acceptance: from `FlexState::default()`, pressing `A`, `A` gives a selected box that is a `Column` and holds two inner boxes. Stacking top to bottom follows from the column layout tests from spec 197, so there's no view test here.
4. Flip the `d` expectations that started from a default row:
   - `d_in_move_mode_turns_the_box_into_a_column_and_keeps_the_rest` becomes `..._into_a_row_...`.
   - `d_twice_...` toggles back to column.
   - `d_in_write_mode_...` leaves the direction as `Column`.
   - `d_only_changes_the_direction_of_the_selected_box` expects `Column`/`Row`.
   - `d_on_a_selected_text_turns_its_parent_box_into_a_column` becomes `..._into_a_row`.
5. `the_default_node_is_the_window_a_bare_centred_column` still passes unchanged after `window()` drops its direction.
6. Run the whole suite. Fix each row-dependent test in `state.rs` and `view.rs` with `row()` at the call site, without changing its assertions.

### Out of scope

- Any change to how rows or columns lay out.
- Removing `FlexBox::default()` from fixtures that don't depend on direction.
