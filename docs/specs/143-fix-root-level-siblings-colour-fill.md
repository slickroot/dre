## User Story

Doug draws a row of boxes at the root level of his diagram, with no parent box selected. He selects one of the root-level boxes and presses `C`; the colour cycles across all the root-level boxes in that row. He then presses `F`; the fill toggles across all of those same root-level boxes. Both behave exactly like `R` already does for rounded corners at the root level.

## Acceptance Criteria

- Selecting any root-level box and pressing `C` cycles the colour of all root-level boxes in that row (not just the selected one).
- Selecting any root-level box and pressing `F` toggles the fill of all root-level boxes in that row (not just the selected one).
- Both match the existing behavior of `C` and `F` on nested sibling boxes, and the existing behavior of `R` at the root level.
- The existing tests asserting `C` and `F` do nothing on a top-level box are updated to reflect the new expected behavior.

## Technical Design
