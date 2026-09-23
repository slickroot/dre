# New box palette colours

## User Story

Bob opens dre and colors a box on his diagram. He cycles through the available colors and sees the new palette — Acid lime, Mint, UV violet, Hot magenta, and Amber — instead of the old colors. When he exports his diagram to SVG, the colored boxes keep those same exact colors.

## Acceptance Criteria

- Cycling a box through the 5 colors shows, in order: Acid lime `#C6FF00`, Mint `#39FFB0`, UV violet `#B388FF`, Hot magenta `#FF3DF5`, Amber `#FFB020`.
- These colors are visible in the terminal rendering of a colored box.
- Exporting a diagram with colored boxes to SVG produces the same 5 hex values for the corresponding boxes.
- The old palette colors no longer appear anywhere.

## Technical Design
