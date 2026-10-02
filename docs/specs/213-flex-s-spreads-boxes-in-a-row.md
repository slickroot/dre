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
