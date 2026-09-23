# 103: Numbered colour palette

## User Story

Doug selects a box and presses `c`. A numbered palette overlay appears, listing all 7 colors (lime, mint, violet, pink, amber, foreground, background), with the box's current color highlighted. He presses `2` and the box turns mint immediately, the palette closes. Happy with the result, he moves on.

## Acceptance Criteria

- Pressing `c` (or `C`) on a selected box opens a numbered palette overlay listing all 7 palette colors.
- The color currently applied to the box is visually marked/highlighted in the overlay.
- Pressing a digit key matching a listed color applies that color to the box immediately and closes the overlay.
- Pressing any non-digit key closes the overlay without changing the box's color.
- Existing `c`/`C` cycling behavior (cycle this box / cycle siblings) continues to work while the overlay is visible — i.e. this palette is additive, not a replacement.

## Technical Design
