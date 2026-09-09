# 013 - Add a box to the right

## Story

Bob has a labelled box. He presses `b` then `l`, and a second empty box
appears to the right of the first with a gap between them. He types, and the
new box takes his label while the first one keeps its own.

## Acceptance Criteria

- Pressing `b` then `l` when a box already exists places the new box to the
  right of the last one.
- There are four blank columns between each box and the next.
- The two boxes' vertical centers line up.
- The pair of boxes together is horizontally centered on screen.
- Typing after the new box appears goes into that new box; earlier boxes
  keep their labels unchanged.

## Technical Design
