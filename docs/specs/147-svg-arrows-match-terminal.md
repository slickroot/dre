# Svg arrows match terminal

Amara exports her diagram to SVG and opens it in a browser. She expects the arrows to look the same as they did in her terminal — instead the arrow shafts look thinner and the arrowhead edges look longer, so the diagram feels visually inconsistent between the two. After the fix, she opens the same diagram in both the terminal and as an exported SVG and the arrows look visually the same.

## Acceptance Criteria

- Arrow shaft stroke width in the SVG export visually matches the arrow stroke width in the terminal render.
- Arrowhead edge length in the SVG export visually matches the arrowhead edge length in the terminal render.
- Verified by eyeballing a diagram with arrows side-by-side in the terminal and in a browser-opened SVG export.

## Technical Design
