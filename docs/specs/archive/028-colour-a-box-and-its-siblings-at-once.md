# 028 - Colour a box and its siblings at once

## Story

Bob has a box with several children. He selects one of the children, presses
`C`, and the whole row takes the colour together instead of him colouring each
box one at a time.

## Acceptance Criteria

- `C` on a box whose siblings all share its colour advances the whole row one
  step along the palette.
- `C` on a box whose siblings have mixed colours sets the whole row to the
  first colour in the palette.
- Repeated `C` keeps stepping the row along the palette together, wrapping
  around to no colour after the last one.
- `C` colours only the selected box and the boxes sharing its parent.
- `C` on a top-level box does nothing.
- `C` with nothing selected does nothing.

## Technical Design

- Add a new pure helper `colour_row(boxes: Tuple[Box, ...], path: Path) -> Tuple[Box, ...]` in `state.py`, alongside `next_colour`, `at`, `rewrite`, and `grow`.
  - `parent = path[:-1]`; `siblings = at(boxes, parent).children if parent else boxes`.
  - If every sibling shares one colour (`len({box.colour for box in siblings}) == 1`), the new colour is `next_colour(siblings[0].colour)`. Otherwise the new colour is `0`, the first real colour in the palette.
  - Loop over each sibling's index and call `rewrite(boxes, parent + (i,), lambda box: replace(box, colour=new_colour))` to apply the new colour to every sibling in turn.
- In `handle_command`, add a `"C"` branch mirroring the existing `"h"`/`"j"`/`"k"` guards: if `len(state.selected) <= 1` (nothing selected, or a top-level box with no parent), return `state` unchanged. Otherwise call `colour_row(state.boxes, state.selected)` and return the updated state.
- No changes needed to key reading (`writer.py`) — `"c"` and `"C"` already arrive as distinct single characters from `stdin.read(1)`.
