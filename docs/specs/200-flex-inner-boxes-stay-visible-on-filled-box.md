# Flex: inner boxes stay visible on a filled box

## User Story

Sami runs `dre-flex`, types "Hello" and presses Enter to switch to MOVE. They press `A` to add a box inside it, and the inner box appears with its border. They press `f` to fill the outer box. It fills with #141416, and the inner box's border stays visible on top of the fill. They press `f` again, the fill goes away, and the inner box is still there.

## Acceptance Criteria

- An inner box added before the outer box is filled keeps its border visible on top of the fill.
- An inner box added after the outer box is filled also shows its border on top of the fill.
- Removing the fill with `f` leaves the inner box's border visible.
- The outer box's fill stays #141416.

## Technical Design
