# Footer LED fills cell height

## User Story

Nassim is editing a diagram and glances at the footer to check which mode he's in. The mode LED now visually fills the cell next to the mode name — its dot and soft glow touch the top and bottom edges of the cell, matching the height of the mode name text — so it's easy to spot at a glance instead of looking like a tiny speck.

## Acceptance Criteria

- In both the SVG renderer and the terminal renderer, the footer LED (dot + halo together) touches the top and bottom edges of its cell.
- This applies in every mode that shows the footer (Command, NamePrompt, Insert), since they all share the same LED rendering.
- The lit LED still shows a soft glow/halo (not a hard-edged full circle); the unlit LED stays dimmed, same as today — only the sizing changes, not the lit/unlit look.

## Technical Design
