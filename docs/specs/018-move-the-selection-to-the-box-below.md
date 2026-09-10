# 018 - Move the selection to the box below

## Story

Bob has a box with another box beside it on the right, and nothing below. He
presses `j` and the selection stays put — `j` no longer carries him sideways.
With a box stacked underneath, `j` takes him down to it.

## Acceptance Criteria

- `j` moves the selection to the box directly below it.
- `j` with no box directly below it leaves the selection where it is.
- `j` on a canvas with no boxes does nothing.

## Technical Design

### `below` — new helper in `state`, sibling of `beside`

```python
def below(nodes: List[Node], selected: int) -> int:
    slot, target = selected + 1, selected + 2
    if target >= len(nodes):
        return selected
    if not isinstance(nodes[slot], (Space, Arrow)):
        return selected
    if axis(nodes[slot]) != "row":
        return selected
    if not isinstance(nodes[target], Box):
        return selected
    return target
```

Same reasoning as `beside`, same shape, hardcoded to "down": `b` always appends
`[Space, Box]` after the *last* node, regardless of what's selected, so the
node list is a single append-only chain. "The box below" is never more than
the immediate next pair in the list — `selected + 1` for the separator,
`selected + 2` for the box. `below` inherits `beside`'s narrow, index-local
adjacency check rather than reasoning over board coordinates from
`layout.walk`. A real 2D lookup stays out of scope until this check fails a
case that matters, the same deferral `beside` made for `l`.

This is the second direction after `beside`, which explicitly deferred
generalising into a shared `step(nodes, selected, direction)` "until a second
direction needs it." It still doesn't: `below` stays its own hardcoded copy
rather than a parameterised call. 019 is next and gets to decide whether a
third direction earns the generalisation.

### `handle_command`

The combined `if key in ("j", "k")` branch splits in two. `j` gets its own
clause, mirroring `l`'s:

```python
if key == "j":
    if not state.nodes:
        return state
    return replace(state, selected=below(state.nodes, state.selected))
if key == "k":
    if not state.nodes:
        return state
    return replace(state, selected=move(state.nodes, state.selected, -1))
```

`k` keeps calling `move` exactly as today — spec 019 owns making it spatial.
`pending == "b"` still returns first, so `bj` is unaffected.

### Tests

`below` directly, mirroring `BesideTest`:

- Crosses a `Space("down")` to the box below it.
- Crosses `Arrow("down")` and `Arrow("up")` alike — axis, not arrowhead.
- Refuses across a `Space("right")`; the selection stands.
- Refuses on the bottom box, where `target` is past the end.
- Refuses when the slot holds a `Box`.

Through `handle_key`, mirroring `l`'s block. The existing list-order `j`
tests (`test_j_skips_a_space_and_lands_on_the_next_box`,
`test_j_on_the_bottom_box_keeps_the_selection`, and the rest of that block)
described `move()`-based order, not spatial "below" — they no longer hold and
are rewritten:

- `j` moves the selection to the box directly below it.
- `j` with nothing directly below it keeps the selection.
- `j` on an empty canvas returns the state unchanged.
- `j` preserves the mode, the running flag and the nodes.
- `j` preserves `source` and does not mutate the given state.
- `j` leaves colours and fills unchanged.
- `j` in insert mode types the letter `j`.
- `j` does not cross a `Space("right")` — the horizontal-axis mirror of `l`'s
  refusal across `Space("down")`.
- `bj` still appends a `Space("down")` and a box — the pending branch wins.

`k`'s existing tests are untouched; it still exercises `move()`.

### Unchanged

`beside`, `move`, `k`, `i`, `c`, `f`, `a`, `b`, `edit`, `handle_insert`, and
every node type. `layout.py`, `render.py`, `writer.py` and `kitty.py` are not
touched.

### Out of scope

- **No generalised `step(nodes, selected, direction)`.** Still deferred, as
  it was in 015.
- **No `h`.** Spec 017.
- **`k` stays list-order.** Spec 019 makes it spatial.
- **No board-coordinate lookup via `layout.walk`.** `below` is index-local,
  same as `beside`.
