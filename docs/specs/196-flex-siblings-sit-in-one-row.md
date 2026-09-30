# Flex: texts and inner boxes sit in one row, in the order they were added

## User Story

Noor runs `dre-flex`, types "Hello" and presses Enter, so they're in MOVE. They press `A`, and an empty inner box appears to the right of "Hello", not below it. They press `s` and type "World", and "World" appears to the right of the inner box. The outer box now shows `Hello [box] World` in one row. "Hello" and "World" are centred vertically on the inner box's middle row. Noor presses Enter, then `w` and `g`. "Hello" moves to the left edge, "World" moves to the right edge, and the inner box sits between them with the same space on each side.

## Acceptance Criteria

- Texts and inner boxes are siblings in a single row, in the order Noor added them.
- In MOVE, `A` adds an inner box to the right of the last sibling.
- In MOVE, `s` adds a text to the right of the last sibling.
- There's a 1-cell gap between neighbouring siblings.
- Texts are centred vertically against the tallest sibling.
- The outer box grows to fit the row and stays centred on the screen.
- With `g` on and the box full width, all siblings are spread out: the first sits at the left edge, the last at the right edge, and every gap is the same size.
- Typing in WRITE still goes into the selected box's last text.

## Technical Design
