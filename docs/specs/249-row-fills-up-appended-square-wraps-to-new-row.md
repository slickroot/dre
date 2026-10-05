# Row fills up, appended square wraps to new row

Dana is adding squares to the flex canvas by pressing `a` repeatedly. Once squares fill the current row edge to edge, pressing `a` one more time doesn't try to cram the square off-screen — instead a new square appears at the start of a new row, directly below the first square of the row she just filled.

## Acceptance Criteria

- While there's room for another square in the current row, pressing `a` adds it to that row as before.
- When the current row has no room left for another square's width, pressing `a` places the new square at the start of the next row (same horizontal start position as the first square in the row above).
- The new row sits directly below the previous row (no gap, no overlap).
- Previously placed squares don't move or change color when wrapping occurs.

## Technical Design
