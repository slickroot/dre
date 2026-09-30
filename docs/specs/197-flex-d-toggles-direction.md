# Flex: d flips the selected box between row and column in MOVE

## User Story

Noor has a box showing `Hello [box] World` in a row, and they're in MOVE. They press `d`. "Hello", the inner box and "World" now stack top to bottom in the same order, each centred horizontally in the box. The outer box reshapes to fit the column and stays centred on the screen. Noor presses `d` again, and everything is back in a row. In WRITE, pressing `d` just types a "d".

## Acceptance Criteria

- In MOVE, `d` switches the selected box from row to column. Pressing `d` again switches it back to row.
- After `d`, `dre-flex` stays in MOVE.
- In column, the siblings stack top to bottom in the order they were added.
- In column, each sibling is centred horizontally in the box.
- In column, there's a 1-row gap between neighbouring siblings.
- The outer box grows or shrinks to fit the column and stays centred on the screen.
- `d` only changes the selected box. Other boxes keep their direction.
- In column, `g` changes nothing on screen. Pressing `d` to go back to row shows the spread again if `g` is still on.
- In WRITE, `d` types a "d" and doesn't change the direction.

## Technical Design
