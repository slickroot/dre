# 006 - Move the selection between boxes

## Story

Bob has three boxes. He presses `Esc`, then `k` twice — the cursor walks up from
the bottom box to the first one. He presses `i` and types, and it is that first
box that changes.

## Acceptance Criteria

- In command mode the selected box shows a block cursor sitting on the last
  character of its label; the other boxes show no cursor.
- `j` moves the selection down one box, `k` moves it up one.
- `j` on the bottom box leaves the selection where it is; `k` on the top box leaves it where it is.
- Pressing `b` adds a new box at the bottom and selects it, so `Esc` leaves Bob on the box he just made.
- Pressing `i` enters insert mode on the selected box, rather than the last box.
- On a canvas with no boxes, `j` and `k` do nothing.

## Technical Design
This story cashes in the promissory note 004 left: "last means focused" was an
assumption we were not happy with, kept local to `layout()` and `state.py`.
Selection becomes explicit, and both `-1`s — `layout()`'s
`index == len(nodes) - 1` and `relabel_last`'s `nodes[:-1]` — are replaced by a
real index carried on `State`.

`render.py` is untouched by this story. That is the design's main result, and
the reason for the departure recorded below.

### Departure from the original acceptance criteria

The story as first written marked the selection with a double-line border. We
replaced that with a vim-style cursor: a block cursor sits **on** the last
character in command mode, and **after** the last character in insert mode.
Acceptance criterion #1 and the story text were rewritten to match.

The reason is that a double border is a fact the *renderer* has to know, so it
would need either a `selected: bool` on `Placement` or a second box node type,
plus a second charset in `TerminalRenderer`. Marking selection with the cursor
instead means `layout()` computes the cursor's coordinates — as it already does
for insert mode — and the renderer never learns that selection exists. It also
makes the two modes one cursor that changes behaviour rather than two unrelated
visual languages.

The cost we accept: `CURSOR` is `█`, a solid block written into a plain
character grid, so in command mode it **covers** the character it sits on. A box
labelled `hello` reads `hell█`, and a one-character label is hidden entirely.
Vim avoids this with inverse video, which we cannot do — `grid` is
`List[List[str]]`, `render()` returns `List[str]`, and an escape sequence is
several characters in one cell, breaking the one-glyph-per-column assumption
every render test relies on. Styled cells are a story of their own; until then
we accept the covered character.

### Components

**`State`** (`state.py`) — gains `selected: int = -1`, the index of the selected
box. Placed last so existing positional constructions keep working.

**`edit()`** (`state.py`, replaces `relabel_last`) — a pure list helper that
knows nothing about `State`:

    def edit(nodes: List[Node], index: int, label: str) -> List[Node]:
        return nodes[:index] + [Box(label)] + nodes[index + 1:]

Same contract as `relabel_last`, arbitrary position. Collaborators: `Box`
(constructs).

**`handle_command()`** (`state.py`) — learns `j`, `k` and `i`, and sets the
selection on `b`. Collaborators: `State`.

**`handle_insert()`** (`state.py`) — reads and writes `nodes[state.selected]`
instead of `nodes[-1]`. Collaborators: `edit`.

**`layout()`** (`layout.py`) — reads `state.selected` instead of deriving focus
from position, and emits a cursor placement in **both** modes rather than insert
mode only. Collaborators: `State`, `Placement`.

**`Placement`**, **`TerminalRenderer`**, **`Cursor`**, **`writer.py`** —
unchanged.

### Selection on State

`selected` is an `int`, not an identity. The story only needs up and down over a
fixed list; giving `Box` an id to survive reordering would touch every existing
test that writes `Box("a")` to buy something no story needs yet. When deletion
or reordering arrives, that story fixes up the index.

It does not live on `Box`. `State.nodes` is the document — the thing a later
story saves to disk — and a label is content while a selection is where the
caret is sitting this second. A `bool` per box also cannot express "exactly one
box is selected", where an `int` on `State` can, and it would turn `j` into a
list rebuild that clears one flag and sets another.

It does not live on `Placement` either, though that is where the *derived*
selection would have gone under the double-border design. With the cursor
carrying the signal, nothing derived is needed.

**Invariant:** `selected == -1` when `nodes` is empty; otherwise
`0 <= selected < len(nodes)`. `-1` is a deliberate sentinel and buys two things:
`b` needs no special case, because the new box is `len(nodes) - 1` whether the
canvas was empty or not; and `[][-1]` raises `IndexError` rather than silently
succeeding, so a missed guard fails loudly on an empty canvas.

The matching hazard, recorded because it is real: `-1` is a legal index into a
*non-empty* list, so a state that violates the invariant reads as "the last box"
instead of failing. Nothing constructs such a state — `nodes` only grows via
`b`, which always sets `selected` — but see the note on existing tests below.

### Moving the selection

`j` and `k` clamp at the ends rather than wrapping:

    step     = 1 if key == "j" else -1
    selected = min(max(state.selected + step, 0), len(state.nodes) - 1)

The empty canvas is guarded explicitly before this, not left to the clamps. The
two directions do not degrade symmetrically: with `nodes == []`, `j` gives
`min(0, -1) == -1`, which is correct by accident, but `k` gives
`max(-2, 0) == 0`, which would select a box that does not exist. One guard —
`if not state.nodes: return state` — covers both keys.

### `b` and `i`

`b` appends a box and selects it: `selected = len(nodes) - 1` on the new list,
entering insert mode as it already does. `Esc` then leaves Bob on that box,
because `Esc` changes only `mode`.

`i` enters insert mode on the selected box, leaving `selected` alone. On an
empty canvas it does nothing. **The guard belongs here, on the way in**, so that
insert mode keeps its existing precondition that a box exists — today the only
route in is `b`, which creates one, and `i` is the second route.

This subsumes story 005, which is not yet implemented: `handle_command` has no
`i` branch at all today. 005's `i` targets the last box, which is the same
behaviour as 006's when the selection is the last box, and 005's "cursor at the
end of its existing label" already falls out of `layout()`'s insert-mode cursor.
Whether 005 is archived as delivered is a bookkeeping call, not a design one.

`j` and `k` need no insert-mode handling: `handle_insert` already treats
printable characters as text, so typing `j` inserts a `j`, which is correct.

### Width and the cursor

The `+1` in 004's `width = len(label) + BORDERS + (1 if focused else 0)` existed
because the insert cursor needs a cell past the end of the label. That is now a
genuinely insert-mode-only rule, and the command-mode cursor needs no extra cell
because it sits on a character that is already there. Expressed as interior
width:

    interior = len(label) + 1        if the box is selected and mode is insert
    interior = max(len(label), 1)    otherwise
    width    = interior + BORDERS

The `max(..., 1)` gives an empty box a one-column interior so the command-mode
cursor has somewhere to sit — without it, an empty box is 2 columns (`┌┐`) and
the cursor lands on the left border. A selected empty box therefore draws as
`┌─┐ / │█│ / └─┘`. This applies to unselected empty boxes too, for one uniform
rule; the only width that changes from 004 is the empty command-mode box, 2 to
3. It partially un-retires `test_box_never_shrinks_below_3x3`, for the interior
only. An empty box in insert mode stays 3 wide, as today.

Cursor coordinates, emitted for `nodes[selected]` whenever `selected >= 0`:

    y = box.y + 1
    x = box.x + 1 + len(label)        insert mode
    x = box.x + max(len(label), 1)    command mode

Command mode puts the cursor on the last label character, since characters sit
at `box.x + 1 + offset` and the last offset is `len(label) - 1`.

Boxes stay centered on their own width, `x = (cols - width) // 2`, so the
selected box in insert mode still sits half a column off its neighbours.

### Worked example

Three boxes labelled `a`, `bb`, `c` in command mode with `selected == 0`,
`cols=11`, `rows=11`: `total = 11`, `top = 0`.

| box | interior | width | x | y |
|-----|----------|-------|---|---|
| `a`  | 1 | 3 | 4 | 0 |
| `bb` | 2 | 4 | 3 | 4 |
| `c`  | 1 | 3 | 4 | 8 |

The cursor lands at `x = 4 + max(1, 1) = 5`, `y = 1` — on the `a`, which sits at
`4 + 1 = 5`. Pressing `j` twice moves it to the `c` box; a third `j` leaves it
there.

### Notes deferred

`edit()` *replaces* the box rather than updating it, constructing a fresh
`Box(label)` and discarding the old one. That is lossless while `label` is
`Box`'s only field; the day `Box` gains a colour or an id it would silently drop
it, and `dataclasses.replace(nodes[index], label=label)` is the fix. Left as-is
now — introducing it before there is a second field is speculative.

`edit()` copies the whole node list on every keystroke, O(n) per character. The
same keystroke already rebuilds a `rows × cols` character grid and repaints the
screen, so the copy is noise by orders of magnitude, and the immutability it
buys is what the `does_not_mutate_the_given_state` tests rest on. If undo ever
arrives, the copying becomes an asset.

### Tests

`tests/test_state.py`:

- `State([])` starts with `selected == -1`
- `b` on an empty canvas selects the new box (`selected == 0`)
- `b` on a non-empty canvas selects the new last box
- `b` selects the new box even when the selection was not at the end
- `j` moves the selection down one
- `k` moves the selection up one
- `j` on the bottom box leaves the selection where it is
- `k` on the top box leaves the selection where it is
- `j` and `k` on an empty canvas return the state unchanged
- `j` and `k` do not change `mode`, `running` or `nodes`
- `j` and `k` do not mutate the given state
- `i` enters insert mode
- `i` leaves the selection alone
- `i` on an empty canvas returns the state unchanged
- `i` on a selection that is not the last box types into that box
- `Esc` preserves the selection
- typing edits the selected box, not the last one
- backspace edits the selected box, not the last one
- `edit()` replaces the box at the given index and leaves its neighbours alone
- `edit()` does not mutate the list it is given

The existing `HandleInsertTest` cases construct states like
`State([Box("")], mode="insert")`, which leaves `selected == -1` alongside
non-empty nodes — a violation of the invariant that currently passes because
`nodes[-1]` is the last box. **These should be updated to pass `selected`
explicitly**, so the suite stops depending on the sentinel resolving to the
right answer by luck.

`tests/test_layout.py`:

- command mode emits a cursor placement (replacing 004's "command mode emits no
  cursor placement")
- the command-mode cursor sits on the last character of the selected label
- the insert-mode cursor sits one past the last character, as today
- the cursor follows `selected` rather than the last box: with three boxes and
  `selected == 0`, the cursor placement is geometrically inside the first
- an empty canvas emits no cursor placement
- an empty box has a one-column interior, so a selected empty box is 3 wide and
  its cursor sits on the interior cell
- a selected box in insert mode is one column wider than the same box in
  command mode
- boxes are unaffected by which one is selected, apart from that width

`tests/test_render.py` — unchanged.
