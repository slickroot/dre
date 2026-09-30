# Flex: the box wears its own colours

## User Story

Sami runs `dre-flex` and sees an empty box with a dark #2A2A2E border. They type "Hello", and the text shows in a soft #C9C9CF grey while the box grows around it, the border still dark. They press Backspace, the box shrinks, and the colours don't change. They press Enter to switch to MOVE, and everything keeps the same colours. They press Ctrl-C to leave.

## Acceptance Criteria

- The box border is drawn in #2A2A2E.
- The text inside the box is drawn in #C9C9CF.
- The colours stay the same in both WRITE and MOVE mode.
- The border stays #2A2A2E while the box grows and shrinks as you type and backspace.

## Technical Design
