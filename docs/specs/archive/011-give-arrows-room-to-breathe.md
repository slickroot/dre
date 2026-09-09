# 011 - Give arrows room to breathe

## Story

Bob has two boxes stacked on the canvas. The gap between them is now 2 rows
instead of 1, so they no longer feel cramped. When he connects them with an
arrow, the arrow fills both rows — a shaft with the head at the end — so it
reads as a proper arrow rather than a single glyph.

## Acceptance Criteria

- The gap between two stacked boxes is 2 rows tall.
- An arrow is drawn in pixels, one pixel thick, like a box's border — no
  character is written into the gap.
- An arrow pointing down draws as a shaft running the first row and a head
  occupying the second, its tip at the bottom.
- An arrow pointing up draws as a head occupying the first row, its tip at the
  top, and a shaft running the second.
- The shaft is centred in the arrow's cell and runs the full height of the gap;
  the head is two diagonal strokes meeting at the tip, symmetric about the shaft.
- An arrow is drawn in the same plain grey an uncoloured border uses.
- A gap with no arrow in it stays blank across both rows.
- Everything else is unchanged: box size, the stack staying centred, the label,
  the fill, the border colour and the cursor.

## Technical Design

The gap grows in `layout`, and the arrow moves out of the character grid
entirely: it becomes a sprite, drawn in pixels by `GraphicsRenderer`, exactly as
the box border did in story 010.

### `layout` — the gap is two rows

`height()` keeps its shape and grows a constant:

```python
GAP_HEIGHT = 2

def height(node: Node) -> int:
    if isinstance(node, Box):
        return BOX_HEIGHT
    return GAP_HEIGHT
```

The gap height belongs to `height()` rather than to the renderer: `layout`
already sums `height(node)` to centre the stack and to advance `y`, so a single
change keeps `Space` and `Arrow` symmetric, keeps the stack centred, and hands
the renderer a `Placement` that is genuinely two rows tall. Nothing else in
`layout` moves — `width()` is untouched (`Arrow` 1, `Space` 0), and the `Cursor`
placement is still built directly at `box.y + 1` with height 1.

### `TerminalRenderer` — the gap is blank

`render` drops its `Arrow` branch, so the two gap rows stay `BLANK_CELL`.

`_draw_arrow` and the `ARROW_DOWN` / `ARROW_UP` constants are **kept, uncalled**,
against a future story for terminals without pixel graphics — the same treatment
`_box_character` and the border glyphs got in 010. As there, `_draw_arrow` gains
a direct unit test of its own so it cannot rot.

### `GraphicsRenderer` — dispatch and shared clipping

`_sprites` keeps ownership of the crop arithmetic and gains a type dispatch:

```python
for placement in placements:
    ...  # left/top/right/bottom as today
    if isinstance(placement.node, Box):
        sprites.append(self._outline_box(placement, left, top, right, bottom))
    elif isinstance(placement.node, Arrow):
        sprites.append(self._outline_arrow(placement, left, top, right, bottom))
```

`Space` and `Cursor` are still skipped. Both rasterisers take the same
`(placement, left, top, right, bottom)` signature, so there is one clipping rule
in the file, not two: geometry stays in `_sprites`, rasterising in the outliners.

### `_outline_arrow` — shaft plus chevron

The sprite covers the whole placement: `placement.width * cell_width` by
`placement.height * cell_height`, i.e. one cell wide and two cells tall. In that
pixel space:

- **Centre column** `cx = width // 2`.
- **Shaft**: one pixel wide at `cx`, running the **full sprite height**, tip
  included. A shaft stopping at the head's base would leave a visible break
  where the chevron's 1px strokes have not yet converged.
- **Head**: the tip is the last row for `forward` (`y = height - 1`) and the
  first row for `backward` (`y = 0`). The head occupies the whole cell at that
  end — `cell_height` rows — and each diagonal is interpolated from the tip out
  to `cx ± reach` at the head's base, so the slope follows the cell's aspect
  ratio rather than being fixed at 45°. At a typical 2:1 cell a 45° head would
  be a stubby tick in the corner of the row the story gives it.
- **Symmetry**: `reach = min(cx, width - 1 - cx)`. With an even `cell_width`
  there is no true centre column, so the head is kept symmetric about the shaft
  and one edge column of the sprite is simply left unused. A lopsided head reads
  as a mistake; an unused column does not.
- Colour is `_colour(PLAIN) + (OPAQUE,)` — mid grey. `Arrow` grows no `colour`
  field; the story changes nothing about colour.

Everything off the shaft and off the diagonals is `TRANSPARENT`, so the arrow
composites over the blank cells beneath it the way the box outline composites
over its fill. Rasterising is a per-pixel predicate over the cropped window,
mirroring `on_edge` in `_outline_box`:

```python
def _on_arrow(self, x, y, width, height, direction) -> bool:
    if x == cx:
        return True
    spread = round(distance_from_tip(y) * reach / (cell_height - 1))
    return spread <= reach and abs(x - cx) == spread and in_head(y)
```

### Tests

- `layout`: `height(Space())` and `height(Arrow())` are both `GAP_HEIGHT`; two
  stacked boxes are `BOX_HEIGHT + GAP_HEIGHT` apart; the two rows between them
  are unoccupied; the stack stays centred with the taller total.
- `TerminalRenderer`: the gap rows render blank for both arrow directions;
  `_draw_arrow` keeps its own direct test on both glyphs; box, label, fill and
  cursor tests unchanged.
- `GraphicsRenderer`: with a stubbed text renderer — a sprite is emitted for an
  `Arrow`, sized one cell by two; the shaft column is opaque on every row; the
  head's diagonals are opaque and symmetric about the shaft; the tip is at the
  bottom for `forward` and the top for `backward`; off-shape pixels are alpha 0;
  the colour is `PLAIN` grey; cropping at each edge behaves as it does for boxes;
  `Space` and `Cursor` still produce nothing.

### Known limitations

- The head's slope is derived from the cell aspect ratio read once at startup, so
  a resize leaves it stale along with the box border — the same limitation 010
  already carries.
- Diagonal rounding can repeat an `x` on adjacent rows for tall cells, giving a
  slightly stepped edge. Anti-aliasing is deferred and would be confined to
  `_outline_arrow`.
- No fallback for terminals without pixel graphics, per 010.
