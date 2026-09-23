# 101: Insert mode "Enter to add a child" hint

## User Story

Sofia is editing a box's label in Insert mode. Immediately, before she types anything, she sees a faded hint box positioned exactly where a new child box would appear, reading "Enter to add a child". She keeps typing her label, and the hint stays visible the whole time. She presses Enter, and a child box is added there as promised.

## Acceptance Criteria

- While in Insert mode editing a box's label, a hint is shown at the position where a new child box would be created.
- The hint reads "Enter to add a child".
- The hint appears immediately upon entering Insert mode, before any text is typed.
- The hint remains visible as the user types the label.

## Technical Design
