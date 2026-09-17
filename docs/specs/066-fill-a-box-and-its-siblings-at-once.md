# 066 - Fill a box and its siblings at once

## Story

Sam has a box with several children. He selects one of the children, presses
`F`, and the whole row takes the same fill together instead of him filling each
box one at a time.

## Acceptance Criteria

- `F` on a box whose siblings all share its fill advances the whole row one
  step along the palette.
- `F` on a box whose siblings have mixed fills sets the whole row to the first
  fill colour in the palette.
- Repeated `F` keeps stepping the row along the palette together, wrapping
  around to no fill after the last one.
- `F` fills only the selected box and the boxes sharing its parent.
- `F` on a top-level box does nothing.
- `F` with nothing selected does nothing.

## Technical Design