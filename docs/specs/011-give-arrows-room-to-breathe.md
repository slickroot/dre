# 011 - Give arrows room to breathe

## Story

Bob has two boxes stacked on the canvas. The gap between them is now 2 rows
instead of 1, so they no longer feel cramped. When he connects them with an
arrow, the arrow fills both rows — a shaft with the head at the end — so it
reads as a proper arrow rather than a single glyph.

## Acceptance Criteria

- The gap between two stacked boxes is 2 rows tall.
- An arrow pointing down draws as a shaft on the first row and the head on the
  second: `│` then `↓`.
- An arrow pointing up draws as the head on the first row and a shaft on the
  second: `↑` then `│`.
- A gap with no arrow in it stays blank across both rows.
- Everything else is unchanged: box size, the stack staying centred, the label,
  the fill, the border colour and the cursor.

## Technical Design
