# Flex: Enter commits the text

## User Story

Lina runs `dre-flex` and types "Hello" into the box. Happy with it, she presses Enter, and `dre-flex` switches from WRITE to MOVE. She taps a few letters and Backspace, and her box still says "Hello", untouched. She presses Ctrl-C to leave.

## Acceptance Criteria

- Pressing Enter in WRITE mode switches to MOVE mode.
- The text typed before Enter stays in the box.
- In MOVE mode, typed characters don't change the box's text.
- In MOVE mode, Backspace doesn't change the box's text.
- In MOVE mode, Ctrl-C quits without saving.

## Technical Design
