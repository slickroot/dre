# 036 - Toggle a box between square and rounded corners

## Story

Bob selects a box and presses `r`. Its corners become rounded with a 20px
radius. Pressing `r` again makes them square.

## Acceptance Criteria

- `r` toggles the selected box between square corners and 20px rounded
  corners.
- A new box starts with square corners.
- `r` only changes the selected box's corners; other boxes are untouched.
- There is no intermediate radius to cycle through.
- The box's size and position stay exactly as they are today — only the drawn
  corners change.
- On a canvas with no boxes, `r` does nothing.

## Technical Design
