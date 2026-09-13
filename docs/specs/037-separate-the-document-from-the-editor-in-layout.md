# 037 - Separate the document from the editor in layout

## Motivation

A second renderer is coming — SVG — and `layout()` is not in a shape it can
use.

`layout(state, cols, rows)` takes a whole `State` and reads exactly two of its
four fields. `running` and `mode` are already invisible to rendering: neither
appears anywhere in `layout.py`, `render.py` or `kitty.py`. `running` is read
only by the loop in `writer.run`, and `mode` only by `handle_key`. Those two
are noise in the signature, not coupling.

The field that genuinely leaks is `selected`, and it leaks in the worst
possible place. `emit` (`layout.py:160`) compares each node's path against it
and yields a `Cursor()` placement into the middle of the geometry. The cursor
is therefore not a decoration a renderer may decline to draw — it is part of
what `layout` says the document *is*. An SVG export taken through today's
`layout` comes out with a block character stamped on the last letter of one
box, and there is no seam at which to remove it.

`Cursor` compounds this by living in `state.py:20`, where it is the only
declaration that describes something drawn rather than something edited.
`render.py` imports it from there to paint a `█`.

The separation this story wants is: `boxes` is the document, `selected` is a
view into the document, and `mode` and `running` are the interaction
lifecycle. `selected` is the interesting middle case — it is editor state, but
the terminal renderer legitimately needs it to draw a cursor. So the cursor
does not move up out of layout, and it does not move down into the renderer.
It becomes a pass applied *over* a document layout, which the editor runs and
an exporter does not.

This is a refactoring. There is no user story and no observable change.

## Acceptance Criteria

- `layout` is a pure function of the document and the terminal size. Its
  signature becomes `layout(boxes: Tuple[Box, ...], cols: int, rows: int) ->
  List[Placement]`, and it never yields a `Cursor`.
- Placing the cursor is a separate, separately testable function over the
  result of `layout`.
- `State` keeps its four fields and its flat shape. No `Document` wrapper.
- No `Cursor` is declared in `sketch/state.py`.
- Every existing assertion survives unedited. Call sites that name the changed
  signature may be rewritten mechanically — `layout(state, ...)` becomes
  `layout(state.boxes, ...)`, and the cursor tests compose the new function —
  but no expected value, no coordinate and no test name may change. A failing
  assertion means the refactoring is wrong, not the test.
- Rendered output is byte-identical for every state the editor can reach.

## Technical Design

### `Cursor` moves to `layout.py`

`Cursor` is deleted from `state.py` and declared in `layout.py` beside
`Arrow` and `Label`, which are the other two node types layout synthesises and
state knows nothing about. `render.py` imports it from `.layout`, as it
already imports `Arrow`, `Label` and `Placement`. `state.py` stops being
imported by `render.py` for anything but `PLAIN` and `Box`.

`tests/test_render.py:26` and `tests/test_graphics.py:16` import `Cursor` from
`sketch.state`; both move to `sketch.layout`. That is an import line, not an
assertion.

### `Label` carries its path

```python
@dataclass(frozen=True)
class Label:
    text: str
    path: Path = ()
```

The cursor pass has to find the box it belongs to, and `Placement.node` for a
box is a `Box`, which has no path — only the internal `Positioned` and
`Celled` know where a node came from. Giving the emitted `Label` the path it
was built from is the cheapest way to carry that across the seam, and `Label`
is already layout-local.

The default keeps it invisible to the renderers: the eight `Label("hi")`
constructions in `tests/test_render.py` compare equal as they stand, because
the placements under test are hand-built and never selected.

### `emit` stops synthesising the cursor

The `if here.path == selected:` block at `layout.py:160` is deleted, and with
it `emit`'s `selected` parameter and the `partial` that bound it
(`layout.py:206`). `emit` becomes a function of a node and its children alone.
The `Label` yield gains the path:

```python
yield Placement(Label(here.box.label, here.path), x=start, y=middle,
                width=interior(here.box.label), height=1)
```

### `with_cursor`

```python
def with_cursor(placements: List[Placement], selected: Path) -> List[Placement]:
    for placement in placements:
        if isinstance(placement.node, Label) and placement.node.path == selected:
            return placements + [
                Placement(
                    Cursor(),
                    x=placement.x + placement.width - 1,
                    y=placement.y,
                    width=1,
                    height=1,
                )
            ]
    return placements
```

The arithmetic is the same arithmetic, read off the label instead of
recomputed from the box. `emit` set the label's `x` to `start` and its `width`
to `interior(here.box.label)`, and placed the cursor at `start +
interior(here.box.label) - 1` on the same row. So `x + width - 1` and `y` are
identical by construction, for every label including the empty one, where
`interior` floors at 1 and both forms give `start`.

Selecting nothing is `selected == ()`. No emitted path is empty — `forest`
seeds every path with at least the top-level index — so the scan falls through
and no cursor is appended. That is `test_nothing_selected_emits_no_cursor` and
`test_an_empty_canvas_emits_no_cursor`, unchanged.

Appending rather than inserting is safe against the ordering rule at the foot
of `layout`, which puts every `Box` before everything else so boxes cannot
paint over the arrows and cursor on top of them. The cursor was already in the
non-box group; it is now last within it. The only ordering that ever mattered
to it is cursor-after-box, and that still holds. It is also still after its own
label, and it now follows other nodes' labels and arrows too — which changes
nothing, because a cursor covers one cell inside its own box's interior and
those never overlap.

### `layout`

```python
def layout(boxes: Tuple[Box, ...], cols: int, rows: int) -> List[Placement]:
    trees = forest(boxes)
    ...
    placements = [
        placement
        for tree in trees
        for placement in flatten(emit, fmap(place, tree))
    ]
```

`State` is no longer imported by `layout.py`. The body is otherwise untouched:
`forest`, `walk`, `column_tracks`, `centre` and the ordering rule all stand.

### `writer.frame`

```python
def frame(state: State, renderer: Renderer, stream: TextIO) -> None:
    cols, rows = os.get_terminal_size()
    placements = with_cursor(layout(state.boxes, cols, rows), state.selected)
    paint(stream, renderer.render(placements, cols, rows))
```

This is the only place the two halves are joined, and it is the only place
that should be: `frame` is the editor drawing itself. An SVG exporter calls
`layout` and stops.

`state.mode` is still nowhere in this path, as it already was. The cursor is
drawn in command mode too, sitting on the last character rather than one past
it, and that behaviour is unchanged — the difference between the two modes is
carried entirely by the trailing `PAD` in the label, not by a branch.

### Tests

`tests/test_layout.py` has 35 `layout(...)` call sites, all of the form
`layout(State(...), ...)` or `layout(state, ...)`. Each becomes
`layout(State(...).boxes, ...)` or `layout(state.boxes, ...)`. Several already
build a `state` local only to hand it to `layout`; those may pass the tuple
directly. Coordinates and expected placements do not move.

`CursorTest`'s four tests compose the new pass, keeping their names,
their `interior = box.x + BORDERS // 2` derivation and their expected values:

```python
def test_command_mode_cursor_sits_on_the_last_character(self):
    placements = with_cursor(layout((Box("hi"),), 11, 11), (0,))
    box = boxes(placements)[0]
    cursor = cursors(placements)[0]
    ...
```

`State`, `PAD` and `mode="insert"` stay in the insert-mode cases, because what
those tests pin down is that a trailing `PAD` pushes the cursor one column
right — a fact about labels that is still true and still worth a test.

New tests for `with_cursor` on its own, on hand-built placements, with no
`layout` involved:

- An empty placement list comes back empty.
- A placement list with no matching label comes back unchanged, and is the
  same list content, not a copy with something dropped.
- A `Cursor` is appended at the right-hand end of the matching label's row.
- A one-cell label puts the cursor on its single cell.
- Only the label whose `path` equals `selected` is matched; a sibling at a
  different path does not attract the cursor.
- Exactly one cursor is appended, and it is the last placement.

And one for `layout`: a selected-looking document laid out on its own emits no
`Cursor` at all, whatever `selected` would have been. That is the criterion
the SVG renderer depends on, and it is the test that would have caught this
story's bug before it existed.

### Out of scope

Two things surfaced while reading and are deliberately left alone.

`running` in `State` is arguably the loop's business rather than the sketch's,
and `handle_key` returning `Optional[State]` would say so. It is one honest
flag and it costs the renderers nothing; folding it in here would drag
`writer.run` into a layout refactoring.

`PAD` is editor state stored inside the document: a box is born as `Box(PAD)`
and insert mode appends and strips a trailing space (`state.py:114` and
`state.py:166`). Labels are clean in command mode, so this only shows in an
export taken mid-edit.
Worth its own story once there is something to export.

### Collaborators

`sketch/state.py` loses `Cursor` and gains nothing. `sketch/render.py` changes
one import line. `sketch/writer.py` changes one line in `frame`. The
`Placement` contract is unchanged, so `TerminalRenderer`, `GraphicsRenderer`
and `KittyGraphics` are untouched.
