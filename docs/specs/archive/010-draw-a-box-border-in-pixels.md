# 010 - Draw a box's border in pixels

## Story

Bob adds a box and types "hi". The box sits where it always did, but its border
is now a thin crisp line riding the very edge of the box instead of a row of
characters. He presses `c` and that line turns red. He presses `f` and the inside
turns black — right up to the red line, with no gap anywhere.

## Acceptance Criteria

- A box's border is drawn in pixels, one pixel thick, sitting on the outer edge
  of the box's area rather than inside a character cell.
- A box keeps the size and screen position it has today: as wide as its label
  plus one cell of padding on each side, and three rows tall.
- Every cell of the box belongs to the box — no cell is given over to the border.
  A 3×3 box has all nine cells to itself.
- The label sits in the middle row, one cell in from the left edge.
- `c` cycles the border colour as it does today, and the pixel border is drawn in
  that colour.
- `f` fills every cell of the box, so the fill runs right up to the border with
  no gap and the border does not cut through it.
- The label, the fill cycle, the arrows and the cursor are otherwise unchanged.
- The terminal is assumed to support pixel graphics; there is no fallback in this
  story.

## Technical Design

The character grid and the pixel border are produced by two collaborators, joined
by a wrapper so that `writer.frame` still takes exactly one renderer.

### Components

**`TerminalRenderer` — unchanged contract, simpler box**

Still `render(placements, cols, rows) -> List[str]`. The only change is
`_draw_box`: every cell of the box becomes `(BLANK, PLAIN, fill)`, so the fill
covers all nine cells of a 3×3 box and no cell is given over to a border.
`_draw_label`, `_draw_cursor`, `_draw_arrow`, `_put` and `_cell` are untouched.

`_box_character` and the glyph constants `TOP_LEFT`…`VERTICAL` are **kept**, no
longer called, against a future story for terminals without pixel graphics. To
stop them rotting, `_box_character` gains a direct unit test of its own.

**`Sprite` — the seam**

```python
@dataclass(frozen=True)
class Sprite:
    pixels: bytes   # RGBA, row-major, width * height * 4
    width: int      # pixels
    height: int     # pixels
    col: int        # cell column of its top-left corner
    row: int        # cell row of its top-left corner
```

Geometry and rasterising live on one side of this type, wire format on the other.

**`GraphicsRenderer` — the wrapper**

Satisfies the existing `Renderer` protocol structurally, so `frame` and `paint`
are unchanged.

```python
class GraphicsRenderer:
    def __init__(self, text: Renderer, graphics: GraphicsProtocol,
                 cell_width: int, cell_height: int) -> None: ...

    def render(self, placements, cols, rows) -> List[str]:
        lines = self.text.render(placements, cols, rows)
        payload = self.graphics.draw(self._sprites(placements, cols, rows))
        return lines[:-1] + [lines[-1] + payload]
```

It wraps rather than subclasses `TerminalRenderer`: it needs a character
renderer's output rather than specialising one, and wrapping keeps the text
renderer and the graphics protocol independently swappable and independently
stubbable in tests.

The payload is appended to the **last line**, not added as an extra element.
`paint` joins lines with `"\r\n"`, so an extra element would emit a newline on
the bottom row and scroll the alternate screen. The Kitty payload carries its own
absolute cursor positioning, so its only requirement is to come after the text —
which this construction guarantees. Writing a character repaints its whole cell,
so pixels drawn before text would be erased by it.

`_sprites` handles geometry: for each `Box` placement it rasterises a
one-pixel outline into an RGBA buffer of
`placement.width * cell_width` by `placement.height * cell_height`, opaque on the
edge and alpha 0 inside. The transparent interior is what makes the fill run right
up to the border with no gap. `Arrow` and `Cursor` placements are ignored.

**`GraphicsProtocol` and `KittyGraphics`**

A `typing.Protocol` — `draw(sprites: List[Sprite]) -> str` — alongside the
existing `Renderer` protocol, so sixel or a future format can be substituted at
construction. `KittyGraphics` is the only implementation for now:

- frame start: delete all placements, `\x1b_Ga=d,d=A,q=2;\x1b\\`
- per sprite: `\x1b[{row+1};{col+1}H` then
  `\x1b_Ga=T,f=32,s={width},v={height},q=2;{base64}\x1b\\`
- payloads chunked to ≤4096 base64 bytes with `m=1` on all but the last chunk,
  `m=0` on the last; `o=z` zlib compression, since a mostly transparent buffer
  compresses to almost nothing.

**`writer`**

`frame` and `paint` keep their current shape. `run` composes:

```python
renderer = GraphicsRenderer(TerminalRenderer(), KittyGraphics(), *cell_size())
```

`cell_size()` is a new helper reading `ws_xpixel`/`ws_ypixel` from a
`fcntl.ioctl(TIOCGWINSZ)` call and dividing by columns and rows;
`os.get_terminal_size()` does not expose those fields.

**`layout`**

Unchanged. Box width stays `interior + BORDERS` and height stays `BOX_HEIGHT`, so
boxes keep today's size and position, and the label stays in the middle row one
cell in from the left.

### Colour

`Box.colour` is an ANSI index, which the text path renders as `30 + colour` and
lets the terminal theme resolve. A pixel buffer needs concrete bytes, so
`GraphicsRenderer` holds a fixed table of eight RGB triples, with `PLAIN` mapped
to mid grey `(128, 128, 128)` — legible on both light and dark backgrounds, and
distinct from white so an uncoloured border doesn't read as a deliberate choice.

### Clipping

`layout` centres boxes, so `x` goes negative once a label is wider than the
terminal, and `\x1b[6;0H` is not addressable. `GraphicsRenderer` therefore takes
`cols`/`rows` and crops: sprites are trimmed to the visible region with `col`/`row`
clamped to zero, mirroring how `_put` silently drops out-of-bounds cells.

### Known limitations

- Cell pixel size is read once at startup, so a terminal resize leaves the border
  misaligned until restart. `frame` re-reads `cols`/`rows` every frame; this is the
  one dimension that goes stale.
- Sprites are retransmitted in full every frame. Caching by
  `(width, height, colour)` and re-placing by image ID would cut the per-keystroke
  payload to a few dozen bytes; deferred, and confined to `KittyGraphics` if taken up.
- The palette is hardcoded, so a red border may not match the terminal theme's red
  label. Querying `OSC 4` and `OSC 10` at startup would fix it at the cost of a
  read-with-timeout handshake in `writer`.
- No fallback for terminals without pixel graphics, per the story.

### Tests

- `TerminalRenderer`: existing box tests change from expecting `"┌─┐"` to expecting
  spaces; fill covers all nine cells; label and cursor unchanged. New direct test
  for `_box_character`.
- `GraphicsRenderer`: assert on buffer contents given a placement — alpha 255 on
  the edge, 0 inside, correct pixel dimensions, correct colour per index, `PLAIN`
  grey, cropping at each edge, no sprite for arrows or cursors. Text renderer
  stubbed, so these tests never touch the character grid.
- `KittyGraphics`: assert on the escape string for a hand-built small `Sprite` —
  positioning, header fields, chunking boundaries.
- `writer`: `run` composes the wrapper; `frame` and `paint` unchanged.
