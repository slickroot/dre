# Flex: `J` and `K` move the selected box among its siblings

## User Story

Noor is arranging boxes in dre-flex. She's in Move mode with one of the boxes selected among its siblings in a column. She presses `J`, and the box swaps places with the sibling below it, staying highlighted so she can keep nudging it down. When it reaches the bottom, `J` leaves it there, so she presses `K` and walks it back up until it's at the top, where `K` leaves it. The whole time, everything nested inside the box travels with it. She changes her mind, presses `u`, and the old order comes back. Happy, she carries on drawing!

## Acceptance Criteria

1. In Move mode, with a box selected whose parent lays out as a Column, `J` swaps it with its next sibling, moving it one position down.
2. `K` swaps it with its previous sibling, moving it one position up.
3. The selected box's whole subtree of nested boxes moves with it.
4. The selection travels with the box, so the same box stays highlighted in its new position.
5. On the last sibling, `J` changes nothing; on the first sibling, `K` changes nothing.
6. When the parent lays out as a Row, `J` and `K` change nothing.
7. When the canvas is selected (no box highlighted), `J` and `K` change nothing.
8. In Write and Replace modes, `J` and `K` are typed as the letters "J" and "K" and reorder nothing.
9. One `u` after a move restores the previous sibling order and the selection.

## Technical Design
