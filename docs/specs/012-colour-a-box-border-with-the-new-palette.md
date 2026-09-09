# 012 - Colour a box's border with the new palette

## Story

Bob has a box selected. In command mode he presses `c` and its border turns
Amber Gold, then Blaze Orange, then Neon Pink as he keeps pressing. He presses
`j` down to another box and presses `c` once — that box turns Amber Gold, and
the first box is still Neon Pink.

## Acceptance Criteria

- In command mode, `c` advances the selected box's border one step through the
  cycle: grey (plain), Amber Gold (`#ffbe0b`), Blaze Orange (`#fb5607`), Neon
  Pink (`#ff006e`), Blue Violet (`#8338ec`), Azure Blue (`#3a86ff`), and the
  next press returns it to grey.
- `c` changes only the selected box's border; other boxes are untouched.
- Each box keeps its own colour — moving the selection with `j`/`k`, and
  adding a box with `b`, leave every box's colour as it was.
- The label inside a coloured box stays the plain default colour.

## Technical Design
