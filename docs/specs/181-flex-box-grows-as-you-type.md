# Flex box grows as you type

## User Story

Lina runs `dre-flex`. She sees one empty box on the canvas, and the footer shows WRITE. She types "Hello", and the letters appear inside the box as she types, with the box growing to fit them. She mistypes a letter, presses Backspace, and it's gone. Happy with her box, she presses Ctrl-C to leave.

## Acceptance Criteria

- Running `dre-flex` opens a canvas with one empty box.
- `dre-flex` starts in WRITE mode, and the footer shows WRITE.
- Each typed character appears inside the box right away.
- The box grows to fit the text as Lina types.
- Backspace removes the last character, and the box shrinks to fit.
- Ctrl-C quits without saving.

## Technical Design
