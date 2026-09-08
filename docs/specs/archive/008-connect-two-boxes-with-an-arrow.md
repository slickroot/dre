# 008 - Connect two boxes with an arrow

## Story

Bob has two boxes. In command mode he presses `a` on the top one, presses `j` to
move down to the second, and presses `a` again — a `↓` appears in the blank row
between them, pointing from the first box to the second.

## Acceptance Criteria

- In command mode, `a` on the selected box marks it as the arrow's source.
  Nothing on screen changes.
- Moving with `j` or `k` and pressing `a` again draws an arrow from the source
  box to the box the selection has landed on.
- An arrow to the box below draws as `↓`; an arrow to the box above draws as `↑`.
- The arrow sits in the existing blank row between the two boxes, horizontally
  centered like the boxes are. Box spacing is unchanged.
- Pressing `a` on a box that is not directly above or below the source does
  nothing.

## Technical Design
An arrow is a node, sitting in `state.nodes` between the two boxes it joins.
That single decision drives everything else: the list order *is* the vertical
order, so an arrow's position already says which boxes it connects, and the only
thing left to store is which end the head is on.

The gap between boxes stops being arithmetic and becomes a node too — `Space` —
so that creating an arrow is a **replacement**, not an insertion. This is the
design's main result and the reason for most of what follows.

### `Space`, and why the gap is a node

`layout()` currently derives every position from a node's index:

```python
total = len(state.nodes) * BOX_HEIGHT + (len(state.nodes) - 1) * GAP
y = top + index * (BOX_HEIGHT + GAP)
```

Both lines assume every element is a 3-row box. Putting a 1-row arrow in the
list breaks the proportionality between index and offset, and the obvious repair
— let an arrow occupy the blank row by advancing `y` by nothing, reading
`y - GAP` — works but leaves "box spacing is unchanged" as a claim held up by
arithmetic that has to be kept in agreement.

Making the gap an explicit node removes the special case instead of encoding it:

```python
total = sum(height(node) for node in state.nodes)
y = (rows - total) // 2
for index, node in enumerate(state.nodes):
    ...
    y += height(node)
```

No `GAP` constant, no `len(nodes) - 1`, no per-type branch in the loop body. It
also fixes a latent bug in passing: an empty canvas currently computes
`total = 0 * 3 + (-1) * 1 == -1`, where `sum` of nothing is `0`.

The payoff that actually decided it: an `Arrow` is 1 row and so is the `Space` it
replaces, so `nodes` never changes length when an arrow is drawn. No index below
it shifts, `selected` needs no fix-up, and a `source` index recorded on the first
`a` cannot go stale before the second. "Box spacing is unchanged" becomes
structural rather than computed.

The price, named honestly: every mutation of `nodes` must keep the alternating
rhythm, and every existing test that writes `State([Box("a"), Box("b")])` now
describes a canvas with no gap and must be rewritten.

**Invariant:** `nodes` alternates `Box`, slot, `Box`, slot, `Box`… where a slot
is a `Space` or an `Arrow`. It never leads or trails with a slot, and never has
two slots in a row. `b` is the only thing that grows the list and `a` never
changes its length, so the invariant has exactly one place to be upheld.

### Nodes

```python
Direction = Literal["forward", "backward"]

@dataclass(frozen=True)
class Box:
    label: str = ""

@dataclass(frozen=True)
class Space:
    pass

@dataclass(frozen=True)
class Arrow:
    direction: Direction = "forward"

Node = Union[Box, Space, Arrow, Cursor]
```

`Box` stores its label, `Arrow` stores its direction: content, in both cases,
with position and size derived by `layout()` every frame. `Space` stores nothing
and exists to occupy a row, like `Cursor` exists to mark one.

### Why `direction` is list-relative and not `"up"`/`"down"`

Direction cannot be derived from coordinates. Both arrows between the same pair
of boxes occupy the *same* row at the *same* centred column — `↓` from the top
box and `↑` from the bottom box are geometrically identical 1×1 placements.
Direction is a fact about the order Bob pressed `a`, and that order is gone the
moment the keystrokes are handled. It has to be stored.

But `"down"` is screen vocabulary in `state.py` — the same category error as
storing the glyph `"↓"`, and the same line 007 draws when it keeps ANSI codes out
of `Box.colour`. Turn the canvas sideways and a stored `"down"` is simply false.

`"forward"` means the head is on the *later* node in `nodes`. That is the
document's own vocabulary, so it survives any axis: the vertical layout renders
`forward → ↓` and `backward → ↑`, and a future horizontal layout renders the
same two stored values as `→` and `←` with no state change. The axis is a
property of the canvas, resolved where the glyph is chosen.

### Rejected: signed height

Encoding direction as the sign of `Placement.height` (`1` down, `-1` up) was
considered and rejected. It does not remove the stored fact — `layout()` is a
pure function of `State`, so something in `State` still has to say which way, now
spelled as a sign instead of a name. It makes `height` mean "extent, except when
it means direction", so every reader needs `abs()`: `_draw_box` computes
`bottom = y + height - 1`, and the layout cursor's `y += height(node)` would walk
*backwards* a row and lift every box below the arrow. It fails the same rotation
test that killed `"down"`, since a sideways arrow still has height 1 and the sign
would have to migrate to `width`. And the renderer branches identically either
way — `height > 0` instead of `direction == "forward"` — with an obscured source.

An arrow does get a plain 1×1 `Placement` like any other node. Only the sign
trick is rejected.

### Rejected: an arrow that knows its two boxes

Storing endpoints was considered, motivated by a future horizontal layout. It
was rejected because list-relative direction already covers rotation for free,
and endpoints cost a great deal more. `Box` is a frozen dataclass with value
equality, so `Box("a") == Box("a")` and two same-labelled boxes are literally
indistinguishable — endpoints would require `Box.id`, something to mint ids,
`edit()` preserving them, and every existing test rewritten, which is the work
006 looked at and deliberately deferred. Indices instead of ids rot on the first
delete or reorder. And it creates two sources of truth: the arrow's position in
`nodes` already asserts which boxes it joins, so stored endpoints can contradict
it.

The tension is real and worth recording: an arrow that owns its endpoints does
not want to be *in* the list at all — it wants to be a relation that `layout()`
positions by looking up its endpoints' placements. That is a coherent design,
and it is the one to reach for when a story needs arrows that skip boxes or
survive reordering. It is not this one.

### Components

**`State`** (`state.py`) — gains `source: int = -1`, the index in `nodes` of the
box marked as the arrow's source, `-1` for "no arrow in progress". Same shape and
sentinel as `selected`, and on `State` rather than on `Box` for the same reason:
it is caret-like, not document content.

**`handle_command()`** (`state.py`) — learns `a`; `b` and `i` now clear `source`.

**`height()`, `width()`** (`layout.py`) — free functions dispatching on node
type. Geometry stays in the layout module and `state.py` keeps knowing only
content; `interior(label, editing)` is already exactly this shape and `width()`
absorbs it.

**`layout()`** (`layout.py`) — walks `nodes` with a running `y` cursor, emits one
placement per node, and emits the cursor placement only when the selected node is
a `Box`.

**`TerminalRenderer`** (`render.py`) — gains `_draw_arrow` alongside `_draw_box`
and `_draw_cursor`. `Space` needs no branch: the `isinstance` chain simply does
not match it, so it draws nothing.

**`writer.py`**, **`Placement`**, **`Cursor`** — unchanged.

### Geometry

```python
BOX_HEIGHT = 3
BORDERS = 2

def height(node: Node) -> int:
    if isinstance(node, Box):
        return BOX_HEIGHT
    return 1

def width(node: Node, editing: bool) -> int:
    if isinstance(node, Box):
        return interior(node.label, editing) + BORDERS
    if isinstance(node, Arrow):
        return 1
    return 0
```

Every node is centred by the one rule already in use, `x = (cols - width) // 2`,
so an arrow lands at `(cols - 1) // 2` — the centre column, which is where a box
centred on the same canvas puts its own middle. "Horizontally centred like the
boxes are" needs no separate code path. A `Space` has width `0` and is centred
too; it draws nothing, so its `x` is inert.

`GAP` is deleted. `Space` replaces it.

**Placement indices match node indices.** `layout()` emits a placement for every
node including `Space`, in order, so `placements[state.selected]` still resolves
to the selected node's placement. The cursor placement is appended after the
loop, as today, so it does not disturb the correspondence.

### The cursor and selection

`j` and `k` move between **boxes only**. Slots are skipped:

```python
def move(nodes: List[Node], selected: int, step: int) -> int:
    index = selected + step
    while 0 <= index < len(nodes):
        if isinstance(nodes[index], Box):
            return index
        index += step
    return selected
```

Walking to the next `Box` subsumes 006's `min`/`max` clamp: running off either
end finds no box and returns the selection unchanged, which is exactly "`j` on
the bottom box leaves the selection where it is". The empty-canvas guard stays
where 006 put it, on the way in.

Arrows are therefore not selectable in this story. That was a deliberate call:
making them stops would cost Bob an extra keypress to cross every gap he had
already connected, silently changing box-to-box movement in a story whose
criteria say `j` gets him from one box to the next.

Because selection can only land on a `Box`, `layout()`'s cursor block is guarded
accordingly and the cursor arithmetic from 006 is untouched.

### Pressing `a`

```python
def connect(state: State) -> State:
    if state.selected < 0:
        return state
    if state.source < 0:
        return State(..., source=state.selected)
    if abs(state.selected - state.source) != 2:
        return state
    slot = (state.source + state.selected) // 2
    direction = "forward" if state.selected > state.source else "backward"
    nodes = state.nodes[:slot] + [Arrow(direction)] + state.nodes[slot + 1 :]
    return State(nodes, ..., source=-1)
```

Adjacency is arithmetic on the list, not on coordinates: under the alternating
invariant two boxes are neighbours exactly when their indices differ by 2, and
the slot between them is their midpoint. Direction compares the same two
indices, because list order is vertical order.

Lifecycle of `source`:

- **`a` on an adjacent box** — draws the arrow, then `source = -1`.
- **`a` on a non-adjacent box** — nothing happens and `source` **survives**. A
  stray press two boxes away is not a silent cancel Bob has no way to see.
- **`a` on the source box itself** — falls out of the same rule as a no-op,
  since `abs(i - i) == 0`. There is consequently no way to cancel a pending
  source. `Esc` is unused in command mode and is the obvious home for it; it is
  outside these criteria and left to a later story.
- **`j` and `k`** — `source` survives. This is the story.
- **`b` and entering insert mode** — `source` is cleared. Both change what Bob
  is doing, and `b` additionally grows the list under a pending gesture.
- **An arrow already spanning the slot** — it is overwritten, since the slot is
  replaced either way. Re-running the gesture in the opposite direction flips
  `↓` to `↑` and needs no extra code.

In insert mode `a` is a printable character and `handle_insert` already appends
it to the label, so typing "arrow" works. No branch is needed, exactly as 007
found for `c`.

### `b` keeps the rhythm

```python
nodes = state.nodes + ([Space(), Box("")] if state.nodes else [Box("")])
```

The new box is still `len(nodes) - 1`, so 006's selection rule is unchanged, and
`source` is cleared on the way out.

### Rendering

```python
ARROW_DOWN = "↓"
ARROW_UP = "↑"

def _draw_arrow(self, grid: Grid, placement: Placement) -> None:
    glyph = ARROW_DOWN if placement.node.direction == "forward" else ARROW_UP
    self._put(grid, placement.x, placement.y, (glyph, PLAIN))
```

One cell, through the same `_put` chokepoint that clips everything else. It is
written as a `Cell` — 007 made every grid entry a `(character, colour)` pair, so
a glyph cannot land without a colour. `PLAIN` is right here: these criteria say
nothing about arrow colour, and an arrow is not a box's border. The
`forward`/`backward` to `↓`/`↑` mapping is the *only* place the vertical axis is
assumed, which is what makes a later horizontal layout a change to one function.

### `State` construction

`State` now has five fields, and the existing handlers build it positionally:
`State(state.nodes, state.running, state.mode, state.selected)`. Adding `source`
silently defaults it to `-1` at every one of those sites. That is correct for the
insert-mode handlers — `source` is always `-1` in insert mode, since `i` and `b`
clear it — but correct by accident is exactly the hazard 006 recorded about the
`-1` sentinel. **Handlers should pass `source=state.source` explicitly**, and
the returns are past the point where keyword arguments read better than a row of
positional ones.

### Worked example

Two boxes labelled `a` and `bb`, `cols=11`, `rows=11`, command mode,
`nodes == [Box("a"), Space(), Box("bb")]`. `total = 3 + 1 + 3 = 7`,
`y = (11 - 7) // 2 = 2`.

| index | node | height | width | x | y |
|-------|------|--------|-------|---|---|
| 0 | `Box("a")`  | 3 | 3 | 4 | 2 |
| 1 | `Space()`   | 1 | 0 | 5 | 5 |
| 2 | `Box("bb")` | 3 | 4 | 3 | 6 |

Box `a` occupies rows 2–4, the slot is row 5, box `bb` occupies rows 6–8. Bob
presses `a` with `selected == 0`, so `source = 0`. He presses `j`, which skips
index 1 and lands on index 2. He presses `a`: `abs(2 - 0) == 2`, the slot is
index 1, direction is `forward`. `nodes[1]` becomes `Arrow("forward")`, laid out
at `x = (11 - 1) // 2 = 5`, `y = 5`, and drawn as `↓` — the centre column, in the
blank row that was already there. `source` returns to `-1`.

### Interaction with 007

007 is merged, and this design builds on the code as it actually shipped rather
than as 007 described it. The difference matters in one place: 007's design
proposed *two parallel grids*, characters and colours, with `colour` as a second
required parameter to `_put`. What landed is a **single** grid of
`(character, colour)` pairs, with `Cell` and `Grid` aliases and `_put` taking a
whole cell. `_draw_arrow` follows the shipped shape, as above.

Everything else 007 touched is compatible. `edit()` is already
`replace(nodes[index], label=label)`, so it preserves `colour` and will preserve
any field a later story adds. `handle_command`'s `c` branch reads
`state.nodes[state.selected]` and is safe under this design, because `selected`
can only ever land on a `Box`.

007's claim that "`layout.py` is untouched by this story" was true of 007 —
`layout.py` on `main` is still the 006 version quoted above, `GAP` and all — and
is emphatically false of 008, which rewrites its stacking.

### Tests

`tests/test_state.py`:

- `State([])` starts with `source == -1`
- `b` on an empty canvas appends one `Box`
- `b` on a non-empty canvas appends a `Space` and a `Box`, and selects the `Box`
- `a` on the selected box sets `source` and changes nothing else
- `a` on an empty canvas returns the state unchanged
- `a` on the box below the source replaces the slot with `Arrow("forward")`
- `a` on the box above the source replaces the slot with `Arrow("backward")`
- drawing an arrow leaves `len(nodes)` and every other index unchanged
- drawing an arrow clears `source`
- `a` on a box two boxes from the source does nothing and keeps `source` pending
- `a` on the source box itself does nothing and keeps `source` pending
- `a` over an existing arrow replaces it, flipping direction when reversed
- `j` and `k` preserve `source`
- `b` clears `source`
- `i` clears `source`
- `j` from a box skips the slot and lands on the next box
- `j` on the bottom box and `k` on the top box leave the selection where it is
- `a` does not mutate the given state

`tests/test_layout.py`:

- boxes are spaced exactly as before, now via `Space` — the regression guard for
  "box spacing is unchanged"
- an empty canvas produces no placements and does not compute a negative total
- a `Space` produces a placement of height 1 that draws nothing
- an `Arrow` is placed in the slot's row, 1×1, at the centre column
- an arrow does not move the box below it: the same canvas with `Space` and with
  `Arrow` yields identical box placements
- placement indices correspond to node indices
- `height()` and `width()` per node type
- the cursor placement is emitted for a selected `Box` and its 006 arithmetic is
  unchanged

`tests/test_render.py`:

- `Arrow("forward")` draws `↓` at its placement's cell
- `Arrow("backward")` draws `↑`
- a `Space` placement draws nothing
- an arrow is clipped like any other node when placed off-grid
