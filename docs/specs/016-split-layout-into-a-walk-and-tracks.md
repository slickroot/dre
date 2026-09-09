# 016 - Split layout into a walk and tracks

## Motivation

`layout()` is a pure function of `(state, cols, rows)` and should stay one.
Inside, it does five jobs at four levels of abstraction with no seams between
them: measuring nodes, walking the flat node list into board positions, sizing
rows and columns, centring everything on the terminal, and synthesising the
cursor.

The walk is the part that carries all the design decisions — separators own
their own position, the turtle keeps its column when it turns, an arrow's
direction is decorative and only its axis moves the turtle — and it is the part
with no name and no direct tests. Twelve lines of comment stand in for both.

Two consequences to remove:

- A `single_column` branch centres a node on the terminal instead of on its
  column, guarded by a comment that calls it a behaviour-preserving fallback.
  It is not a second layout model. It compensates for flooring twice: once in
  `x0 = (cols - total_width) // 2` and again in `(col_widths[c] - node_width)
  // 2`. Two floors on a track and a node of different parity lose up to a
  cell.
- `state.mode == "insert"` is tested in two places, once to widen the box being
  edited and once to place the cursor inside it, with the two kept in agreement
  by hand.

This is a refactoring. There is no user story and no observable change.

## Acceptance Criteria

- Every existing test passes, unedited. No test may be changed or deleted to
  accommodate this work; a failure means the refactoring is wrong, not the
  test.
- `layout(state, cols, rows) -> List[Placement]` keeps its signature and stays
  pure.
- `walk` is tested directly on coordinates, not through screen positions.
- No `single_column` branch, and no other special case keyed on the shape of
  the board.

## Technical Design

Verified against the suite before writing: the design below is a working
`layout.py` under which all 263 tests pass unedited.

### Vocabulary

`Cell` and `Grid` are taken. `render.py:28-29` uses them for a character on
screen and the screen itself, and `cell_width` / `cell_height` in `render.py`
and `writer.py` are that cell's size in pixels. Those are the correct meanings;
layout uses different words.

```python
@dataclass(frozen=True)
class Coordinate:
    row: int
    col: int


@dataclass(frozen=True)
class Track:
    offset: int
    extent: int
```

A *coordinate* is a position on the board and always spells its fields
`row`/`col`. A *placement* carries screen `x`/`y`. The field names keep the two
spaces apart at every use site, which is what `Cell` could not do. Existing
specs use "coordinates" loosely for screen positions; from here it means the
board.

### `walk`

```python
def walk(nodes: List[Node]) -> List[Coordinate]:
    coordinates = [Coordinate(0, 0)]
    row, col, active = 0, 0, "row"
    for node in nodes[1:]:
        if isinstance(node, (Space, Arrow)):
            active = node_axis(node)
        if active == "row":
            row += 1
        else:
            col += 1
        coordinates.append(Coordinate(row, col))
    top = min(c.row for c in coordinates)
    left = min(c.col for c in coordinates)
    return [Coordinate(c.row - top, c.col - left) for c in coordinates]
```

The body is today's loop, unchanged. It takes no `cols` or `rows`: the board is
independent of the terminal. The return is index-aligned with `nodes`.

The normalising tail is new and is `walk`'s postcondition: the minimum row and
the minimum column are both 0. `tracks` allocates a list per axis and indexes it
by row and column, so it requires positions that are non-negative and
zero-based. Making that a guarantee of `walk` rather than an accident of the
loop is what lets `walk` be tested on its own. It is a no-op today, because
every step is `+= 1`; it stops being one when a direction is added that steps
backwards, and then `tracks`, `centre` and the renderers do not have to learn
that negative positions ever existed.

Steps stay `+= 1`. Nothing here anticipates directions that do not exist yet.

### `tracks` and `centre`

One function sizes an axis, used for both:

```python
def tracks(extents: List[int], indices: List[int]) -> List[Track]:
    sizes = [0] * (max(indices) + 1)
    for extent, index in zip(extents, indices):
        sizes[index] = max(sizes[index], extent)
    offset = 0
    laid = []
    for size in sizes:
        laid.append(Track(offset=offset, extent=size))
        offset += size
    return laid


def span(laid: List[Track]) -> int:
    return sum(track.extent for track in laid)
```

A track is as wide as the widest node in it — alignment is global, so two boxes
in one column share a centre line whatever their labels, and a vertical arrow
between them is straight.

Positioning becomes one expression per axis, and the `single_column` branch
goes away:

```python
def centre(track: Track, extent: int, available: int, total: int) -> int:
    return (available - total + 2 * track.offset + track.extent - extent) // 2
```

`2 * track.offset + track.extent` is twice the track's centre, so the centre
line is exact in half-cell units and the one `// 2` is the only rounding. A
track and a node of different parity now round together, which is what the
special case was for.

The two behaviours that the branch appeared to protect both survive, for the
same reason they always held:

- A box gaining the cursor's column shifts left, so
  `test_each_box_is_centered_on_its_own_width` still sees two different `x`
  values (4 and 3 at `cols=11`) — a node is centred on its track's centre line,
  not left-aligned to the track.
- The arrow in `test_an_arrow_is_placed_in_the_slots_row_at_the_centre_column`
  still lands at `x = 5`.

### `editing` and `interior`

One predicate replaces both `state.mode == "insert"` tests:

```python
def editing(state: State, index: int) -> bool:
    return state.mode == "insert" and index == state.selected
```

`interior` loses its branch and gains the reason it never had one:

```python
def interior(label: str, editing: bool) -> int:
    return max(len(label) + int(editing), 1)
```

Identical to the current pair of returns at every input. It also makes the rule
legible: an empty box already reserves one column, so it does not widen when
you start typing in it — only a box with a label does.

`width(node, editing)` keeps its signature. The cursor genuinely widens the box
it sits in, so that is a measurement fact, not a caller's concern.

### `cursor`

```python
def cursor(state: State, placements: List[Placement]) -> List[Placement]:
    if state.selected < 0 or not isinstance(state.nodes[state.selected], Box):
        return []
    box = placements[state.selected]
    label = box.node.label
    x = box.x + interior(label, editing(state, state.selected))
    return [Placement(Cursor(), x=x, y=box.y + 1, width=1, height=1)]
```

The mode branch on `x` collapses into `interior`, which is the same arithmetic
seen from the other side: the cursor sits at the right-hand edge of the
interior, and in insert mode the interior is one wider. Measuring the box and
placing the cursor in it can no longer disagree, because they are now one
expression.

Returning a list rather than an optional keeps the composition below flat.

### `layout`

```python
def layout(state: State, cols: int, rows: int) -> List[Placement]:
    if not state.nodes:
        return []

    coordinates = walk(state.nodes)
    widths = [
        width(node, editing(state, index)) for index, node in enumerate(state.nodes)
    ]
    heights = [height(node) for node in state.nodes]

    columns = tracks(widths, [c.col for c in coordinates])
    rows_ = tracks(heights, [c.row for c in coordinates])
    total_width, total_height = span(columns), span(rows_)

    placements = [
        Placement(
            node,
            x=centre(columns[at.col], node_width, cols, total_width),
            y=centre(rows_[at.row], node_height, rows, total_height),
            width=node_width,
            height=node_height,
        )
        for node, at, node_width, node_height in zip(
            state.nodes, coordinates, widths, heights
        )
    ]
    return placements + cursor(state, placements)
```

Measure, walk, size, place, decorate — each nameable in a word, none of them
inspecting the others' reasoning.

### Tests

Existing tests are untouched. New tests cover `walk` directly, in coordinates,
replacing the comment block:

- A lone node sits at `(0, 0)`.
- Coordinates are index-aligned with the nodes, separators included.
- A `Space("down")` and the box after it each take their own row.
- A `Space("right")` and the box after it each take their own column, on the
  same row.
- Turning down after turning right keeps the column: `[Box, Space("right"),
  Box, Space("down"), Box]` gives `(0,0) (0,1) (0,2) (1,2) (2,2)`. This is the
  coordinate form of `test_a_box_below_the_last_box_in_column_one_stays_in_
  column_one`.
- An `Arrow` moves the turtle exactly as the `Space` it replaced did, on both
  axes.
- `Arrow("left")` advances the column, like `Arrow("right")`. Direction chooses
  a glyph; only axis moves the turtle. Today this is inferable only from
  pixels.
- The minimum row and minimum column are both 0.

Plus `tracks` on its own: a track takes the extent of its widest member, and
offsets are the running sum of the extents before it.

### Collaborators

None outside `layout.py`. `state.py` is untouched; `render.py` and the writers
consume `Placement`, whose shape and contents do not change. `Coordinate` and
`Track` stay internal to layout — nothing exported and nothing rendered knows
the board exists.
