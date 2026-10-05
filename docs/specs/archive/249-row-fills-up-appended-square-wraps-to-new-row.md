# Row fills up, appended square wraps to new row

Dana is adding squares to the flex canvas by pressing `a` repeatedly. Once squares fill the current row edge to edge, pressing `a` one more time doesn't try to cram the square off-screen — instead a new square appears at the start of a new row, directly below the first square of the row she just filled.

## Acceptance Criteria

- While there's room for another square in the current row, pressing `a` adds it to that row as before.
- When the current row has no room left for another square's width, pressing `a` places the new square at the start of the next row (same horizontal start position as the first square in the row above).
- The new row sits directly below the previous row (no gap, no overlap).
- Previously placed squares don't move or change color when wrapping occurs.

## Technical Design

`layout_rect(idx, window)` currently places every square in a single row: `col = 2 * idx, row = 0`. It gains row-wrapping by computing capacity inline:

- `squares_per_row = window.cols / 2` (2 is the square's fixed width in character columns; integer division, computed fresh each call — no new field on `Window`).
- `row = idx / squares_per_row`
- `col = (idx % squares_per_row) * 2`

`row` and `col` are character-cell cursor positions, not pixels, so each wrapped row sits exactly one character-row below the previous one (height stays `window.cell_height`, i.e. 1 character row), giving no gap and no overlap.

No guard for `window.cols < 2` (division by zero) — out of scope, the window is assumed wide enough for at least one square.
