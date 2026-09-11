# 028 - Centre a parent between its children

## Story

Bob presses `b` three times off one box and gets three children stacked to the
right. Instead of the parent clinging to the top of the stack, it sits level
with the middle of them, the arrow leaving straight out of its side and
branching evenly up and down.

## Acceptance Criteria

- A parent sits midway between its first child and its last child.
- With an odd number of children, the parent lines up level with the middle
  child.
- With an even number of children, the parent sits in the gap between the two
  middle children, filling it exactly.
- Centring is measured against the children themselves, not against their
  descendants — a child with a tall subtree of its own does not pull the parent
  towards it.
- The vertical gap between boxes is 3 rows, everywhere boxes are stacked.

## Technical Design
Rows are counted in **half-slots**. A half-slot is `BOX_HEIGHT` rows, so with
`GAP_HEIGHT` raised from 2 to 3 a full slot is `ROW_PITCH = 6` and a half-slot
is exactly 3 — the height of a box and the height of a gap. Two boxes stacked
in a column sit 2 half-slots apart; a parent centred between an even number of
children lands on an odd half-slot, which drops it exactly into the 3-row gap.

Centring never needs a finer unit than a half-slot, because a parent's offset
from its children is `pitch * (n - 1) / 2` and `pitch` is always even.

## Measuring: what a node reports upwards

`Measured.span` (a leaf count) is replaced by how far the node's subtree reaches
either side of the node's **own** row, plus the pitch it chose for its children:

```
Measured(box, width, above, below, pitch, children)
```

- A leaf reports `above = 0`, `below = 0`, `pitch = 0`.
- A parent picks `pitch` as the smallest **even** number satisfying
  `pitch >= below(child_i) + above(child_i+1) + 2` for every adjacent pair, and
  `pitch >= 2`. The `+ 2` keeps neighbouring blocks a full slot apart, so no two
  boxes in a column ever touch.
- With `half = pitch * (n - 1) // 2`, the parent reports
  `above = half + above(first child)` and `below = half + below(last child)`.

The pitch is uniform across siblings: a tall subtree stretches all its siblings
to match, leaving holes in the shorter columns. That is the price of keeping the
parent on a whole half-slot, and we accept it.

Because `above` and `below` are measured from each child's own row, a child with
a deep subtree of its own does not drag the parent towards it — the parent is
centred on the children, not on their descendants.

## Placing: what a node pushes downwards

The flow inverts. Today a parent takes the block start it is handed and deals
out block starts to its children. Now the parent is placed **first**, and its
children hang off it:

```
child_i.row = parent.row - half + i * pitch
```

`cells` reads `pitch` straight off the `Measured` node rather than re-deriving
it. A root is pushed down with `row = 0`, so children above it take negative
rows; `forest` then normalises the whole forest by subtracting the minimum row,
leaving the topmost box at 0.

Top-level trees pack tightly against one another —
`next.row = prev.row + prev.below + 2 + next.above` — rather than sharing a
pitch. There is no parent above them for a uniform pitch to serve.

`position` becomes `y = top + node.row * HALF_PITCH`, and `layout` derives
`total_height` from the normalised maximum row.

## Arrows

An arrow's stops are offsets from the placement's top, and the renderer assumes
`stops[0]` is the shaft. Both break once the parent sits in the middle of its
children rather than level with the first.

The arrow's origin moves to the **topmost stop**, keeping every offset inside
the sprite, and `Arrow` gains an explicit `shaft`:

```
Arrow(stops, shaft)
```

`emit` sets the placement's `y` to the first child's centre row, its `height` to
`stops[-1] - stops[0] + 1`, and `shaft` to the parent's centre row relative to
that same origin. `_outline_arrow` takes `shaft_row` from `shaft` instead of
`stop_rows[0]`, and strokes the trunk from `min(stop_rows)` to `max(stop_rows)`.
The sprite cache key in `render.py` must include `shaft` alongside `stops`.
