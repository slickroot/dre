# Flex: A adds a box inside the selected box in MOVE

## User Story

Noor runs `dre-flex`, types "Hello" and presses Enter, so they're in MOVE. They press `A`, and a new empty box appears inside the "Hello" box, below the text. There's a 1-cell gap between "Hello" and the inner box, and the inner box is centred under the text. The outer box grows to fit it, and it stays centred on the screen. Noor is still in MOVE. They press `A` again, and a second empty box appears inside the outer box, below the first one. Noor presses `i` and types "Hi", and the text goes into the outer box's "Hello", not into an inner box. In WRITE, pressing `A` just types an "A".

## Acceptance Criteria

- In MOVE, `A` adds a new empty box inside the selected box, below its text.
- After `A`, `dre-flex` stays in MOVE.
- There is a 1-cell gap between the text and the inner box.
- The inner box is centred under the text.
- The outer box grows to fit the inner box.
- The outer box stays centred on the screen.
- Each additional `A` adds another inner box inside the same outer box, below the last inner box.
- An inner box looks like an empty box from `a`: a thin border in the usual border colour, a 1-cell interior, and no fill.
- Typing in WRITE still goes into the selected box's last text, never into an inner box.
- Inner boxes can't be selected. `j` and `k` still move only between outer boxes.
- `a` is unchanged. It adds a new outer box at the bottom of the stack, as in spec 194.
- In WRITE, `A` types an "A" and doesn't add an inner box.

## Technical Design
