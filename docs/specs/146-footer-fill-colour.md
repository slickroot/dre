## User Story

Doug opens the app and looks at the footer bar. Instead of a plain outlined box, it has a light foreground-coloured tint (12% opacity) filling it, making it stand out from the body while the footer text stays perfectly readable on top.

## Acceptance Criteria

- The footer box is filled with the foreground colour at 12% opacity (distinct from the existing 30% fill used elsewhere).
- This applies consistently in both the terminal renderer and the SVG renderer.
- The footer text remains fully legible over the tinted fill.

## Technical Design
