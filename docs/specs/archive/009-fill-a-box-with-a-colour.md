# 009 - Fill a box with a colour

## Story

Bob has a box with a red border. In command mode he presses `f` and its inside
turns black, then red, then green as he keeps pressing — the label sits on top of
the colour, and the border stays red the whole time. He presses `j` down to the
next box and presses `f` once — that box's inside turns black, and the first box
keeps its green fill.

## Acceptance Criteria

- A new box has no fill; its inside is the terminal's plain background.
- In command mode, `f` advances the selected box's fill one step through the
  cycle: none, black, red, green, yellow, blue, magenta, cyan, white, and the
  next press returns it to none.
- The fill covers the box's interior area only — the border row and column are
  not painted.
- The fill runs behind the label, so the interior is one solid block of colour
  with the label text sitting on top of it and staying readable.
- `f` never changes the border colour, and `c` never changes the fill — a box can
  have a red border and a blue fill at the same time.
- `f` changes only the selected box; the other boxes are untouched.
- Each box keeps its own fill — moving with `j` and `k`, and adding a box with
  `b`, leave every box's fill as it was.
- In insert mode, `f` types the letter "f" into the label.
- On a canvas with no boxes, `f` does nothing.

## Technical Design
A fill is a second stored fact on a box, sitting beside its border colour. Like
colour, it moves nothing: size and position stay derived from state every frame,
so `layout.py` is untouched by this story. This is a paint-stage change only.

### Fill is a second ANSI index on `Box`

The fill cycle in the acceptance criteria is the same eight ANSI colours as the
border cycle, with "no fill" in front — the same shape as 007's `PLAIN`. So it is
stored the same way, as a second independent field:

```python
@dataclass(frozen=True)
class Box:
    label: str = ""
    colour: int = PLAIN
    fill: int = PLAIN
```

`next_colour` and `CYCLE` are reused as they are. Two plain ints rather than a
`Style` value object: they are independent, they never combine, and `replace()`
already preserves whichever one is not being changed — which is what keeps `c`
from touching the fill and `f` from touching the border.

### Pressing `f`

`handle_command` gains an `f` branch, guarded like `c` already is:

- On an empty canvas, `f` returns the state unchanged.
- Otherwise the selected box is replaced with one whose `fill` is stepped from
  `PLAIN` to `0` (black), and from `7` (white) back to `PLAIN`.
- Only the selected box is rebuilt, so other boxes keep their fills.
- In insert mode nothing changes: `f` is inside the printable range that
  `handle_insert` already appends to the label.

The `f` branch is the `c` branch with one field name changed. It is written out
in full rather than folded into a shared `recolour(state, field)` helper — two
short parallel branches read plainly, and a field-name-driven helper buys little
at two call sites. If a third paint arrives, extract then.

### A third element on `Cell`

`Cell` widens from `(character, colour)` to `(character, colour, fill)`. Every
write still funnels through `_put`, which stays the single clipping chokepoint,
and `fill` is a **required** part of the tuple exactly as `colour` is: a
character can never land without both of its colours, so nothing inherits a stale
background from an earlier placement.

There is no separate background grid and no merging in `_put` — a write always
replaces the whole cell. This works because `_draw_box` already paints the
interior explicitly, writing a blank cell to every non-border position rather
than leaving it untouched. Those blanks simply carry the fill now:

- Border characters: `(character, colour, PLAIN)`. The border row and column are
  never painted, which is the acceptance criterion, and it falls out of the
  existing `_box_character` branch rather than needing new geometry.
- Interior blanks: `(BLANK, PLAIN, fill)`. This is what makes the interior one
  solid block of colour.
- `_draw_label`, which runs after the interior blanks, writes
  `(character, PLAIN, fill)` — plain foreground on the box's fill. Passing the
  fill here is what keeps the label from punching holes in the block.
- `_draw_cursor` writes `(CURSOR, PLAIN, PLAIN)`.

The cursor deliberately does *not* pick up the fill. It is a separate `Placement`
appended by `layout` and drawn after the boxes, so it has no handle on the box it
sits over; giving it one would mean threading a `fill` field through `Cursor` for
a single cell. The block glyph covers its cell entirely anyway, so the hole is
not visible as a hole.

`BLANK_CELL` becomes `(BLANK, PLAIN, PLAIN)` and still serves as the grid
initialiser, so the empty canvas emits nothing.

### Serialising, one escape per cell

Cells are still emitted independently, with no state carried across a row. A cell
may need a foreground, a background, or both, so the codes that apply are
gathered and wrapped in a single SGR sequence with one `RESET`:

```python
def _cell(character: str, colour: int, fill: int) -> str:
    codes = []
    if colour != PLAIN:
        codes.append(30 + colour)
    if fill != PLAIN:
        codes.append(40 + fill)
    if not codes:
        return character
    return f"\x1b[{';'.join(str(code) for code in codes)}m{character}{RESET}"
```

One combined sequence rather than two: emitting foreground and background
separately would still require fixing an order to keep assertions stable, so it
buys nothing over a single canonical form and costs a second `RESET`.

`state.py` still never learns ANSI syntax — the `40 +` offset lives here beside
the `30 +` one. `render` returns `List[str]` and `paint` is unchanged. A box with
neither colour nor fill emits no escapes at all, so the existing renderer tests
keep passing untouched. Filled cells are asserted one at a time rather than by
slicing a row, since escapes occupy string length without occupying columns.

### Note

Black is the first stop in the cycle, so the first press of `f` paints a black
interior. Under a plain default foreground that leaves the label unreadable on a
light terminal, and the fill nearly invisible on a dark one — the same tension
007 recorded for black borders. The cycle is built exactly as the spec writes it
and the label's foreground is left plain; a later story may revisit the cycle's
starting point or make the label's colour follow the fill.
