# 009 - Fill a box with a colour

## Story

Bob has a box with a red border. In command mode he presses `f` and its inside
turns black, then red, then green as he keeps pressing — the label sits on top of
the colour, and the border stays red the whole time. He presses `j` down to the
next box and presses `f` once — that box's inside turns black, and the first box
keeps its green fill.

## Acceptance Criteria

- A new box has no fill; its inside is the terminal's plain background.
- In command mode, `f` advances the selected box's fill one step through the
  cycle: none, black, red, green, yellow, blue, magenta, cyan, white, and the
  next press returns it to none.
- The fill covers the box's interior area only — the border row and column are
  not painted.
- The fill runs behind the label, so the interior is one solid block of colour
  with the label text sitting on top of it and staying readable.
- `f` never changes the border colour, and `c` never changes the fill — a box can
  have a red border and a blue fill at the same time.
- `f` changes only the selected box; the other boxes are untouched.
- Each box keeps its own fill — moving with `j` and `k`, and adding a box with
  `b`, leave every box's fill as it was.
- In insert mode, `f` types the letter "f" into the label.
- On a canvas with no boxes, `f` does nothing.

## Technical Design
