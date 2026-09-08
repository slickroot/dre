# 010 - Draw a box's border in pixels

## Story

Bob adds a box and types "hi". The box sits where it always did, but its border
is now a thin crisp line riding the very edge of the box instead of a row of
characters. He presses `c` and that line turns red. He presses `f` and the inside
turns black — right up to the red line, with no gap anywhere.

## Acceptance Criteria

- A box's border is drawn in pixels, one pixel thick, sitting on the outer edge
  of the box's area rather than inside a character cell.
- A box keeps the size and screen position it has today: as wide as its label
  plus one cell of padding on each side, and three rows tall.
- Every cell of the box belongs to the box — no cell is given over to the border.
  A 3×3 box has all nine cells to itself.
- The label sits in the middle row, one cell in from the left edge.
- `c` cycles the border colour as it does today, and the pixel border is drawn in
  that colour.
- `f` fills every cell of the box, so the fill runs right up to the border with
  no gap and the border does not cut through it.
- The label, the fill cycle, the arrows and the cursor are otherwise unchanged.
- The terminal is assumed to support pixel graphics; there is no fallback in this
  story.

## Technical Design
