# 036 - Toggle a box between square and rounded corners

## Story

Bob selects a box and presses `r`. Its corners become rounded with a 20px
radius. Pressing `r` again makes them square.

## Acceptance Criteria

- `r` toggles the selected box between square corners and 20px rounded
  corners.
- A new box starts with square corners.
- `r` only changes the selected box's corners; other boxes are untouched.
- There is no intermediate radius to cycle through.
- The box's size and position stay exactly as they are today — only the drawn
  corners change.
- On a canvas with no boxes, `r` does nothing.

## Technical Design

Spec 032 previously gave `Box` a `radius: Literal[0, 10, 20]` field with `r`
cycling through all three values. This story replaces that with a strict
two-state toggle:

- `Box.radius` becomes `Box.rounded: bool = False` (`sketch/state.py`). A new
  box defaults to `False` (square), matching the acceptance criteria.
- The `r` command handler in `handle_command` changes from the cycle formula
  to a plain negation: `replace(box, rounded=not box.rounded)`. It keeps the
  existing guard (no-op when `state.selected` is empty) and continues to use
  `rewrite` so only the selected box is touched.
- `render.py` gains a module-level constant `ROUNDED_RADIUS = 20`. In
  `_outline_box`, the radius passed to `RoundedBox` is derived as
  `ROUNDED_RADIUS if placement.node.rounded else 0`; a falsy value keeps the
  existing `_square_pixels` fast path.
- The sprite cache key's shape tuple swaps `node.radius` for `node.rounded`
  directly (`(node.colour, node.fill, node.border, node.rounded)`), so cache
  entries are keyed on the boolean rather than a pixel value.

No changes are needed for persistence or undo/redo, since neither exists in
the codebase; `Box` remains a plain immutable dataclass.
