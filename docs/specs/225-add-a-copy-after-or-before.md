# o/O add a copy of the selected box after/before it

## User Story

Doug is arranging boxes in a column. He lands on one he likes the look of and presses `o`; a fresh copy of it appears right below, empty and selected, and he starts typing its label straight away. Later he wants one above instead, so he presses `O`, and a copy appears above the box he's on, also ready to type.

## Acceptance Criteria

1. In Move mode with a box selected, `o` inserts a new box immediately after the selected box among its siblings — at the next position, not appended at the end.
2. `O` inserts the new box immediately before the selected box among its siblings.
3. The new box copies the selected box's border, fill, and padding, and starts with empty text.
4. The new box becomes selected and dre-flex enters Write mode, so the next keystrokes fill its label.
5. For a top-level box, the new box is inserted among the top-level boxes (after for `o`, before for `O`).
6. In Write mode, `o` and `O` just type the letter and add nothing.
7. The old `o` — dropping an empty text inside the selected box — is gone.
8. Each press is undoable: one `u` removes the new box and restores the previous selection.

## Technical Design
