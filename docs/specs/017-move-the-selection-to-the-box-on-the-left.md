# 017 - Move the selection to the box on the left

## Story

Bob has two boxes side by side, with the selection on the right one. He presses
`h` and the selection jumps to the box beside it. He presses `i` and types, and
it is that left-hand box that changes.

## Acceptance Criteria

- `h` moves the selection to the box directly beside it on the left.
- `h` with no box directly beside it on the left leaves the selection where it is.
- `h` on a canvas with no boxes does nothing.

## Technical Design

015 shipped `beside` hardcoded to "right" and said 017–019 could generalise it
"when they arrive with tests that exercise the other three arms". 017 brings
the second arm. It is the mirror image of the first — same alternation rule,
same `axis == "col"` slot test, opposite sign — so `h` is `beside` with a step,
not a second function.

### `beside` grows a step

```python
def beside(nodes: List[Node], selected: int, step: int) -> int:
    slot, target = selected + step, selected + 2 * step
    if not 0 <= target < len(nodes):
        return selected
    if not isinstance(nodes[slot], (Space, Arrow)):
        return selected
    if axis(nodes[slot]) != "col":
        return selected
    if not isinstance(nodes[target], Box):
        return selected
    return target
```

`step` is `+1`/`-1`, an index increment, matching `move`'s existing parameter
of the same name and the `step = 1 if key == "j" else -1` the `j`/`k` clause
already writes. The stride stays inside `beside`: the caller says "left", not
"two". A `+2`/`-2` step would read fewer operations in the body at the cost of
teaching every call site that boxes and separators alternate, which is the one
rule this helper exists to own.

The parameter is required, with no default. A default of `1` would encode
"rightward is the unmarked case", which stopped being true the moment `h`
existed, and would let a call site omit the direction and silently get `l`.
The six existing `beside(nodes, 0)` calls gain an explicit `1`.

### The bound guard widens

`if target >= len(nodes)` was one-sided because with a positive step it could
only ever fall off the far end. Leftward it can fall off the near end, and
Python does not raise there — it wraps. On `[Box("a"), Space("right"),
Box("b")]`, `h` at index 0 computes `target == -2` and inspects `nodes[-1]` as
the slot, refusing by luck rather than by rule. Worse, `State`'s default
`selected == -1` gives `target == -3`, which is index 0: a real `Box`, across a
real `Space("right")`. Without a lower bound `h` would return `-3` and set the
selection to a negative index that renders as box `a`.

`if not 0 <= target < len(nodes)` states the rule the guard was always after —
the target must be a real index — and the negative-`selected` case falls out of
it rather than needing a clause of its own. It is behaviour-neutral for `l`,
where `target` is never negative, so no existing test changes meaning.

### `handle_command`

`l`'s clause becomes the pair, in the shape of the `j`/`k` clause below it:

```python
if key in ("h", "l"):
    if not state.nodes:
        return state
    step = 1 if key == "l" else -1
    return replace(state, selected=beside(state.nodes, state.selected, step))
```

A separate `if key == "h"` would duplicate the empty-canvas guard and the
`replace` to no end. The clause stays where `l` put it, under the
`pending == "b"` block: `bh` is not a binding, `pending == "b"` returns early
either way, and `bl` still appends a right-hand box without reaching here.

Only `selected` changes. `nodes`, `mode`, `running`, `source` and `pending`
ride through `replace` untouched, so `h` cannot disturb an arrow in progress.
Insert mode is untouched — `handle_key` dispatches to `handle_insert` first, so
`h` types the letter `h`.

### Tests

The six existing `BesideTest` calls gain their explicit `1`. Three added for
the left arm, one per acceptance criterion the helper can decide:

- Crosses a `Space("right")` leftward: `beside(nodes, 2, -1) == 0`.
- Crosses an `Arrow("right")` leftward — axis, not arrowhead, in the direction
  where the arrowhead is most tempting to read as a wall.
- Refuses on the leftmost box, where `target` is `-2`.

The `Space("down")` and `Box`-in-slot refusals are not mirrored: those lines
are direction-blind and the right arm already proves them.

Through `handle_key`, a short mirror of `MoveSelectionRightTest`:

- `h` moves the selection to the box on the left.
- `h` with nothing on the left keeps the selection.
- `h` on an empty canvas returns the state unchanged.
- `h` in insert mode types the letter `h`.

The immutability, `source`, colour and fill guarantees are not mirrored. They
are about `replace` touching only `selected`, and `h` and `l` now reach that
through the same line; proving it twice would give two tests that can only ever
fail together.

### Unchanged

`move`, `j`, `k`, `i`, `c`, `f`, `a`, `b`, `edit`, `handle_insert`, and every
node type. `layout.py`, `render.py`, `writer.py` and `kitty.py` are not
touched: nothing about the board or the drawing changes, only which index is
selected.

### Out of scope

- **`j`/`k` stay list-order.** They still step across a `Space("right")` as
  though it were "down". 018 and 019 own that.
- **No reordering of the clauses.** The `h`/`l` pair and the `j`/`k` pair read
  a dozen lines apart, an accident of when each was written. 018 and 019 turn
  `j`/`k` spatial and will likely collapse all four into one clause; the
  regrouping belongs there, with the tests that justify it.
- **`selected == -1` is guarded but untested.** The widened bound refuses it,
  and the acceptance criteria say nothing about a canvas with no selection.
  Whoever gives "nothing selected" a defined meaning should pin it then.
