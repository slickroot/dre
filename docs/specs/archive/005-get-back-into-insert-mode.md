# 005 - Get back into insert mode

## Story

Bob presses `Esc` and looks at his box. He presses `i`, the cursor reappears at the end of the label, and he carries on typing where he left off.

## Acceptance Criteria

- Pressing `i` in command mode enters insert mode on the last box, with the cursor at the end of its existing label.
- Typing then continues that box's label.
- On a canvas with no boxes, `i` does nothing.

## Technical Design
This story is one branch in `handle_command`. Everything the cursor needs is
already in place: `layout()` emits a cursor placement inside the last box
whenever `state.mode == "insert"`, positioned at `x = box.x + 1 + len(label)`,
and `TerminalRenderer` already draws a `Cursor` node. So `layout.py` and
`render.py` are untouched, and the new tests all live in `tests/test_state.py`.

### Components

**`handle_command`** (`state.py`) — gains one responsibility: mapping `i` to a
mode switch. It knows the key and the current state; it produces a new `State`
with `mode="insert"` and the nodes and `running` flag carried over unchanged.
Collaborators: `State` (reads `nodes`, constructs).

Nothing else changes. `handle_insert` already appends to the last box's existing
label, so "typing continues that box's label" falls out with no new code.

### The branch

    if key == "i" and state.nodes:
        return State(state.nodes, state.running, "insert")

The `state.nodes` guard is the only place the empty-canvas rule lives.
`handle_insert` reads `state.nodes[-1].label` unguarded and would raise on an
empty list; we keep that precondition rather than duplicating a defensive check,
because this guard makes insert mode with no boxes unreachable — `b` and `i` are
the only ways in, and `b` appends a box first.

With no boxes, `i` falls through to `handle_command`'s existing
`return state`, the same path unknown keys take. It returns the identical
object, matching `test_unknown_key_returns_the_state_unchanged`. We deliberately
do not write an explicit `if key == "i" and not state.nodes: return state`: "no
box, nothing to enter" is exactly what the shared fallthrough means.

### Focus

`i` enters insert mode on the *last* box, keeping the assumption `layout()` and
`relabel_last` already share. Story 006 introduces an explicit selection and
will change `i` to read it; when that happens the guard here becomes
"a box is selected" and the change stays inside `handle_command`.

### Tests

`tests/test_state.py`, in `HandleKeyTest`:

- `i` enters insert mode when a box exists
- `i` preserves the nodes, including their labels
- `i` keeps the state running
- `i` does not mutate the given state
- `i` on an empty canvas leaves the mode as command
- `i` on an empty canvas returns the state unchanged

And in `HandleInsertTest`, the round trip Bob actually performs:

- after `Esc` then `i`, typing appends to the last box's existing label rather
  than replacing it
- with two boxes, `i` then typing edits only the last one

`test_typing_appends_to_an_existing_label` already covers `i` being an ordinary
printable character once we are in insert mode; it must keep passing.
