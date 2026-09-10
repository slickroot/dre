# 019 - Move the selection to the box above

## Story

Bob has a box with another box beside it on the left, and nothing above. He
presses `k` and the selection stays put. With a box stacked overhead, `k` takes
him up to it.

## Acceptance Criteria

- `k` moves the selection to the box directly above it.
- `k` with no box directly above it leaves the selection where it is.
- `k` on a canvas with no boxes does nothing.

## Technical Design

`k` is the fourth and last arm. 015 shipped `beside` hardcoded to "right" and
deferred generalising "until a second direction needs it"; 017 gave `beside` a
`step` and took the second arm; 018 wrote `below` as its own hardcoded copy and
handed the question on — "019 is next and gets to decide whether a third
direction earns the generalisation."

It does. With all four arms present, `beside` and `below` are the same five
guards differing in one string and one sign, and `k` differs from `below` only
in that sign. Rather than grow a fourth copy or give `below` a `step` to match
`beside`, the two collapse into one function taking a direction.

### One direction vocabulary

The collapsed helper needs the axis of the direction it was *asked for*, while
the slot guard still needs the axis of the *node* it finds. Today `axis` takes
the node. A `Space` or an `Arrow` does not have a different kind of direction —
it holds one — so `axis` takes the direction and callers unwrap:

```python
def axis(direction: Direction) -> Literal["row", "col"]:
    return "col" if direction in HORIZONTAL else "row"
```

One rule, one input type, both sides of the guard. The alternative — an
`axis(Union[Space, Arrow, Direction])` opening with an `isinstance`, or a
second `axis_of` beside it — buys nothing that `.direction` at the call site
doesn't already give, and any version that states "these directions are
horizontal" twice is the bug that `HORIZONTAL` exists to prevent.

Four call sites unwrap. `state.py`'s `a` clause becomes
`axis(state.nodes[slot].direction)`. `layout.py` (which imports it as
`node_axis`) unwraps at `walk` and at `width`. `render.py` gets *shorter*: it
already unpacks `direction = placement.node.direction` on the line above, so
`axis(placement.node)` becomes `axis(direction)`.

### `move` — one helper, four arms

`beside` and `below` are deleted. So is the old list-order `move`: `j` left it
in 018, `k` leaves it here, and it has no other caller. The name is freed in
the same commit and reused:

```python
def move(nodes: List[Node], selected: int, direction: Direction) -> int:
    step = -1 if direction in ("up", "left") else 1
    slot, target = selected + step, selected + 2 * step
    if not 0 <= target < len(nodes):
        return selected
    if not isinstance(nodes[slot], (Space, Arrow)):
        return selected
    if axis(nodes[slot].direction) != axis(direction):
        return selected
    if not isinstance(nodes[target], Box):
        return selected
    return target
```

`move` is the better name for what this returns. `neighbour` would be a lie:
the function answers with `selected` when there is nothing there, and "the
neighbour above, or you, if there is no neighbour" is not what a neighbour is —
a truthful `neighbour` returns `None` and makes every caller unwrap it. "Move
up, and if you can't, stay put" is exactly the refusal semantic, and it is the
verb every spec in this series is titled with.

The parameter is a `Direction`, not an `(axis, step)` pair: the call site then
says the thing the keystroke means, and the incoherent pairings — `("col", -1)`
for `k` — are unrepresentable rather than merely unwritten. It is annotated and
has no default. Old and new `move` share an arity and their first two
parameters, so a stale `move(nodes, 0, 1)` would not crash — `1 in HORIZONTAL`
is `False`, `axis` returns `"row"`, `step` falls to `1`, and the call silently
behaves as "down". The annotation makes that a type error instead of a quiet
wrong answer, the same reasoning 017 used to refuse `beside` a default step.

The bound guard is `beside`'s two-sided `not 0 <= target < len(nodes)`, not
`below`'s one-sided `target >= len(nodes)`. `below` could only ever fall off
the far end; going up it can fall off the near end, where Python wraps rather
than raises, and `State`'s default `selected == -1` would otherwise compute
`target == -3` and select a real box. Adopting the wider guard is
behaviour-neutral for `j`, whose `target` is never negative.

`move` keeps the index-local adjacency the whole series has used — the slot at
`selected ± 1`, the box at `selected ± 2` — rather than reasoning over board
coordinates from `layout.walk`. Nothing here changes that trade.

### `handle_command` — one clause

The `h`/`l` clause and the `j`/`k` clause become one, the regrouping 017
predicted and deferred to "018 and 019". A module-level table sits beside
`HORIZONTAL`:

```python
DIRECTIONS: Dict[str, Direction] = {
    "h": "left",
    "j": "down",
    "k": "up",
    "l": "right",
}
```

```python
if key in DIRECTIONS:
    if not state.nodes:
        return state
    return replace(
        state, selected=move(state.nodes, state.selected, DIRECTIONS[key])
    )
```

The membership test and the lookup read off one table, so a key cannot be
dispatched to a direction it is not mapped to — which a hand-kept
`if key in ("h", "j", "k", "l")` plus a four-arm conditional could drift into.

The table is the seam between what the user pressed and what the model calls
that direction, and it is why `move` takes `"up"` rather than `"k"`. The same
`Direction` values live inside `Arrow` and `Space`, where `render.py` reads
them to point an arrowhead and `layout.py` reads them to advance the turtle.
Letting `move` take key letters would mean either two membership rules inside
`axis` or renaming the model's directions to vim keys, with the renderer
drawing an arrowhead for `"j"`.

The clause sits where the `h`/`l` clause sits now, under the `pending == "b"`
block, so the four movement keys read as one block instead of straddling `i`.
`bk` and `bh` are unaffected: `pending == "b"` returns first, and `bj`/`bl`
still append without reaching here.

Only `selected` changes. `nodes`, `mode`, `running`, `source` and `pending`
ride through `replace` untouched, so `k` cannot disturb an arrow in progress.
`handle_key` dispatches to `handle_insert` first, so `k` in insert mode still
types the letter `k`.

### Tests

`BesideTest` and `BelowTest` merge into one `MoveTest`, each case gaining its
explicit direction. There is one function now, so "what does `move` refuse?"
should have one answer in one place. Following 017, the direction-blind guards
are proved once rather than four times — a non-separator in the slot, and a
target past the end — while the per-arm facts are repeated per direction:
crossing the right axis, and refusing the wrong one.

Upward, the arms that are genuinely new:

- Crosses a `Space("down")` upward: `move(nodes, 2, "up") == 0`.
- Crosses an `Arrow("down")` upward — axis, not arrowhead.
- Refuses across a `Space("right")`; the selection stands.
- Refuses on the top box, where `target` is `-2`.

`MoveSelectionTest` is the existing `k` block driven through `handle_key`; it
is renamed `MoveSelectionUpTest`, mirroring the `Right`, `Left` and `Down`
blocks. Most of it survives the change untouched, which is worth knowing before
rewriting it:

- `test_k_skips_a_space_and_lands_on_the_previous_box` still lands on `2` —
  slot 3 is a `Space("down")`, target 2 is a `Box`. Only its name is wrong now:
  it describes list-order skipping, not "above". It is renamed for the
  behaviour it actually pins.
- `test_k_on_the_top_box_keeps_the_selection` — `target == -2`, refused by the
  widened bound.
- The empty-canvas, mode/running/nodes, non-mutation, `source`, colour and fill
  guarantees all still hold.
- `test_k_leaves_fills_unchanged` uses `[Box, Box]` with no separator between
  them, so the slot holds a `Box` and `k` refuses either way.

One case is added, the acceptance criterion 018 added for `j` and the story's
opening line — Bob with a box to his left and nothing above:

- `k` does not cross a `Space("right")`.

Elsewhere, `PressATest`'s
`test_a_over_an_existing_arrow_replaces_it_flipping_direction` walks `k` across
an `Arrow("down")` to build an `Arrow("up")`. It still lands on `0`: axis, not
arrowhead. No change.

### Unchanged

`i`, `c`, `f`, `a`, `b`, `edit`, `handle_insert`, and every node type. `Box`,
`Space`, `Arrow`, `Cursor`, `HORIZONTAL` and `Direction` keep their
definitions. `writer.py` and `kitty.py` are not touched. `layout.py` and
`render.py` change only where they unwrap `.direction` for `axis`; nothing
about the board or the drawing changes, only which index is selected.

### Out of scope

- **No board-coordinate lookup via `layout.walk`.** `move` stays index-local,
  as `beside` and `below` were. A real 2D lookup waits until this check fails a
  case that matters.
- **021's bracket traversal starts from nothing.** 021's design leans on the
  old `move` — "`move` already skips non-`Box` nodes, so `j`/`k` step over
  `Push`/`Pop`". That line was written when `j`/`k` were list-order and 018
  already broke its premise. Whatever 021 needs, it builds against the new
  `move`, which knows nothing about brackets.
- **`selected == -1` is guarded but untested.** The widened bound refuses it,
  and the acceptance criteria still say nothing about a canvas with no
  selection. Whoever gives "nothing selected" a defined meaning should pin it
  then. 017 left the same note.
