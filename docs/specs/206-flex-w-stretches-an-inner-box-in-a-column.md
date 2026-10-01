# Leila stretches an inner box across its parent with w

## User Story

Leila has a full-width box in dre-flex, stacked as a column, with the text "Title" and an inner box "Hi" inside it. The inner box is only as wide as "Hi". In move mode she selects the inner box and presses `w`. It stretches from the left inside edge of its parent to the right inside edge, keeping the parent's padding. Happy, she carries on drawing!

## Acceptance Criteria

- In move mode, when the parent stacks its children in a column, `w` on an inner box stretches it to the parent's inside width and keeps the parent's padding on both sides.
- Pressing `w` again shrinks the inner box back to fit its content.
- When the parent lays its children out in a row, `w` on an inner box changes nothing on screen.
- If Leila presses `w` on an inner box while its parent is a row and then switches the parent to a column with `d`, the inner box shows full width.

## Technical Design
