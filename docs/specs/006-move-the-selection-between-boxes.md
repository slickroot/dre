# 006 - Move the selection between boxes

## Story

Bob has three boxes. He presses `Esc`, then `k` twice — the double-lined border walks up from the bottom box to the first one. He presses `i` and types, and it is that first box that changes.

## Acceptance Criteria

- In command mode the selected box is drawn with a double-line border; the others keep their single-line border.
- `j` moves the selection down one box, `k` moves it up one.
- `j` on the bottom box leaves the selection where it is; `k` on the top box leaves it where it is.
- Pressing `b` adds a new box at the bottom and selects it, so `Esc` leaves Bob on the box he just made.
- Pressing `i` enters insert mode on the selected box, rather than the last box.
- On a canvas with no boxes, `j` and `k` do nothing.

## Technical Design
