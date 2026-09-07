# 008 - Connect two boxes with an arrow

## Story

Bob has two boxes. In command mode he presses `a` on the top one, presses `j` to
move down to the second, and presses `a` again — a `↓` appears in the blank row
between them, pointing from the first box to the second.

## Acceptance Criteria

- In command mode, `a` on the selected box marks it as the arrow's source.
  Nothing on screen changes.
- Moving with `j` or `k` and pressing `a` again draws an arrow from the source
  box to the box the selection has landed on.
- An arrow to the box below draws as `↓`; an arrow to the box above draws as `↑`.
- The arrow sits in the existing blank row between the two boxes, horizontally
  centered like the boxes are. Box spacing is unchanged.
- Pressing `a` on a box that is not directly above or below the source does
  nothing.

## Technical Design
