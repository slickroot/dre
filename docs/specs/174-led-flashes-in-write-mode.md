## User Story

Maya is typing a label into a box in Write mode. While she pauses, the footer LED rests dim violet. Each key she presses (a letter, Backspace, an arrow, Enter, or Esc) makes the LED flash fully lit violet and then drop back to dim, just like the lime LED does in MOVE mode. She types a word quickly and sees one violet blink per keystroke, keeping pace with her fingers.

## Acceptance Criteria

- In Write mode, while Maya isn't typing, the footer LED rests dim violet.
- Every key pressed in Write mode (characters, Backspace, ←, →, Enter, Esc) makes the LED go dim → fully lit → dim, staying violet throughout.
- On Esc, the violet flash happens before the LED switches to dim lime for MOVE mode.
- When she types quickly, each key gets its own distinct blink instead of the LED staying lit.
- The LED in Naming mode stays unchanged: amber and always fully lit.

## Technical Design
