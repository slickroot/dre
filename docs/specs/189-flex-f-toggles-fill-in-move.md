# Flex: f toggles the fill in MOVE

## User Story

Sami runs `dre-flex` and sees an empty, unfilled box. They type "Hello" and press Enter, so they're in MOVE. They press `f` and the box fills with #2A2A2E. They press `f` again and the fill goes away. They press `f` once more to fill it, then press `i` to go back to WRITE. The box stays filled while they type. In WRITE, pressing `f` just types an "f".

## Acceptance Criteria

- The box starts unfilled.
- In MOVE, `f` fills the box with #2A2A2E.
- In MOVE, `f` on a filled box empties it again.
- In WRITE, `f` types an "f" and doesn't change the fill.
- The fill stays when Sami switches from MOVE back to WRITE with `i`.

## Technical Design
