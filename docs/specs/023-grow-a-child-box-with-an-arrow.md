# 023 - Grow a child box with an arrow

## Story

Bob has a box selected. He presses `b` and a new box appears to its right,
already joined to it by an arrow — no separate `a` step to connect them. He
presses `h` to walk back to the parent and `b` again, and a second child appears
below the first, the two arrows branching from a shared trunk.

## Acceptance Criteria

- `b` on an empty canvas creates the first box, with no arrow.
- `b` on a selected box appends a new child to its right, joined by an arrow
  pointing right.
- A second `b` on the same parent places that child below the first, and both
  arrows branch from one shared vertical trunk.
- The new box becomes the selection and the app enters insert mode, as today.
- `h` selects the parent, `l` the first child, `j` the next sibling, `k` the
  previous sibling. A move with no target leaves the selection unchanged.
- `h` on a top-level box leaves the selection unchanged — the canvas is not a
  box.
- `a` is gone; an arrow is parenthood, so there is nothing left to connect.

## Technical Design

The flat `List[Node]` cannot express this story. Arrows live in the slot between
two adjacent list entries, so every box has at most one outgoing edge — a path,
not a tree. `[A, Arrow, B, Arrow, C]` cannot put `C` beside `A`; it puts it
beside `B`. The model becomes a forest.

### Model

`Box` gains children. No new node type.

```python
@dataclass(frozen=True)
class Box:
    label: str = ""
    colour: int = PLAIN
    fill: int = PLAIN
    children: Tuple["Box", ...] = ()
```

Children are not directed. Every child hangs to the right of its parent, so
direction is a property of the layout, not of the data — there is nothing to
store and nothing to choose. `Space` disappears: the only unconnected
neighbours left are top-level boxes, and their gap is layout, not a node.

`State` holds the top-level boxes. There is no root box and no root wrapper,
because the first box can have siblings like any other.

```python
@dataclass(frozen=True)
class State:
    boxes: Tuple[Box, ...] = ()
    running: bool = True
    mode: Mode = "command"
    selected: Tuple[int, ...] = ()
```

`source` goes with `a`. `pending` goes too — it existed only to hold the `b` of
`bj`/`bl`, and `b` is now a single keystroke.

### Selection is a path

`selected` is the sequence of `children` indices that reaches a box, starting
from `State.boxes`. For `boxes=(Box("A", children=(Box("C"), Box("D"))),)`:
`(0,)` is A, `(0, 0)` is C, `(0, 1)` is D.

The empty path `()` is the canvas itself, and it does double duty as "nothing
selected". That is what makes the whole thing uniform:

- `parent(path)` is `path[:-1]`, so the parent of a top-level box is the canvas.
- `b` is `grow(selected)`, and on an empty canvas `selected` is already `()`, so
  creating the first box is not a special case.
- Story 024's `s` is `grow(selected[:-1])` and needs no new machinery.

The cost is that `Box` is frozen, so writing to a path rebuilds every box along
it. Three helpers in `state.py` cover every mutation:

```python
def at(boxes, path) -> Box                  # walk down
def rewrite(boxes, path, fn) -> boxes       # apply fn at path, rebuild the spine
def grow(boxes, path) -> (boxes, path)      # append Box(PAD), return its path
```

`edit`, `c` and `f` become `rewrite` calls instead of list slicing. A future
delete story will invalidate the paths of later siblings; an id per box would
avoid that, and is the escape hatch if it ever bites.

### Navigation

Each key is a pure path transform, returning the path unchanged when the move
has no target — matching what `move`, `beside` and `below` do today.

| key | move | result |
| --- | --- | --- |
| `h` | parent | `path[:-1]`, unchanged if `len(path) == 1` |
| `l` | first child | `path + (0,)` if the box has children |
| `j` | next sibling | `path[:-1] + (path[-1] + 1,)` if in range |
| `k` | previous sibling | `path[:-1] + (path[-1] - 1,)` if in range |

`h` must guard `len(path) == 1`: the parent of a top-level box is `()`, which is
the canvas and not a box, so the selection stays put.

Navigation is in scope because without `h` the selection can never return to a
parent, so `b` could only ever extend one chain and the branching this redesign
exists for would be unreachable.

### Layout

A left-to-right tree, parents top-aligned with their first child.

```
┌─────┐    ┌─────┐
│  A  │──┬─►  C  │
└─────┘  │ └─────┘
         │
         │  ┌─────┐
         ├─►│  D  │
         │  └─────┘
         │
         │  ┌─────┐
         └─►│  E  │
            └─────┘
```

Depth is the column, and a recursive fold assigns rows:

```python
def place(box, depth, row) -> rows_consumed
```

A box sits at `(depth, row)`. Its first child sits at `(depth + 1, row)` — that
is what top-aligned means. Each later child starts where the previous child's
subtree ended. A leaf consumes one row; a parent consumes the sum of its
children, minimum one. Top-level boxes are laid out as if siblings, each
starting where the previous one's subtree ended.

Arrows are no longer nodes in the model, so layout allocates the gaps itself:
box depth `d` takes track column `2d` and the arrow gap after it takes `2d + 1`
with extent 4; box row `r` takes track row `2r` and the gap below it takes
`2r + 1` with extent 2. That feeds the existing `tracks` unchanged — widths
indexed by column, heights by row — and `centre` still centres the whole
diagram in the terminal. Column widths stay global per depth, so every box at
the same depth shares a column width.

`walk`, `beside`, `below`, `move` and `axis` are replaced by the fold and the
path transforms.

### Rendering

Arrows are pixel sprites, not glyphs. `Arrow` stops being a state node and
becomes a layout-only value, emitted once per parent that has children, with its
`direction` field replaced by the rows it must reach:

```python
@dataclass(frozen=True)
class Arrow:
    stops: Tuple[int, ...]   # child centre-rows, relative to the sprite's top
```

Layout emits one `Placement(Arrow(stops), …)` per parent, spanning the gap
column from the parent's centre row down to the last child's centre row.
`_outline_arrow` paints it in three parts: a horizontal shaft at the parent's
row, a vertical trunk at the gap's midpoint, and a stub ending in an arrowhead
at each row in `stops`. `_on_arrow` keeps the existing 30° arrowhead.

With one child, `stops == (0,)`: the trunk has zero length and the sprite
reduces to exactly the straight `──►` drawn today. One drawing path covers both
cases. Because these are pixel sprites rather than box-drawing characters, the
corners are real pixel joins with no seams.

`_outline_arrow` loses its `axis`/`head_at_far_end` branching, since every arrow
now points right. `TerminalRenderer` still draws no arrows — that is today's
behaviour, as its `_draw_arrow` is never called — so `_draw_arrow` and
`ARROW_GLYPHS` are deleted rather than revived.
