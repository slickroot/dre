Lina is typing a label into a box in write mode. She's happy with what she's typed but doesn't want a new box to pop up after it, so instead of pressing Enter she presses Esc. The box keeps her text and she's back in move mode, ready to navigate. Another time, she opens a fresh empty box, changes her mind, and presses Esc — the box disappears instead of leaving an empty one behind.

## Acceptance Criteria

- Pressing Esc in write mode with non-empty text returns to move mode, keeping the typed text, and does not create a sibling box.
- Pressing Esc in write mode with empty text removes (or clears) the box, same as Enter does today, and returns to move mode.
- This behavior is the same regardless of how write mode was entered (`i`, `o`, or the Enter-adds-a-sibling flow).

## Technical Design
