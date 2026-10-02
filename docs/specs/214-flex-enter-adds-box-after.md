# Flex: Enter while writing adds a box after it

Lina is in Write mode typing "DRE" into a box. She presses Enter. A new, empty box appears immediately after the current one — in whichever direction the parent lays out (right for Row, below for Column) — and she's instantly in Write mode on that new box, able to keep typing without pressing any other key.

## Acceptance Criteria

- Pressing Enter while writing non-empty text commits that text to the current box.
- A new empty box is created as the next sibling, inserted immediately after the current box (not appended to the end of the parent's children).
- The new box is selected and Write mode stays active, so typing continues immediately in the new box.
- The new box's position follows the parent's existing direction (beside it for Row, below it for Column) — Enter never changes the parent's direction.
- Pressing Enter on an empty box still drops that box and returns to Move mode on the parent (unchanged from today — spec 182).

## Technical Design
