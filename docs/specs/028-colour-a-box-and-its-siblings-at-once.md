# 028 - Colour a box and its siblings at once

## Story

Bob has a box with several children. He selects one of the children, presses
`C`, and the whole row takes the colour together instead of him colouring each
box one at a time.

## Acceptance Criteria

- `C` on a box whose siblings all share its colour advances the whole row one
  step along the palette.
- `C` on a box whose siblings have mixed colours sets the whole row to the
  first colour in the palette.
- Repeated `C` keeps stepping the row along the palette together, wrapping
  around to no colour after the last one.
- `C` colours only the selected box and the boxes sharing its parent.
- `C` on a top-level box does nothing.
- `C` with nothing selected does nothing.

## Technical Design
