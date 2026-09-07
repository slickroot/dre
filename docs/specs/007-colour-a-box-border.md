# 007 - Colour a box's border

## Story

Bob has two boxes. In command mode he presses `c` on the selected one and its
border turns black, then red, then green as he keeps pressing. He presses `j`
down to the other box and presses `c` once — that box turns black, and the
first box is still green.

## Acceptance Criteria

- A new box is drawn in the terminal's plain default colour.
- In command mode, `c` advances the selected box's border one step through the
  cycle: plain, black, red, green, yellow, blue, magenta, cyan, white, and the
  next press returns it to plain.
- `c` changes only the selected box's border; the other boxes are untouched.
- Each box keeps its own colour — moving the selection with `j` and `k`, and
  adding a box with `b`, leave every box's colour as it was.
- The label inside a coloured box stays the plain default colour.
- In insert mode, `c` types the letter "c" into the label.
- On a canvas with no boxes, `c` does nothing.

## Technical Design
