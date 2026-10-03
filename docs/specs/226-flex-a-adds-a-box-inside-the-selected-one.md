# Flex: `a` adds a box inside the selected one

## User Story

Doug is in move mode with a box selected. He presses `a`. A new empty box
appears inside the selected box, and Doug is now in write mode, ready to type
into it. Pressing `a` again adds another box inside that new box.

## Acceptance Criteria

1. In move mode, `a` adds a new empty box inside the selected box, as its last
   child.
2. The selection moves to the new box.
3. After `a`, dre-flex is in write mode and typing goes into the new box.
4. `a` no longer adds a box at the root.
5. `A` is ignored completely: it changes neither the diagram nor the undo
   history.
6. In write mode, `a` still types an "a".
7. If the selected thing is a text, `a` does nothing.
8. Adding with `a` is undoable.

## Technical Design
