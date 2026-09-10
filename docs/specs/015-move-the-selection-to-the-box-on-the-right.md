# 015 - Move the selection to the box on the right

## Story

Bob has two boxes side by side, with the selection on the left one. He presses
`l` and the selection jumps to the box beside it. He presses `i` and types, and
it is that right-hand box that changes.

## Acceptance Criteria

- `l` moves the selection to the box directly beside it on the right.
- `l` with no box directly beside it on the right leaves the selection where it is.
- `l` on a canvas with no boxes does nothing.

## Technical Design

`l` is the first key whose meaning is spatial. `j`/`k` are list-order: `move`
scans indices until it lands on a `Box` and never looks at what it stepped
over, so in `[A, Space("right"), B]` pressing `j` on `A` selects a box that is
drawn to the *right*. This story does not fix that — 018 and 019 own it — but
it does mean `l` cannot be built out of `move`.

### Where "right" comes from

From the flat list, not from board coordinates. `layout.walk` is the only thing
that knows `(row, col)`, and `layout` imports `state`; reaching coordinates
from `state` would mean either inverting that dependency or moving `walk`. We
don't need to. Boxes and separators strictly alternate — `bj`/`bl` always
append `[Space, Box]`, and `a` overwrites a `Space` in place — so the box
beside the selection is at `selected + 2` and the separator between them is at
`selected + 1`. This is the adjacency rule `a` already relies on with
`abs(selected - source) != 2`.

### `beside` — new helper in `state`, sibling of `move`

```python
def beside(nodes: List[Node], selected: int) -> int:
    slot, target = selected + 1, selected + 2
    if target >= len(nodes):
        return selected
    if not isinstance(nodes[slot], (Space, Arrow)):
        return selected
    if axis(nodes[slot]) != "col":
        return selected
    if not isinstance(nodes[target], Box):
        return selected
    return target
```

Hardcoded to "right". No `direction` parameter and no axis argument: only `l`
exists today, and 017–019 can generalise it when they arrive with tests that
exercise the other three arms. Splitting the rule across four early-returns
keeps each refusal in the acceptance criteria readable on its own line.

The slot test is on **`axis`, not `direction`**. 016 settled that an arrow's
direction is decorative and only its axis moves the turtle, so `Arrow("left")`
advances the column exactly as `Space("right")` does — the box after it really
is drawn on the right, and `l` moves to it. A connection the user drew is not a
wall. Reading the arrowhead here would refuse a box that is plainly beside the
selection.

The `isinstance(nodes[slot], (Space, Arrow))` check is load-bearing, not
defensive noise. `axis` reads `.direction`, which `Box` does not have. A state
built as `State([Box("a"), Space("right"), Box("b")])` keeps the default
`selected == -1` — the suite does this throughout — putting a `Box` in the
slot. `j`/`k` survive that by accident because `nodes[-1]` is the last box;
`beside` says out loud that it needs a separator there.

### `handle_command`

A new clause below the `pending == "b"` block, guarded like `j`/`k`:

```python
if key == "l":
    if not state.nodes:
        return state
    return replace(state, selected=beside(state.nodes, state.selected))
```

Placement matters and costs nothing: `pending == "b"` returns early, so `bl`
still appends a right-hand box and never reaches here. Insert mode is
untouched — `handle_key` dispatches to `handle_insert` first, so `l` types the
letter `l`.

Only `selected` changes. `nodes`, `mode`, `running`, `source` and `pending` all
ride through `replace` unchanged, so `l` cannot disturb an arrow in progress.

### Tests

`beside` directly:

- Crosses a `Space("right")` to the box beside it.
- Crosses an `Arrow("right")` and an `Arrow("left")` alike — axis, not
  arrowhead.
- Refuses across a `Space("down")`; the selection stands.
- Refuses on the rightmost box, where `target` is past the end.
- Refuses when the slot holds a `Box`.

Through `handle_key`, mirroring the existing `j`/`k` block:

- `l` moves the selection to the box on the right.
- `l` with nothing on the right keeps the selection.
- `l` on an empty canvas returns the state unchanged.
- `l` preserves the mode, the running flag and the nodes.
- `l` preserves `source` and does not mutate the given state.
- `l` leaves colours and fills unchanged.
- `l` in insert mode types the letter `l`.
- `bl` still appends a `Space("right")` and a box — the pending branch wins.

### Unchanged

`move`, `j`, `k`, `i`, `c`, `f`, `a`, `b`, `edit`, `handle_insert`, and every
node type. `layout.py`, `render.py`, `writer.py` and `kitty.py` are not
touched: nothing about the board or the drawing changes, only which index is
selected.

### Out of scope

- **`j`/`k` stay list-order.** They will still step across a `Space("right")`
  as though it were "down". Specs 018 and 019 own that; fixing it here would
  ship two unwritten stories and rewrite their tests.
- **No `h`.** Spec 017.
- **No generalised `step(nodes, selected, direction)`.** Deferred until a
  second direction needs it.
