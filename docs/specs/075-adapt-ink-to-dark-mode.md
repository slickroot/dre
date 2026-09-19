# 075: Adapt ink to dark mode

## User Story

When I open my exported diagram on a dark-themed page (like GitHub dark mode), the diagram's
plain color — the labels, the arrows, and the plain box borders — turns white, so the diagram
stays readable with no background added. On light mode it looks exactly like today.

## Acceptance Criteria

- On a light-mode page, the exported SVG looks exactly as it does today
- On a dark-mode page, the labels turn white
- On a dark-mode page, the arrows turn white
- On a dark-mode page, the plain (uncoloured) box borders turn white
- Coloured box borders and fills stay the same in both modes
- No background is added — the page's own background still shows through

## Technical Design