# 005 - Get back into insert mode

## Story

Bob presses `Esc` and looks at his box. He presses `i`, the cursor reappears at the end of the label, and he carries on typing where he left off.

## Acceptance Criteria

- Pressing `i` in command mode enters insert mode on the last box, with the cursor at the end of its existing label.
- Typing then continues that box's label.
- On a canvas with no boxes, `i` does nothing.

## Technical Design
