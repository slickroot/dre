# 032 - Round box corners

## Story

Bob selects a box and presses `r` repeatedly. Its corners cycle through three
radii — 0px (square), then 4px round, then 8px round — and pressing `r`
again after 8px wraps back to 0px.

## Acceptance Criteria

- `r` cycles the selected box's corner radius through three levels in order:
  0px → 4px → 8px → back to 0px.
- A new box starts with 0px (square) corners.
- `r` only changes the selected box's corners; other boxes are untouched.
- The box's size and position stay exactly as they are today — only the
  drawn corners change.
- On a canvas with no boxes, `r` does nothing.
- In insert mode, `r` types the letter "r" into the label.

## Technical Design
