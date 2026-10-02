# Fix row-direction defaults in spec 213's tests

## Problem

Three tests added by spec 213 build their row fixtures from `FlexBox::default()`
(directly, or via `spreading()`) without setting `direction: Direction::Row`.
Spec 210, merged on top of the branch spec 213 was written against, changed
`Direction::default()` from `Row` to `Column`. The tests now build columns
instead of rows:

- `a_space_between_row_spreads_two_boxes_to_its_inner_edges`
- `a_space_between_row_with_one_box_keeps_it_its_width_at_the_left_edge`
- `a_start_row_gives_each_of_two_boxes_half_of_its_width`

Three other spec 213 tests that also go through `spreading()` happen to keep
passing, but only by coincidence: in a column, `arrange` gives every item the
full cross width (src/flex/view.rs:218-224), so their x/width assertions hold
even though the fixture is no longer a row.

## Technical Design

No production code changes. Only the test fixtures in `src/flex/view.rs` are
wrong.

### Decisions

- **`row_of_boxes` forces `direction: Direction::Row`.** All seven call sites
  of `row_of_boxes` (src/flex/view.rs:874) pass `spreading()` or
  `FlexBox::default()` and always intend a row; none intend a column. Rather
  than require every caller to remember to set `direction`, `row_of_boxes`
  sets it itself:
  ```rust
  fn row_of_boxes(row: FlexBox, own_text: &[&str], box_texts: &[&str]) -> FlexState {
      let row = FlexBox {
          direction: Direction::Row,
          ..row
      };
      ...
  }
  ```
  This fixes `a_space_between_row_with_one_box_keeps_it_its_width_at_the_left_edge`
  and `a_start_row_gives_each_of_two_boxes_half_of_its_width`, and keeps the
  other `row_of_boxes`-based tests passing for the right reason instead of by
  coincidence.
- **`a_space_between_row_spreads_two_boxes_to_its_inner_edges` is fixed at its
  call site, not by changing `outer`.** This test builds its fixture directly
  with `outer(FlexBox { justify: Justify::SpaceBetween, ..FlexBox::default() }, &[], 2)`.
  `outer` itself is used by many non-row tests and must not be forced to a
  direction. Instead, the test starts from the existing `row()` helper
  (src/flex/view.rs:340), which already sets `direction: Direction::Row`:
  ```rust
  outer(FlexBox { justify: Justify::SpaceBetween, ..row() }, &[], 2)
  ```

### Collaborators

- `src/flex/view.rs` test module only: `row_of_boxes`, and the one
  `a_space_between_row_spreads_two_boxes_to_its_inner_edges` test. No change
  to `arrange`, `distribute`, `share`, `measure`, or `state::reduce`.

### Testing plan

No new tests. The three failing tests above must pass unchanged (their
assertions already encode spec 213's acceptance criteria correctly); only
their fixtures move from an implicit column to an explicit row.
