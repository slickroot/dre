# Flex: g spreads the texts across a full-width box

## User Story

Noor has a box with "Hello" and "World" in it, and the box is in MOVE. They press `w`, so the box goes full width. The texts sit side by side at the left, with a 1-cell space between them. Noor presses `g`. "Hello" stays at the left edge, "World" moves to the right edge, and the free space is shared between them. Noor presses `g` again, and the texts are back next to each other with the 1-cell space. In WRITE, pressing `g` just types a "g".

## Acceptance Criteria

- In MOVE, `g` turns the gap on. Pressing `g` again turns it off.
- With the gap on and the box full width, the first text sits at the left edge and the last text sits at the right edge.
- With three or more texts, the texts in between are placed so every gap is the same size.
- If the free space doesn't split evenly, the extra cells go to the right gaps.
- With the gap on and only one text, the text stays at the left, where it is today.
- With the gap on and the box fit-to-text, nothing changes on screen. The texts spread out as soon as `w` makes the box full width.
- With the gap off, the texts sit next to each other with a 1-cell space between them.
- In WRITE, `g` types a "g" and doesn't change the gap.

## Technical Design
