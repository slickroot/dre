# Flex: `d` cuts the selected box

## User Story

Doug is arranging boxes in dre-flex. One of them isn't working out, so in Move
mode he selects it and presses `d`. The box and everything inside it disappear,
and the box that held it is now selected. If it was sitting straight on the
canvas, nothing is highlighted. He changes his mind, presses `u`, and it all
comes back. Happy, he carries on drawing!

## Acceptance Criteria

1. In Move mode with a box selected, `d` removes the selected box together with
   everything nested inside it.
2. After `d`, the selection moves to the box's parent.
3. If the removed box sat directly on the canvas, the selection becomes the
   canvas and no box is highlighted.
4. Cutting away the last box leaves an empty canvas.
5. In Write mode, `d` is typed into the text and removes nothing.
6. If the canvas is selected (no box highlighted), `d` does nothing.
7. One `u` after a cut brings the box and its contents back and restores the
   selection.

## Technical Design
