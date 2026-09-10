# Insert box at selection

As a user, when I have a box selected that isn't the last one on the canvas, I want `bj`/`bl` to insert the new box right next to my selected box, so the new box appears where I'm actually working instead of at the end of the canvas.

## Acceptance Criteria

- Given a box is selected that is not the last box on the canvas, when I press `bj`, the new box is inserted directly below the selected box (spliced in at that position, not appended to the end).
- Given a box is selected that is not the last box on the canvas, when I press `bl`, the new box is inserted directly to the right of the selected box (spliced in at that position, not appended to the end).
- Boxes and connections that came after the selected box remain in place relative to the selected box after the insertion.
- After insertion, the newly created box becomes the selected box and the app enters insert mode, same as today.

## Technical Design
