# Flex: Backspace removes an empty new box

## User Story

Mina is in Write mode on a box she just created with `o`, empty, having changed her mind. She presses Backspace. The box disappears, and she lands in Move mode on the previous sibling box — or, if it was the first child, on its parent.

## Acceptance Criteria

- Pressing Backspace while the current box's text is already empty, and the box is borderless with no children (the same condition used for dropping an empty box on Enter), removes the box.
- After removal, the previous sibling under the same parent becomes selected, if one exists.
- If the removed box was the first child (no previous sibling), its parent becomes selected instead.
- Mode switches to Move after the removal.
- Pressing Backspace on a bordered box with empty text (e.g. one entered via `i`) leaves the box in place — unchanged from today, just clears/keeps text as before.
- Backspace on non-empty text still just removes the last character, unchanged from today.

## Technical Design
