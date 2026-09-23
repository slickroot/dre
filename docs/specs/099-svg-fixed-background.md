# SVG fixed background

## User Story

Doug exports his diagram from dre to SVG. Instead of the SVG switching between light and dark backgrounds depending on the viewer's system preference, it always shows with the fixed background color `#0A0B0D`, matching the terminal exactly.

## Acceptance Criteria

- Exported SVG files have a background of `#0A0B0D` regardless of the viewer's OS/browser light or dark mode setting.
- The `prefers-color-scheme` media query and its alternate color values are removed from the exported SVG.
- Viewing the SVG in both a light-mode and dark-mode browser produces the identical background color.

## Technical Design
