# 046 - Command letters as an enum

## User Story

As a maintainer, I want the command-mode key-to-action mapping defined once as an enum instead of scattered string literals, so that changing which letter triggers a command means editing a single line instead of hunting through dispatch code and tests.

## Acceptance Criteria

- A `Command` enum in `dre/state.py` has one member per command-mode action, with each member's value set to that action's current letter: `UNDO="u"`, `NEW_BOX="b"`, `SELECT_PARENT="h"`, `SELECT_CHILD="l"`, `SELECT_NEXT="j"`, `SELECT_PREVIOUS="k"`, `EDIT_LABEL="i"`, `RENAME_LABEL="I"`, `CYCLE_COLOUR="c"`, `CYCLE_SIBLINGS_COLOUR="C"`, `CYCLE_FILL="f"`, `TOGGLE_ROUNDED="r"`, `QUIT="q"`.
- `handle_command` still takes `key: str` (the raw keypress). Every `if key == "x":` branch is replaced with a comparison against a `Command` member.
- A key string that doesn't match any `Command` member leaves `state` unchanged, exactly as today's fall-through behavior.
- `UNDOABLE_KEYS` is renamed `UNDOABLE_COMMANDS` and holds `Command` members (`NEW_BOX`, `CYCLE_COLOUR`, `CYCLE_FILL`, `TOGGLE_ROUNDED`, `CYCLE_SIBLINGS_COLOUR`, `RENAME_LABEL`) instead of raw letters.
- `tests/test_state.py` no longer hardcodes raw letters when driving `handle_key`/`handle_command`; call sites use `Command.<NAME>.value` instead, so a test suite still passes unmodified if a command's letter is reassigned.
- `handle_insert`'s handling of `\x1b` (escape), `\x7f` (backspace), and the printable-character range is unchanged — those are protocol-given keys, not assignable commands, and are out of scope for this story.

## Technical Design

### The `Command` enum

Defined in `dre/state.py`, next to `Mode`/`Path`, since it's only used there today:

```python
from enum import Enum

class Command(Enum):
    UNDO = "u"
    NEW_BOX = "b"
    SELECT_PARENT = "h"
    SELECT_CHILD = "l"
    SELECT_NEXT = "j"
    SELECT_PREVIOUS = "k"
    EDIT_LABEL = "i"
    RENAME_LABEL = "I"
    CYCLE_COLOUR = "c"
    CYCLE_SIBLINGS_COLOUR = "C"
    CYCLE_FILL = "f"
    TOGGLE_ROUNDED = "r"
    QUIT = "q"
```

Each member's value is the single source of truth for that command's letter — reassigning a letter means editing exactly one line here.

### Converting the raw keypress

`handle_command` keeps its `key: str` signature (the terminal reader and `handle_key` are untouched), and converts at the very top:

```python
def handle_command(state: State, key: str) -> State:
    try:
        command = Command(key)
    except ValueError:
        return state
    if command in UNDOABLE_COMMANDS:
        state = replace(state, before=replace(state, before=None))
    if command == Command.UNDO:
        return state.before if state.before is not None else state
    if command == Command.NEW_BOX:
        ...
```

An unrecognized `key` (e.g. a stray keypress) fails `Command(key)` with `ValueError`, caught immediately to return `state` unchanged — matching today's fall-through-to-`return state` behavior at the bottom of the function.

### `UNDOABLE_COMMANDS`

```python
UNDOABLE_COMMANDS = {
    Command.NEW_BOX,
    Command.CYCLE_COLOUR,
    Command.CYCLE_FILL,
    Command.TOGGLE_ROUNDED,
    Command.CYCLE_SIBLINGS_COLOUR,
    Command.RENAME_LABEL,
}
```

Checked against the already-converted `command` variable rather than the raw string.

### Dispatch branches

Every branch's condition changes from `if key == "x":` to `if command == Command.NAME:`; branch bodies are unchanged.

### `handle_insert` and `handle_key`

Untouched. `handle_insert` keeps comparing raw `\x1b`, `\x7f`, and the `\x20`-`\x7e` printable range directly — these come from the terminal/ASCII, not from an assignable command set. `handle_key` keeps dispatching on `state.mode` and passing the raw `key: str` through to whichever handler applies.

### Tests

`tests/test_state.py` call sites like `handle_key(state, "b")` become `handle_key(state, Command.NEW_BOX.value)`. This keeps the calls simulating a raw keypress (matching production usage) while removing every hardcoded letter, so the suite stays correct if a `Command` value is reassigned.
