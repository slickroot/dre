# New foreground colour

## User Story

Doug opens dre and draws a plain, uncolored box with some text and an arrow. Instead of the old grey/black look, the borders, text, arrows, and cursor all appear in the new foreground color. When he exports to SVG, those same elements use the same foreground color there too.

## Acceptance Criteria

- Uncoloured box borders render in `#E8EAED` in the terminal.
- Text render in `#E8EAED` in the terminal.
- Arrows render in `#E8EAED` in the terminal.
- The cursor renders in `#E8EAED` in the terminal.
- Exporting to SVG produces `#E8EAED` for all of the above (borders, text, arrows, cursor).
- The old grey and black colors no longer appear for these elements.

## Technical Design
