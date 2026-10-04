Maya is editing a flex layout in dre-flex. She selects a box and presses **b** — its border disappears. She presses **b** again — the border comes back. Happy with the look, she moves on to the next box.

## Acceptance Criteria

- In MOVE mode, pressing `b` on the selected box toggles its `border` field (on ↔ off).
- The box renders with or without its border accordingly, immediately.
- Pressing `b` on the canvas root box has no effect — its border always stays on.
- In WRITE/Replace mode, pressing `b` types a literal "b" character instead of toggling anything.
- Pressing `u` undoes the border toggle.

## Technical Design
