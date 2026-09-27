# Footer border and padding

Doug is using dre in a Kitty-graphics-capable terminal. He glances at the footer in the bottom-right, and now sees it fully boxed in with a border on all four sides (matching the existing top/left border style), with the text sitting 8px away from each edge instead of flush against it. Satisfied with the clearer, more polished look, he keeps working.

## Acceptance Criteria

- The footer has 1px borders on all four sides (top, right, bottom, left), using the same style/color as the existing top/left border.
- The footer text has 8px of padding between it and each of the four borders.
- This applies only to the interactive terminal rendering (via Kitty graphics); SVG export/rendering is unchanged.

## Technical Design
