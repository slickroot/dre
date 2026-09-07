# 003 - Type a label inside the box

## Story

Bob presses `b` and a box appears. He starts typing straight away, and his label appears inside the box, which widens to fit as he goes. He mistypes, hits backspace, and the box shrinks back down with the text.

## Acceptance Criteria

- After the box appears, typed characters go straight into the box - no mode to enter first.
- A visible cursor inside the box shows where the next character will land.
- The box widens as the label grows so the text fits.
- The box stays centered on screen as it grows, expanding in both directions.
- Backspace removes the last character.
- The box shrinks back as the label gets shorter, staying centered.
- The box never shrinks below 3x3.

## Technical Design

The label is a field on `Box`, not a node of its own. Two modes exist, but `b`
enters insert mode as part of creating the box, so Bob never enters one by
hand. The pipeline from 001 is unchanged.

### Pipeline

Unchanged. `handle_key` grows a dispatch, `Placement` grows a field:

```
key ──handle_key(state, key)──▶ State  (nodes, running, mode)
                                  │
                    layout(state, cols, rows)
                                  │
                    [Placement(..., cursor)]
                                  │
                 renderer.render(placements, cols, rows)
                                  │
                             list[str]  (grid)
```

### Components

**`Box`** — gains `label: str = ""`. Still inert, still no size and no
position.

**`State`** — gains `mode: Literal["command", "insert"] = "command"` alongside
`nodes` and `running`. A fresh canvas starts in command mode, so 002's `q`
still quits.

**`handle_key(state, key) -> State`** — pure, and now a two-branch dispatch:

```python
def handle_key(state, key):
    if state.mode == "insert":
        return handle_insert(state, key)
    return handle_command(state, key)
```

*Command mode* — `b` appends `Box("")` **and** sets `mode="insert"`; `q` sets
`running=False`. Unknown keys return the state unchanged, as in 001.

*Insert mode* — `\x1b` (Esc) returns to command mode. `\x7f` (backspace) drops
the last character of the last box's label, and is a no-op on an empty label.
Printable characters (`\x20`–`\x7e`) append to the last box's label. Everything
else returns the state unchanged. `b` and `q` are ordinary letters here.

Focus is implicit: typing edits **the last box in `nodes`**. There is only ever
one box in this story.

**`layout(state, cols, rows) -> list[Placement]`** — pure, and still the sole
owner of size and position:

```python
width  = len(node.label) + 3       # two borders, the label, one cursor cell
height = 3
x      = (cols - width) // 2
y      = (rows - height) // 2
cursor = len(node.label) if state.mode == "insert" else None
```

The 3x3 minimum is not enforced, it is *implied*: an empty label gives
`0 + 3 = 3`. There is no `max()` and no branch to get wrong. Re-centering on
every keystroke falls out of recomputing `x` from the new width.

**`Placement`** — gains `cursor: int | None`, the interior column offset where
the next character will land, or `None` outside insert mode. Layout answers the
question; the renderer only paints the answer.

**`TerminalRenderer`** — implements `Renderer`; still pure. Draws the border as
in 001, the label at `(x + 1, y + 1)`, and — when `cursor is not None` — a `█`
at `x + 1 + cursor`.

**Writer** — unchanged from 001 and 002. The real terminal cursor stays hidden;
the visible cursor is a painted glyph in the grid.

### Test seams

Every acceptance criterion is a plain-data assertion. No terminal, no mocks.

```python
def test_typing_goes_straight_into_the_box():
    state = handle_key(State([]), "b")
    assert state.mode == "insert"
    assert handle_key(state, "h").nodes == [Box("h")]

def test_box_widens_to_fit_the_label():
    p = layout(State([Box("hi")], mode="insert"), cols=11, rows=11)
    assert p == [Placement(Box("hi"), x=3, y=4, width=5, height=3, cursor=2)]

def test_box_stays_centered_as_it_grows():
    x = lambda s: layout(State([Box(s)], mode="insert"), 21, 11)[0].x
    assert [x(""), x("ab"), x("abcd")] == [9, 8, 7]

def test_backspace_removes_the_last_character():
    state = State([Box("hi")], mode="insert")
    assert handle_key(state, "\x7f").nodes == [Box("h")]

def test_box_never_shrinks_below_3x3():
    p = layout(State([Box("")], mode="insert"), cols=11, rows=11)[0]
    assert (p.width, p.height) == (3, 3)

def test_backspace_on_empty_label_is_a_no_op():
    state = State([Box("")], mode="insert")
    assert handle_key(state, "\x7f") == state

def test_cursor_is_drawn_after_the_label():
    grid = TerminalRenderer().render(
        [Placement(Box("hi"), 0, 0, 5, 3, cursor=2)], 5, 3)
    assert grid[1] == "│hi█│"

def test_no_cursor_in_command_mode():
    assert layout(State([Box("hi")]), 11, 11)[0].cursor is None

def test_letters_do_not_trigger_commands_while_typing():
    state = State([Box("")], mode="insert")
    assert handle_key(state, "q").running is True
    assert handle_key(state, "b").nodes == [Box("b")]

def test_esc_returns_to_command_mode():
    assert handle_key(State([Box("")], mode="insert"), "\x1b").mode == "command"
```

### Decisions and trade-offs

- **Label as a field on `Box`, not a sibling `Text` node.** 001 deferred this.
  A sibling node needs a pointer back to its box — by ID, by reference, or by
  list position — because the box's *size is derived from the label*, so
  something must relate the two. That pointer buys nothing in this story: there
  is one box and one label, no free-floating text, and no nesting. A real
  document tree can be introduced when a story asks for one, and moving a field
  into a node is mechanical. This reverses 001's guess, which is what "revisit
  at 003" was for.

- **Two modes, but `b` enters insert mode for you.** The story says "no mode to
  enter first", and this satisfies it literally — Bob presses `b` and types.
  The alternative, letting printable keys always win once a box exists, would
  have forced 002's `q` criterion to be rewritten as Esc. Instead a fresh
  canvas is in command mode and 002's test passes unchanged. The cost is that
  `q` now means two different things depending on state; Esc is the only way
  back, and nothing on screen indicates the mode. A mode indicator is the
  obvious next ask.

- **The cursor is painted into the grid, not the terminal's real cursor.**
  Driving the real cursor would mean `render` returning both a grid and a
  position, pushing the criterion into the writer — the one layer we've agreed
  not to test. Painting it keeps 001's `placements → grid` shape and makes
  "a visible cursor" an ordinary string assertion. The price is no blink.

- **The cursor cell is always reserved; only the glyph is conditional.** Width
  is `len + 3` in both modes, so the box does not jump when Bob presses Esc,
  and `layout`'s sizing stays independent of `mode`. In command mode the extra
  interior column reads as padding.

- **`cursor` on `Placement` rather than `mode` passed to `render`.** Keeps the
  renderer a function of placements alone, and keeps the "where does the next
  character go" rule in `layout`, next to the width rule it has to agree with.

- **Implicit focus — the last box in `nodes`.** An explicit focused-node field
  would need node identity, which needs ID minting on `State`. With one box it
  is machinery bought on speculation. It is also the first thing a second box
  will demand.

### Correction to 001

`Placement` gains a sixth field, so 001's `test_box_is_centered` becomes
`Placement(Box(), x=4, y=4, width=3, height=3, cursor=None)`. 001 also
specified the width as the constant `3`; it is now `len(label) + 3`, which
yields 3 for the empty box 001 tested.

### Deferred

- **Labels wider than the terminal.** Past `cols - 3` characters, `x` goes
  negative and the renderer writes outside the grid. No criterion covers it and
  it takes a deliberate 80-key hold to reach. The fix is a `min()` in `layout`
  plus a slice in the renderer, both in functions this story already rewrites.
  Left out on purpose; the first person to hit it can decide whether the label
  should clip or the keystroke should be refused.
- **A second box.** `b` in command mode appends another box, and `layout`
  centers every box at the same coordinates, so they would stack invisibly.
  Whichever story introduces multiple boxes has to bring positioning, focus and
  node identity with it — all three at once, which is why none of them are
  here.
- **No mode indicator.** Nothing on screen distinguishes command from insert.
- **Ctrl-C still does nothing**, as in 002.
