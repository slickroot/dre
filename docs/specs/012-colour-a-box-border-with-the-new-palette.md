# 012 - Colour a box's border with the new palette

## Story

Bob has a box selected. In command mode he presses `c` and its border turns
Amber Gold, then Blaze Orange, then Neon Pink as he keeps pressing. He presses
`j` down to another box and presses `c` once — that box turns Amber Gold, and
the first box is still Neon Pink.

## Acceptance Criteria

- In command mode, `c` advances the selected box's border one step through the
  cycle: grey (plain), Amber Gold (`#ffbe0b`), Blaze Orange (`#fb5607`), Neon
  Pink (`#ff006e`), Blue Violet (`#8338ec`), Azure Blue (`#3a86ff`), and the
  next press returns it to grey.
- `c` changes only the selected box's border; other boxes are untouched.
- Each box keeps its own colour — moving the selection with `j`/`k`, and
  adding a box with `b`, leave every box's colour as it was.
- The label inside a coloured box stays the plain default colour.

## Technical Design

The border's colour cycle changes from the 8 ANSI colours to 5 fixed hex
colours. `Box.colour` keeps its current shape — a plain int with `PLAIN = -1`
as the sentinel — only the cycle length and the palette it resolves to change.
`fill` is untouched: it keeps its own 8-ANSI-colour cycle, its own `CYCLE = 9`,
and its own stepping function, since border and fill now have different cycle
lengths and can no longer share one.

### `state.py` — a named cycle length, a separate stepping function

```python
PALETTE_SIZE = 5

def next_colour(colour: int) -> int:
    return (colour + 2) % (PALETTE_SIZE + 1) - 1
```

`PALETTE_SIZE` is the domain fact — 5 real colours, plain in front — kept in
`state.py` because it never needs to know what those colours actually are,
only how many there are. `next_colour` is the same formula as before with
`PALETTE_SIZE + 1` in place of the old `CYCLE`, so `PLAIN` steps to `0`, `0` to
`1`, … `4` back to `PLAIN`.

The existing stepping function is renamed `next_fill` and keeps `CYCLE = 9`
and its old formula unchanged, so the `f` branch in `handle_command` calls
`next_fill` where it used to call `next_colour`; the `c` branch keeps calling
`next_colour`, now with the new formula. Both still return `Box`es built with
`replace()`, so neither branch's shape in `handle_command` changes beyond the
function name.

### `render.py` — swap the table, not the logic

`_colour()` already does exactly the lookup this story needs:

```python
def _colour(colour: int) -> Tuple[int, int, int]:
    if colour == PLAIN:
        return PLAIN_COLOUR
    return PALETTE[colour]
```

Only `ANSI_COLOURS` changes — renamed `PALETTE` and its 8 ANSI RGB triples
replaced with the 5 new ones, in cycle order so index `0` is Amber Gold
through index `4` Azure Blue:

```python
PALETTE = (
    (255, 190, 11),   # Amber Gold  #ffbe0b
    (251, 86, 7),     # Blaze Orange #fb5607
    (255, 0, 110),    # Neon Pink   #ff006e
    (131, 56, 236),   # Blue Violet #8338ec
    (58, 134, 255),   # Azure Blue  #3a86ff
)
```

`PLAIN_COLOUR` (mid grey) is unchanged, and `_colour` is still the only
consumer of `PALETTE` — `fill`'s ANSI escape codes in `_cell` (`40 + fill`)
never went through this table and are untouched. No other function in
`render.py` changes: `_outline_box` already calls `_colour(placement.node.colour)`
and needs nothing new from this story.

### Tests

- `state.py`: `next_colour` cycles `PLAIN → 0 → 1 → 2 → 3 → 4 → PLAIN` (6
  steps, was 9); a new `next_fill` test keeps the old 9-step cycle. `handle_command`
  tests for `c` build expected boxes with `next_colour`, and for `f` with
  `next_fill`, instead of both sharing the one function.
- `render.py`: `_colour` is asserted against each of the 5 `PALETTE` entries
  by index and against `PLAIN_COLOUR` for `PLAIN`, in place of the old 8-entry
  ANSI assertions. `GraphicsRenderer`/`_outline_box` tests that build a
  coloured `Box` switch to the new palette values; no other renderer test
  changes.
