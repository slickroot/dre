# Flex: spacing is one unit everywhere, 2 columns across and 1 row down

## User Story

Lina runs `dre-flex` and presses `A` to add an inner box. It sits inside the outer box with 2 columns of space on its left and right and 1 row above and below. The space looks even on all sides. She presses `A` again, and the second inner box appears 2 columns to the right of the first. She switches the box to a column, and the inner boxes stack with 1 row between them. Wherever she looks, the space between things is the same: 2 columns sideways, 1 row up and down. The diagram no longer looks cramped.

## Acceptance Criteria

- Inside a box, there are 2 columns of space between its left and right borders and what it holds.
- Inside a box, there is 1 row of space between its top and bottom borders and what it holds.
- Siblings side by side in a row are 2 columns apart by default.
- Siblings stacked in a column, and stacked outer boxes, are 1 row apart by default.
- Nowhere are two things closer than 2 columns side by side or 1 row stacked, including when siblings are spread out.

## Technical Design

All the change is in `src/flex/view.rs`. `state.rs` is untouched.

### One spacing constant

`FLEX_GAP` is replaced by a single constant that reuses the existing `Size` type:

```rust
const FLEX_SPACE: Size = Size { width: 2, height: 1 };
```

It is both the padding inside a box and the default gap between siblings. Padding and gap are deliberately one value. They don't change independently.

### The border is not a cell

The border is drawn as a thin stroke on the box edge, not as a cell of its own. The 1-cell `FLEX_BORDER` that layout used to add was really padding. So:

- `FLEX_BORDER` stays only in `paint`, as the stroke thickness (`border: FLEX_BORDER`). Layout no longer uses it.
- Padding is `FLEX_SPACE`: `FLEX_SPACE.width` columns on the left and right, `FLEX_SPACE.height` rows on the top and bottom. The border line sits inside that space.

### Responsibilities

- **`measure`**: a box is its children's main sizes plus `FLEX_SPACE.main(direction)` between each pair, then `2 * FLEX_SPACE.width` wider and `2 * FLEX_SPACE.height` taller. Texts and inner boxes are padded the same way.
  - Empty box: 2×2 → **4×2**.
  - Box holding "Hello": 7×3 → **9×3**, with the label starting at `x + 2`.
- **`Rect::inner()`**: shrinks by `FLEX_SPACE.width` on the left and right and by `FLEX_SPACE.height` on the top and bottom, instead of by `FLEX_BORDER`.
- **`distribute`**: takes the default gap as a parameter (`gap: i64`). `arrange` passes `FLEX_SPACE.main(direction)`, so a row's siblings are 2 columns apart and a column's are 1 row apart. `SpaceBetween` is unchanged and still spreads the free room.
- **`lay_out_window`**: stacked outer boxes are `FLEX_SPACE.height` (1 row) apart, both in the stack height and when advancing `y`.

### Out of scope

A full-width outer box in a window narrower than its contents may get a spread gap below the minimum. `distribute` does not clamp it.

### Tests

The existing tests in `view.rs` that assert positions or sizes through `FLEX_BORDER` or `FLEX_GAP` are rewritten in terms of `FLEX_SPACE`. Two examples: an empty box is `2 * FLEX_SPACE.width` × `2 * FLEX_SPACE.height`, and a full box's label starts `FLEX_SPACE.width` in from its edge. The tests that check the painted border thickness keep using `FLEX_BORDER`. New tests cover each acceptance criterion: horizontal padding, vertical padding, the row gap, the column gap, and the outer stack gap.
