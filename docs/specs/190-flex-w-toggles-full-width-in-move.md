# Flex: w toggles full width in MOVE

## User Story

Sami runs `dre-flex` and types "Hello". The box is just wide enough for the text. They press Enter to go to MOVE, then press `w`. The box stretches from the left edge of the window to the right edge, with "Hello" at the left. They press `w` again and the box shrinks back to fit the text. They press `w` once more to make it full width, then press `i` to go back to WRITE. The box stays full width while they type. In WRITE, pressing `w` just types a "w".

## Acceptance Criteria

- The box starts at text width.
- In MOVE, `w` stretches the box across the whole window, left edge to right edge.
- When the box is full width, the text sits at the left.
- In MOVE, `w` on a full-width box shrinks it back to text width.
- In WRITE, `w` types a "w" and doesn't change the width.
- The box stays full width when Sami switches from MOVE back to WRITE with `i`.

## Technical Design
