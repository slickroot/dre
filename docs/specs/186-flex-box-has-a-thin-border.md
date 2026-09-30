# Flex box has a thin border

## User Story

Lina runs `dre-flex`. She sees one empty box on the canvas with a thin 1-pixel outline. She types "Hello", and the box grows while the outline stays thin. She presses Backspace, and the box shrinks with the same thin outline. Happy with how light it looks, she presses Ctrl-C to leave.

## Acceptance Criteria

- When `dre-flex` starts, the empty box has a 1-pixel border on all four sides.
- The border stays 1 pixel as the box grows while Lina types and shrinks when she presses Backspace.
- Boxes in the main `dre` editor keep their current thickness.

## Technical Design
