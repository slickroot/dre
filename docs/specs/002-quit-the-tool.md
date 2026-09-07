# 002 - Quit the tool

## Story

Bob is done looking at his box. He presses `q`, the tool closes, and he goes back to sleep.

## Acceptance Criteria

- Pressing `q` closes the tool.
- The terminal is left in a clean, usable state.

## Technical Design

Nothing new is built. This story adds one field, one branch, and the loop
condition that reads them. `Node`, `layout` and `Renderer` are untouched.

### Pipeline

Unchanged from 001, with the loop now driven by state rather than by Ctrl-C:

```
        ┌──────────────────────────────┐
        │        while state.running   │
        │                              │
   render(state) ──▶ read_key() ──▶ handle_key(state, key) ──▶ State
        │                                                        │
        └────────────────────────────────────────────────────────┘
```

### Components

**`State`** — gains `running: bool = True` alongside `nodes: list[Node]`.
Quitting is ordinary data, not control flow. `State` was already application
state rather than a document, so the flag belongs here.

**`handle_key(state, key) -> State`** — stays pure. `q` returns a new state
with `running=False`. Everything else is unchanged, as before.

**Writer** — unchanged from 001. `try/finally` around the loop restores raw
mode, the cursor and the main screen buffer, including on exception. Impure and
deliberately untested.

### Loop shape

Render at the *top* of the loop, not the bottom:

```python
try:
    while state.running:
        write(renderer.render(layout(state, *size()), *size()))
        state = handle_key(state, read_key())
finally:
    restore()
```

One render site. The empty canvas on startup falls out for free, and `q` exits
without painting a final frame nobody will see.

### Test seams

Both acceptance criteria are covered by one new pure test each side of the
line — and the second one isn't automated:

```python
def test_q_stops_the_loop():
    assert handle_key(State([Box()]), "q").running is False

def test_state_starts_running():
    assert State([]).running is True

def test_other_keys_keep_running():
    assert handle_key(State([]), "b").running is True
```

"The terminal is left in a clean, usable state" is verified by hand. It lives
entirely in the writer, which is the one layer we've agreed not to test.

### Decisions and trade-offs

- **A `running` flag on `State`, not an exception or a command object.**
  `handle_key`'s signature from 001 is unchanged and the test seam stays plain
  data. A `(State, Command)` return would generalise to future effects (save,
  redraw), but we have exactly one effect, so it's machinery bought on
  speculation.
- **No context manager for terminal setup/teardown.** A `terminal_session()`
  context manager would make setup and restore structurally inseparable and
  guard against a future story stranding the terminal in raw mode. Rejected for
  now: the writer is a thin shell of side effects that we're choosing to leave
  untested, and a bare `try/finally` already covers today's single call site.
  Revisit if the loop grows more exit paths.
- **`q` quits unconditionally.** See Deferred.
- **Render before read, not after handle.** Avoids both a duplicated initial
  render and a dead final frame.

### Correction to 001

001 states that it "exits via Ctrl-C". That is not accurate: `tty.setraw()`
clears `ISIG`, so Ctrl-C never raises `KeyboardInterrupt` — it arrives as a
plain `\x03` byte and is ignored as an unknown key. Under 001 alone the tool
has no working exit, which is what this story supplies.

### Deferred

- **`q` collides with 003.** In 003 typed characters go straight into the box
  with no mode to enter, so typing a label beginning with `q` would quit. Left
  unresolved on purpose: 003 must decide how keys are dispatched once text
  entry exists, and inventing a focus or mode model here would be guessing at
  that decision one story early. 003 already rewrites `handle_key`; this is one
  more line in a function it was going to touch.
- **Ctrl-C does nothing.** `\x03` falls through as an unknown key. Mapping it
  to quit would cost one branch and would never collide with 003, since it
  isn't printable — but `q` satisfies the story, so it stays out until
  something asks for it.
