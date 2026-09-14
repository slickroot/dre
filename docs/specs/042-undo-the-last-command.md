# Undo the last command

As a sketch user, I want to press `u` to undo my last command so that I can recover from a mistake without redoing my work.

## Acceptance Criteria

- After performing a single box-changing command (`b`, `c`, `f`, `r`, or `C`), pressing `u` restores the state (boxes and selection) to exactly what it was before that command.
- Pressing `u` when there is no previous action to undo (e.g. at the very start, or after already undoing) leaves the state unchanged.
- Undo only applies to command-mode actions; insert-mode typing is out of scope for this story.

## Technical Design

`State` gains a `before` field holding a single snapshot of the prior state:

```python
@dataclass(frozen=True)
class State:
    boxes: Tuple[Box, ...] = ()
    running: bool = True
    mode: Mode = "command"
    selected: Path = ()
    before: Optional["State"] = None
```

Undo is capped at a single level: the snapshot stored in `before` always has its own `before` set to `None`, so undoing twice in a row is a no-op rather than a redo.

At the top of `handle_command`, before dispatching on `key`, stamp a fresh snapshot whenever `key` is one of the undoable commands:

```python
UNDOABLE_KEYS = {"b", "c", "f", "r", "C"}

def handle_command(state: State, key: str) -> State:
    if key in UNDOABLE_KEYS:
        state = replace(state, before=replace(state, before=None))
    ...
```

Because this reassigns `state` before the existing `if key == ...` branches run, every branch's `replace(state, ...)` call automatically carries the new `before` forward — no other branch needs to change. This applies unconditionally to the five keys, even when a branch's own guard clause leaves `boxes`/`selected` unchanged (e.g. `c` with nothing selected); in that case `before` ends up equal to the current state, which is harmless since undoing restores an identical state.

Keys outside this set — movement (`h`, `j`, `k`, `l`), `i`, `q`, and all insert-mode keys (handled by `handle_insert`, which never touches `before`) — leave the existing `before` snapshot untouched, so `u` still reaches back to the last undoable command even after intervening navigation or an insert session.

`u` is a new branch in `handle_command`, alongside the others:

```python
if key == "u":
    return state.before if state.before is not None else state
```

Restoring `state.before` naturally clears further undo (since that snapshot's own `before` is `None`), satisfying the requirement that undoing when there's nothing to undo — including immediately after an undo — leaves state unchanged.
