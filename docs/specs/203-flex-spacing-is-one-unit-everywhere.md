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
