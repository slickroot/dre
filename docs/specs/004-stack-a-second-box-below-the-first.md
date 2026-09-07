# 004 - Stack a second box below the first

## Story

Bob has a labelled box. He presses `Esc`, then `b` again, and a second empty box appears below the first with a blank line between them. He types, and the new box takes his label while the first one keeps its own.

## Acceptance Criteria

- Pressing `b` when a box already exists places the new box below the last one.
- There is one blank line between each box and the next.
- Each box is horizontally centered on its own, so the boxes' middles line up.
- Typing after the new box appears goes into that new box; earlier boxes keep their labels unchanged.

## Technical Design
