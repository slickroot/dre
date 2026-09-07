# 004 - Stack a second box below the first

## Story

Bob has a labelled box. He presses `Esc`, then `b` again, and a second empty box appears below the first with a blank line between them. He types, and the new box takes his label while the first one keeps its own.

## Acceptance Criteria

- Pressing `b` when a box already exists places the new box below the last one.
- There is one blank line between each box and the next.
- Each box is horizontally centered on its own, so the boxes' middles line up.
- Typing after the new box appears goes into that new box; earlier boxes keep their labels unchanged.

## Technical Design
All the stacking work lands in `layout()`. `handle_key` already appends a box on
`b` and edits the last one while typing, and `TerminalRenderer.render` already
draws every placement it is given, so neither needs to learn about stacks.

### Components

**`Cursor`** (new, `state.py`) — a frozen dataclass with no fields, alongside
`Box` in the `Node` union. It knows nothing; it exists so the renderer can draw
it by type instead of `Placement` carrying a `cursor: Optional[int]` field.
That field is removed.

**`Placement`** (`layout.py`) — unchanged apart from dropping `cursor`. A cursor
now arrives as its own `Placement(Cursor(), x, y, width=1, height=1)`.

**`layout()`** (`layout.py`) — grows two responsibilities: stacking boxes
vertically, and emitting the cursor placement. Collaborators: `State` (reads
`nodes` and `mode`), `Placement` (constructs).

**`TerminalRenderer`** (`render.py`) — dispatches on node type: `Box` draws the
frame plus label as today, `Cursor` writes the block glyph at the placement's
own `x`/`y`. `_draw_label` no longer knows about the cursor.

### Vertical stacking

Boxes are `BOX_HEIGHT` tall with `GAP = 1` blank line between them, and the
whole stack is centered vertically:

    total = len(nodes) * BOX_HEIGHT + (len(nodes) - 1) * GAP
    top   = (rows - total) // 2
    y     = top + index * (BOX_HEIGHT + GAP)

Centering the whole stack means a second box shifts the first one up. That is
deliberate: a later story centers everything, and this keeps it consistent now.

When the stack is taller than the terminal, `top` goes negative and boxes run
off both ends. We accept that — `_put` already clips silently. Scrolling and
overflow are a separate story.

### Width and the cursor

`BORDERS_AND_CURSOR = 3` is replaced by `BORDERS = 2`. The cursor takes up a
cell just like a character does, so it widens the box it sits in:

    width = len(label) + BORDERS + (1 if the box has the cursor else 0)

There is no minimum width. An empty box in command mode is 2 columns and draws
as `┌┐ / ││ / └┘`, which is fine — a freshly created box is always 3 wide
because `b` enters insert mode straight away. `test_box_never_shrinks_below_3x3`
is retired.

Each box is centered horizontally on its own width, `x = (cols - width) // 2`,
so a focused box sits half a column off its neighbours. That satisfies "each box
is horizontally centered on its own".

### Focus

In insert mode `layout()` appends one extra placement for the cursor, positioned
inside the focused box at `x = box.x + 1 + len(label)`, `y = box.y + 1`. In
command mode no cursor placement is emitted at all.

`layout()` treats the last node as the focused one, matching what
`handle_insert` already assumes with `relabel_last`. "Last means focused" is an
assumption we are not happy with, but it stays local to `layout()` and
`state.py` is untouched by this story; a later story can introduce explicit
focus and only `layout()` will need to read it.

### Worked example

Two boxes labelled `a` and `b` in insert mode, `cols=11`, `rows=11`:
`total = 7`, `top = 2`. The first box is unfocused, width 3, `x=4`, `y=2`. The
second is focused, width 4, `x=3`, `y=6`. The cursor lands at `x=5`, `y=7`.

### Tests

`tests/test_layout.py` carries the new behaviour:

- a second box is placed `BOX_HEIGHT + GAP` rows below the first
- there is exactly one blank row between two stacked boxes
- the stack as a whole stays vertically centered as boxes are added
- each box is centered on its own width, so a focused and an unfocused box
  differ in `x`
- insert mode emits a cursor placement inside the focused box
- command mode emits no cursor placement
- only the focused box gets a cursor: with two boxes, the single cursor
  placement is geometrically inside the last one
- a box with the cursor is one column wider than the same box without it

`tests/test_render.py` gains: a `Cursor` placement draws the block glyph at its
own coordinates, and a `Box` placement no longer draws a cursor.
