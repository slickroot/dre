# 107: Delete a box

## Story

Doug has a diagram with "API gateway" at the top and "Auth", "Payments" and
"Orders" underneath. "Payments" is outdated, so he selects it and presses `d`.
"Payments" and anything under it vanish, and the selection lands on "Orders".
He changes his mind and presses `u`, and "Payments" comes back.

## Acceptance Criteria

- Pressing `d` in command mode deletes the selected box and all its descendants.
- The selection moves to the next sibling. If there is none, it moves to the
  previous sibling. If the box was an only child, it moves to the parent.
- Deleting the root box empties the canvas, like a fresh `dre`.
- Pressing `u` right after a delete brings back the box and its descendants,
  with the selection where it was before the delete.

## Technical Design
