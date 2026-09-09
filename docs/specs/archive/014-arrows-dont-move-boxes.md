# Arrows don't move boxes

## User Story

As a user, when I connect two boxes with an arrow, I want the arrow to only connect them — never move the boxes — and to always point in the correct direction, so that adding a connection never disrupts my layout.

## Acceptance Criteria

- Given boxes arranged in a horizontal row, when I connect two of them with an arrow, the boxes' positions in the row do not change.
- Given boxes arranged in a vertical column, when I connect two of them with an arrow, the boxes' positions do not change (existing behavior, no regression).
- The arrow always points from the source box to the target box in the correct direction, regardless of layout orientation.

## Technical Design

### Root cause

`handle_command`'s `a` branch replaces the `Space` at `slot` with an `Arrow`.
Orientation lives only on `Space`, and `layout` switches its active axis only
when it sees a `Space` — so replacing one with an `Arrow` silently reverts the
axis to `"row"` and a horizontal pair collapses into a vertical stack. The
size mismatch (`Space("right")` is 4 columns wide, `Arrow` is 1) would move
the boxes even once the axis is fixed.

The fix is to make `Arrow` carry its own orientation and occupy the same cell
the gap did.

### `state.py`

- `Direction` becomes `Literal["up", "down", "left", "right"]`. The old
  `forward`/`backward` spelling is renamed outright — `forward` -> `down`,
  `backward` -> `up` — in source and tests. `Arrow`'s default stays `"down"`,
  so a bare `Arrow()` means what it means today. No compatibility shim:
  `Arrow` is internal, with no persisted documents and no external callers.
- New `HORIZONTAL = ("left", "right")` and a shared helper:

  ```python
  def axis(node: Union[Space, Arrow]) -> Literal["row", "col"]:
      return "col" if node.direction in HORIZONTAL else "row"
  ```

  This is total over both types without any type dispatch, because `Space`'s
  horizontal value is already `"right"`. It is the single place that knows
  which direction values are horizontal; `layout` and `handle_command` both
  use it.
- The `a` branch derives the new arrow's direction from the node already in
  the slot:

  ```python
  slot = (state.source + state.selected) // 2
  forward = state.selected > state.source
  if axis(state.nodes[slot]) == "col":
      direction = "right" if forward else "left"
  else:
      direction = "down" if forward else "up"
  ```

  Because `axis` accepts an `Arrow` as well as a `Space`, connecting an
  already-connected pair in the opposite order reverses the arrow in place
  and keeps its orientation, rather than losing it.

### `layout.py`

- The axis walk switches on either node type: `if isinstance(node, (Space,
  Arrow)): active = axis(node)`. Nothing is lost when a `Space` becomes an
  `Arrow`.
- `width` mirrors the gap the arrow stands in:

  | direction | width | height |
  |---|---|---|
  | `up` / `down` | 1 (unchanged) | `GAP_HEIGHT` |
  | `left` / `right` | 4 (matches `Space("right")`) | `GAP_HEIGHT` |

  Vertical arrows keep width 1 — `_outline_arrow` sizes its pixels from
  `placement.width * cell_width`, so a 0-width arrow would render nothing and
  regress the vertical case. Heights already match (`Space` and `Arrow` are
  both `GAP_HEIGHT`), so the arrow occupies exactly the gap's cell on both
  axes and no box can move.

### `render.py`

- `TerminalRenderer._draw_arrow` gains `ARROW_LEFT` (`←`) and `ARROW_RIGHT`
  (`→`) and looks the glyph up in a dict keyed by direction, keeping the
  mapping total over all four values. (`render()` still does not call it;
  arrows leave the text grid blank so the sprite shows through. Left in place
  rather than deleted — that is outside this story.)
- `GraphicsRenderer` reuses one piece of head geometry for all four
  directions by transposing, rather than growing a mirrored horizontal
  branch that could drift from the tested vertical one. `_on_arrow` is
  reparameterised from screen axes to arrow-relative ones:

  `_on_arrow(along, across, length, centre, reach, head_at_far_end, thickness)`

  with the body otherwise unchanged. `_outline_arrow` then supplies:

  | | along | across | length | thickness | `head_at_far_end` |
  |---|---|---|---|---|---|
  | vertical | `y` | `x` | pixel height | `cell_height` | `direction == "down"` |
  | horizontal | `x` | `y` | pixel width | `cell_width` | `direction == "right"` |

  `reach = min(centre, extent - 1 - centre)` over the across-extent. So
  `down`/`right` put the head at the far end and `up`/`left` at the near end,
  and the direction field collapses to (axis, sign) at this one call site.

### Collaborators

`state.axis` is the only shared piece of new knowledge: `handle_command` uses
it to derive an arrow's direction, `layout` uses it to switch axis, and
`layout.width` uses the same `HORIZONTAL` set to size the cell. `render`
depends only on the direction value itself.
