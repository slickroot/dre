# SVG rendering uses same font as terminal

Hamid draws a diagram in dre and exports it to SVG. Later, he opens that SVG file on any computer — his own, a colleague's, or in a browser that doesn't have Iosevka installed. The text renders in Iosevka, exactly like it looks in the dre terminal.

## Acceptance Criteria

- The exported SVG embeds the Iosevka font data directly (e.g. via `@font-face` with embedded font data), not just a `font-family` name reference.
- Opening the SVG on a machine without Iosevka installed still shows the text in Iosevka, not a fallback monospace font.
- The rendered glyphs visually match the terminal's Iosevka rendering.

## Technical Design
