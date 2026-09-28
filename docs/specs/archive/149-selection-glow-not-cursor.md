# Selection glow, not cursor

Maya selects a box in her diagram. Instead of a blinking text cursor at the end of the label, the box itself lights up with a brighter, glowing highlight that extends a bit beyond its borders. She steps away for a few seconds, and when she comes back, the box is still glowing — it hasn't faded or disappeared.

## Acceptance Criteria

- Selecting a box highlights the whole box (brighter, bolder color, with a glow extending slightly beyond its borders) instead of showing a text cursor.
- The glow stays visible indefinitely while the box remains selected — it no longer disappears after ~1 second of idling.
- Moving selection to a different box moves the glow there, and the previously selected box returns to its normal appearance.

## Technical Design
