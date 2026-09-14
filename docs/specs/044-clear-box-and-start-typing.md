# 044 - Clear a box and start typing

## User Story

As a user editing a box's text, I want to press `Shift+I` to instantly clear the box's existing label and start typing fresh, so I don't have to backspace through old text one character at a time.

## Acceptance Criteria

- Pressing `I` (capital) while a box is selected in command mode clears that box's label and switches to insert mode, ready for typing.
- If the box's label is already empty, pressing `I` still switches to insert mode with no visible change.
- Pressing `u` (undo) after using `I` restores the box's previous label (i.e., `I` is added to `UNDOABLE_KEYS`).

## Technical Design
