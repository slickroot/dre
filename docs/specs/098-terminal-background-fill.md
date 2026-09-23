# Terminal background fill

## User Story

Bob opens dre in his terminal. Instead of seeing his terminal's own background showing through, the entire dre canvas is filled with the new solid background color, `#0A0B0D`, giving the app a consistent look regardless of his terminal theme.

## Acceptance Criteria

- When dre opens, the entire canvas/viewport area is filled with `#0A0B0D`.
- This background fill remains in place regardless of what boxes, text, or arrows are drawn on top of it.
- The fill covers the full viewport, not just the area behind existing shapes.
- No part of the underlying terminal's own background color is visible through the dre canvas.

## Technical Design
