# 007 - Colour a box's border

## Story

Bob has two boxes. In command mode he presses `c` on the selected one and its
border turns black, then red, then green as he keeps pressing. He presses `j`
down to the other box and presses `c` once — that box turns black, and the
first box is still green.

## Acceptance Criteria

- A new box is drawn in the terminal's plain default colour.
- In command mode, `c` advances the selected box's border one step through the
  cycle: plain, black, red, green, yellow, blue, magenta, cyan, white, and the
  next press returns it to plain.
- `c` changes only the selected box's border; the other boxes are untouched.
- Each box keeps its own colour — moving the selection with `j` and `k`, and
  adding a box with `b`, leave every box's colour as it was.
- The label inside a coloured box stays the plain default colour.
- In insert mode, `c` types the letter "c" into the label.
- On a canvas with no boxes, `c` does nothing.

## Technical Design
A box's colour is a stored fact, like its label — not something derived and not
something cached. Size and position stay derived from state every frame, because
colour cannot move anything. In browser terms this is a paint-stage property:
`layout.py` is untouched by this story.

### Colour is an ANSI index on `Box`

The cycle in the acceptance criteria — black, red, green, yellow, blue, magenta,
cyan, white — is exactly the ANSI colour table, indices 0 to 7, with plain in
front. So a colour is stored as that index, and plain is a sentinel:

```python
PLAIN = -1
CYCLE = 9  # eight ANSI colours plus plain

@dataclass(frozen=True)
class Box:
    label: str = ""
    colour: int = PLAIN
```

`state.py` holds the colour's identity but never learns ANSI syntax; the escape
sequence is built in `render.py` from `30 + colour`, so there is no name-to-code
table to keep in sync with the cycle.

### Pressing `c`

`handle_command` gains a `c` branch, guarded like `i` and `j`/`k` already are:

```python
def next_colour(colour: int) -> int:
    return (colour + 2) % CYCLE - 1
```

- On an empty canvas, `c` returns the state unchanged.
- Otherwise the selected box is replaced with one whose colour is stepped.
  From `PLAIN` to `0` (black), from `7` (white) back to `PLAIN`.
- Only the selected box is rebuilt, so other boxes keep their colours.
- In insert mode nothing changes: `c` is inside the printable range that
  `handle_insert` already appends to the label.

`edit()` currently rebuilds a box with `Box(label)`, which would silently reset
colour on every keystroke. It becomes `replace(nodes[index], label=label)` so it
preserves every field it is not changing.

### Two parallel grids in the renderer

`TerminalRenderer` keeps its one-printable-character-per-cell invariant and
carries colour in a second grid of the same shape, allocated alongside the first:

```python
chars = [[BLANK] * cols for _ in range(rows)]
colours = [[PLAIN] * cols for _ in range(rows)]
```

Every write already funnels through `_put`, which stays the single clipping
chokepoint and now writes both grids. `colour` is a **required** parameter: a
character can never land without its colour, which rules out a plain glyph
inheriting a stale colour from an earlier placement.

```python
def _put(self, chars, colours, x, y, character, colour):
    if 0 <= y < len(chars) and 0 <= x < len(chars[y]):
        chars[y][x] = character
        colours[y][x] = colour
```

Colour comes off the node — `Placement` gains no field, since `render` already
reaches through `placement.node` for the label, and `_draw_box` runs inside an
`isinstance(placement.node, Box)` branch.

- `_draw_box` passes the box's colour for border characters, and `PLAIN` for the
  blank interior, so blanks are never wrapped in escapes.
- `_draw_label` passes `PLAIN`, so the label overwrites the border's colour on
  the cells it occupies. This is what keeps a label plain inside a coloured box.
- `_draw_cursor` passes `PLAIN`.

### Serialising, stateless per cell

The two grids collapse back into `List[str]`. Each cell is emitted independently
— no state carried across a row:

```python
RESET = "\x1b[0m"

def _cell(character: str, colour: int) -> str:
    if colour == PLAIN:
        return character
    return f"\x1b[{30 + colour}m{character}{RESET}"
```

`render` still returns `List[str]` and `paint` is unchanged. A plain box emits no
escapes at all, so existing renderer tests keep passing untouched. Coloured cells
are asserted one at a time rather than by slicing a row, since escapes occupy
string length without occupying screen columns.

### Note

Colour 0 is black, and it is the first stop in the cycle. On a dark terminal the
first press of `c` will make the border nearly invisible. That is faithful to the
spec and to the ANSI table, so it is built as written — a later story may want to
revisit the cycle's starting point.
