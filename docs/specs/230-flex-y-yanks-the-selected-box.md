# Flex: `y` yanks the selected box

## User Story

Later, Noor finds a box she wants in two places. She selects it and presses `y` — nothing on screen moves, but dre remembers it. She walks to another box, presses `p`, and a copy appears inside it while the original stays exactly where it was. Delighted, she carries on drawing!

## Acceptance Criteria

1. In Move mode, with a box selected, `y` puts that box and its whole subtree on the clipboard without removing it from the diagram.
2. The diagram and the selection are unchanged by `y`.
3. A later `p` pastes a copy of the yanked box, leaving the original in place.
4. `y` replaces whatever was on the clipboard before.
5. With the canvas selected, `y` changes nothing.
6. `y` is not undoable: `u` does not clear the clipboard and acts on the last real change instead.
7. In Write and Replace modes, `y` is typed as the letter "y" and copies nothing.

## Technical Design
