# Doug adds side padding to a box with p

## User Story

Doug has a box in dre-flex with the text "Hello". The text feels squeezed against the sides. In move mode he selects the box and presses `p`. The box gets one more unit of space on its left and right, and "Hello" now sits 4 columns in from each side. Happy, he carries on drawing!

## Acceptance Criteria

- In move mode, `p` adds one unit (2 columns) of padding on the left and right of the selected box.
- The top and bottom padding stays at 1 row.
- Pressing `p` again keeps adding one unit each time, with no limit.
- `u` undoes one `p`.
- `p` works on inner boxes too, and the parent box grows to make room.
- If a text is selected, `p` does nothing.
- In write mode, `p` just types a "p".
- A full-width box keeps its width, and its content gets the extra room on the inside.

## Technical Design
