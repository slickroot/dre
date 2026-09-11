# 028 - Centre a parent between its children

## Story

Bob presses `b` three times off one box and gets three children stacked to the
right. Instead of the parent clinging to the top of the stack, it sits level
with the middle of them, the arrow leaving straight out of its side and
branching evenly up and down.

## Acceptance Criteria

- A parent sits midway between its first child and its last child.
- With an odd number of children, the parent lines up level with the middle
  child.
- With an even number of children, the parent sits in the gap between the two
  middle children, filling it exactly.
- Centring is measured against the children themselves, not against their
  descendants — a child with a tall subtree of its own does not pull the parent
  towards it.
- The vertical gap between boxes is 3 rows, everywhere boxes are stacked.

## Technical Design
