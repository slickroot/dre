# 001 - Place a box in the center

## Story

Bob opens the sketch tool. He presses `b`, and an empty box appears in the middle of the screen. He's so excited.

## Acceptance Criteria

- Running the tool opens an empty canvas filling the terminal.
- Pressing `b` makes a box appear.
- The box is 3 characters wide by 3 characters tall.
- The box is centered on the screen.
- The box is empty (no label inside).

## Technical Design

Python 3, standard library only. No TUI framework: raw mode via `termios`/`tty`,
drawing via ANSI escapes.

### Pipeline

One pass per keypress:

```
key ──handle_key(state, key)──▶ State
                                  │
                    layout(state, cols, rows)
                                  │
                            [Placement]
                                  │
                 renderer.render(placements, cols, rows)
                                  │
                             list[str]  (grid)
                                  │
                            terminal writer
```

Only the key read and the final write touch the outside world. Everything
between them is pure and tested without a terminal.

### Components

**`Node`** — inert data describing *what* exists. For this story, `Box()`. Nodes
carry no size, no position, and no drawing behaviour.

**`State`** — the nodes that exist: `nodes: list[Node]`. Empty until `b` is
pressed.

**`handle_key(state, key) -> State`** — pure. `b` returns a new state with a
`Box` appended. Unknown keys return the state unchanged. The loop holds no
logic of its own.

**`layout(state, cols, rows) -> list[Placement]`** — pure. Owns *both* size and
position; nodes never know their own dimensions. A box is 3x3 and centered:
`x = (cols - 3) // 2`, `y = (rows - 3) // 2`. `Placement` is
`(node, x, y, width, height)`.

**`Renderer`** — a Protocol with a single method,
`render(placements, cols, rows)`. Each backend owns its whole traversal.

**`TerminalRenderer`** — implements `Renderer`; pure. Walks the placements,
switches on node kind, draws box-drawing characters into a blank grid of
spaces. Returns `rows` strings of `cols` characters each.

**Writer** — impure and deliberately untested. Enters the alternate screen
buffer (`\x1b[?1049h`), hides the cursor, puts the terminal in raw mode;
restores all three on exit, including on exception. Reads terminal size with
`os.get_terminal_size()` once per frame. Full repaint each frame — home the
cursor, write every line. No diffing.

### Test seams

Every acceptance criterion asserts on plain data:

```python
def test_box_is_centered():
    placements = layout(State([Box()]), cols=11, rows=11)
    assert placements == [Placement(Box(), x=4, y=4, width=3, height=3)]

def test_empty_canvas_fills_terminal():
    assert TerminalRenderer().render([], cols=11, rows=5) == ["           "] * 5

def test_box_is_drawn():
    grid = TerminalRenderer().render([Placement(Box(), 4, 4, 3, 3)], 11, 11)
    assert grid[4][4:7] == "┌─┐"
```

No terminal, no mocks, no fakes.

### Decisions and trade-offs

- **Document model over a single hard-coded box.** Nodes + layout + render is
  more machinery than this story alone justifies. Accepted as a deliberate bet
  on where the tool is going.
- **Layout owns size, not the node.** When 003 makes width depend on the label,
  there is one place to change.
- **`Renderer` as a Protocol with one method** rather than `draw_box` /
  `draw_text` primitives. Keeps `TerminalRenderer.render` a pure function; the
  cost is that a second backend re-writes the ~8-line traversal loop. Each
  backend wants different behaviour there anyway (grid clips to bounds, SVG
  does not). Extracting a shared traversal later is mechanical.
- **Pure `handle_key` returning a new `State`** rather than in-place mutation,
  matching the rest of the pipeline.
- **Alternate screen buffer** so that 002's clean exit leaves scrollback
  untouched.

### Deferred

- **Text as a sibling node, not a child of the box.** Revisit at 003, where
  something must relate a label to the box that contains it. Text may turn out
  to be a special case rather than an ordinary node.
- **Quitting** is 002. This story exits via Ctrl-C.
