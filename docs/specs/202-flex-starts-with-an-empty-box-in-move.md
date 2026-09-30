# Flex: dre-flex starts with an empty box in MOVE

## User Story

Lina runs `dre-flex` and sees one empty box, centred on the screen, with no text inside. She's in MOVE. She presses `A`, and a new empty box appears inside it, exactly in the middle, with 1 cell of space between it and the outer box's border on every side. She presses `A` again, and a second inner box appears to the right of the first, with a 1-cell gap between them. There's still 1 cell of space around the pair. She presses `i`, and nothing happens.

## Acceptance Criteria

- When `dre-flex` starts, there is one empty box with no text in it, centred on the screen.
- When `dre-flex` starts, it is in MOVE.
- In MOVE, `A` adds an empty box inside the selected box.
- There is exactly 1 cell of space between the inner box and the outer box's border on the top, bottom, left and right.
- The outer box grows to fit the inner box and stays centred on the screen.
- Each additional `A` adds another inner box to the right of the last one, with a 1-cell gap between them.
- There is still 1 cell of space between the row of inner boxes and the outer box's border on every side.
- `i` does nothing on a box with no text.

## Technical Design
