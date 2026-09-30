# Flex: i returns to WRITE

## User Story

Lina has typed "Hello" and pressed Enter, so she's in MOVE. She decides it needs more, presses `i`, and the footer switches back to WRITE. She types " world", and it goes on the end, so her box reads "Hello world". Then she changes her mind about the whole thing and backspaces all the way into "Hello".

## Acceptance Criteria

- Pressing `i` in MOVE mode switches to WRITE mode, and the footer shows WRITE.
- After `i`, typed characters are added to the end of the existing text.
- After `i`, Backspace can delete characters that were typed before Enter.

## Technical Design
