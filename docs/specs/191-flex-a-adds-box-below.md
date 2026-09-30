# Flex: a adds a box below in MOVE

## User Story

Noor runs `dre-flex`, types "Hello" and presses Enter, so they're in MOVE. They press `a`, and a new empty box appears below "Hello", with a 1-cell gap between them. Noor is still in MOVE. They press `a` again, and a third empty box appears below the second one. The whole stack stays centred on the screen, with the boxes lined up by their centres. Back in WRITE, pressing `a` just types an "a".

## Acceptance Criteria

- In MOVE, `a` adds a new empty box below the last box.
- After `a`, `dre-flex` stays in MOVE.
- Each additional `a` adds another box below the last one.
- There is a 1-cell gap between boxes.
- The boxes are lined up by their centres.
- The whole stack stays centred on the screen.
- In WRITE, `a` types an "a" and doesn't add a box.

## Technical Design
