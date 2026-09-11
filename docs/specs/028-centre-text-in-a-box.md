# Centre text in a box

As someone sketching a diagram, I want the text in every box centred, so my
diagram stays tidy when a neighbouring box grows wider.

## Acceptance Criteria

1. A box's text sits centred between its left and right borders.
2. When one box grows wide enough to widen its neighbours, the neighbours' text
   re-centres instead of staying against the left border.
3. When the leftover space can't be split evenly, the extra space goes on the
   left of the text.
4. In insert mode, the typing cursor sits immediately after the last character
   of the centred text, and moves as the text re-centres.
5. In a box with no text, the cursor sits in the middle of the box.

## Technical Design

A box is sized by its column track (`column_tracks`), so `placement.width` is
already the widest box in that column — that is why a neighbour growing makes
this box grow. Today two separate modules independently assume the text starts
at the left border: `TerminalRenderer._draw_label` paints at `placement.x + 1`,
and `layout.emit` places the `Cursor` at `here.x + interior(label)`. Centring is
one fact, so it is computed once, in the layout, and both the text and the
cursor are handed their final position.

### `centre` — a third measure beside `interior` and `width`

```python
def centre(width: int, label: str) -> int:
    leftover = width - BORDERS - interior(label)
    return 1 + leftover - leftover // 2
```

It answers "how far from the box's left edge does the text begin", given the
width the box was actually laid out at. The block being centred is
`interior(label)` cells wide — the same measure the box is sized by — not
`len(label)`. Two consequences fall out of that choice:

- An empty label still occupies one cell, so the cursor in an empty box lands on
  the exact middle (AC5).
- In insert mode the label carries a trailing `PAD`, and that space is part of
  the centred block, so the cursor cell stays inside it (AC4).

`leftover - leftover // 2` rounds the left share up, putting the extra cell on
the left of the text (AC3).

The function is conservative: a box at its intrinsic width has `leftover == 0`
and `centre` returns `1`, so every placement is exactly what it is today. Only a
box widened by its column moves, which is precisely AC2.

### `Label` — the text becomes its own placement

A new layout-only node, beside `Arrow`:

```python
@dataclass(frozen=True)
class Label:
    text: str
```

`emit` gains it, and uses `centre` once for both placements it positions:

```python
start = here.x + centre(here.width, here.box.label)
middle = here.y + here.height // 2

yield Placement(Box, ...)                                    # unchanged
yield Placement(Label(here.box.label), x=start, y=middle,
                width=interior(here.box.label), height=1)
if here.path == selected:
    yield Placement(Cursor(), x=start + interior(here.box.label) - 1,
                    y=middle, width=1, height=1)
```

The renderer no longer reaches into `node.label` at all: `_draw_box` stops
calling `_draw_label`, and `TerminalRenderer.render` dispatches `Label`
placements to a `_draw_label` that paints the text it is given at the `x` it is
given. `Placement` stays uniform — no field that is meaningless for an `Arrow`.

Ordering already works: `layout` returns all `Box` placements first and the rest
after, stably, so a box's `Label` and `Cursor` are painted over its body, and the
`Cursor` after the `Label` because `emit` yields it second.
`GraphicsRenderer._sprites` filters on `(Box, Arrow)`, so `Label` placements are
ignored there and the text stays a terminal glyph.

### `_stamp` — text is ink, not a filled cell

A grid cell is `(character, colour, fill)`, and `_put` overwrites all three. The
text must sit *on top of* the box without erasing the box's fill, so both the
label and the cursor switch to a stamp that changes only the glyph:

```python
def _stamp(self, grid: Grid, x: int, y: int, character: str) -> None:
    if 0 <= y < len(grid) and 0 <= x < len(grid[y]):
        _, colour, fill = grid[y][x]
        grid[y][x] = (character, colour, fill)
```

`_draw_cursor` uses it too, so the cursor stops punching a hole in a filled box
— which matters far more now that it sits in the middle of the box rather than
against the border.

### Tests

- `centre` directly: no leftover returns 1; an odd leftover puts the extra cell
  on the left; an empty label is measured as one cell.
- A box widened by a sibling has its `Label` placement further right than a box
  at its intrinsic width, and its text stays centred (AC1, AC2).
- The `Cursor` placement sits on the last cell of the `Label` placement (AC4),
  and on the middle cell of an empty box (AC5).
- `TerminalRenderer` keeps a box's fill under both the text and the cursor.
