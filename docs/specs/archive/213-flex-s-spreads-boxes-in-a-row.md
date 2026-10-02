# Flex: s spreads the boxes in a row

## User Story

Doug starts a row box and adds two boxes inside. They share the width equally. He presses `s`. Each box shrinks to fit its content. The first box goes to the left edge and the second to the right edge. He presses `s` again, and the boxes share the width equally again.

## Acceptance Criteria

- With `s` off, the boxes inside a row share its width equally, as they do today.
- Pressing `s` on the row makes each box shrink to fit its content.
- With two boxes, the first sits at the left edge and the second at the right edge.
- With three boxes, the first sits at the left edge, the last at the right edge, and the middle one is placed so both gaps are equal.
- With one box, it shrinks to its content and sits at the left.
- Pressing `s` again returns the boxes to sharing the width equally.

## Technical Design

Only the layout changes. `s` already toggles `justify` between `Start` and `SpaceBetween`, so there is no new key, no new state and no change to `measure`, `distribute`, `paint` or the reducer.

### Decisions

- **`justify` alone decides the row's item lengths.** A row has no separate "shrink" flag. In `arrange` (`src/flex/view.rs`), `mains` becomes:
  ```rust
  let mains: Vec<i64> = match (direction, flex_box.justify) {
      (Direction::Row, Justify::Start) => share(inner.width, sizes.len(), FLEX_SPACE.width),
      _ => sizes.iter().map(|size| size.main(direction)).collect(),
  };
  ```
  - `Start` in a row: items share the width equally, as today.
  - `SpaceBetween` in a row: each item keeps its content width. `distribute` then does the spreading, unchanged: one item at the start, two items at both edges, three or more with equal gaps.
  - `Center` also keeps content widths, because it can't centre items that already fill the row. Only the window uses `Center`, and the window is a column, so nothing visible changes.
  - Columns are unaffected.
- **The text is spread as item 0.** A box's own text and its children are all items of the same row, so `s` spreads them together. There is no special case for the text.
- **Rounding stays as it is.** `distribute` splits the free space with `div_euclid` and gives the remainder to the last gaps, one unit each. With three items and an odd amount of free space, the two gaps can differ by 1 unit. The first item stays flush left and the last flush right.
- **Overflow is out of scope.** When the shrunk items are wider than the row, they overlap, as they do in a column today. There is no test for it.

### Collaborators

- `view::arrange`: the only function that changes.
- `view::distribute`: no change. It already handles the 1, 2 and 3+ item cases.
- `view::share`: no change. It is now called only for a `Start` row.
- `state::reduce`: no change. `s` already toggles `justify` and is already undoable.

### Testing plan (TDD, thin slices)

All tests are in `src/flex/view.rs`, next to the existing `justified_beside` tests.

1. **Two boxes spread.** In a row with `Justify::SpaceBetween` and two inner boxes, the first box starts at the row's inner left edge and the last box ends at its inner right edge. Each box is as wide as its content (`measure`), not half the row. This test fails first, and the `mains` change makes it pass.
2. **One box.** A row with `SpaceBetween` and one inner box: the box is as wide as its content and starts at the left edge.
3. **Three boxes.** Choose widths so the free space divides evenly. The first box is flush left, the last flush right, and the gap before the middle box equals the gap after it.
4. **Odd remainder.** Three boxes with an odd amount of free space. Both edges are still flush, and the two gaps differ by at most 1 unit.
5. **Text spreads with the children.** A row with its own text and two inner boxes under `SpaceBetween`: the text is flush left, the last box flush right, and the gaps are equal.
6. **`Start` is unchanged.** A row with `Justify::Start` and two inner boxes still gives each box half of the width. The existing `a_start_outer_box_keeps_its_texts_one_gap_apart` test keeps passing.
7. **Toggle back.** Press `s` twice on a row with `reduce` and check that the placements equal those from before the first press. This covers the "equal share again" acceptance criterion.
8. **Replace the old test.** `a_space_between_box_in_a_row_places_the_same_as_a_start_box` asserts the behaviour this spec removes. Delete it, because tests 1 and 6 cover both sides of the new rule.
