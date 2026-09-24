# state/ module with one reducer

Step of spec 117 (one-direction architecture). This is a technical spec, not a
user story: it has no user-facing behaviour.

## Problem

About 150 places outside `state.rs` read and write `State` fields. Any mode, `editor.rs`, `lib.rs` or `cli.rs` can change the state, so nothing guarantees that a change is undoable or that state changes in one place.

## Acceptance Criteria

- `state.rs` becomes `state/` with private fields, per-mode reducers as child modules and one public `reduce(State, Action) -> State`.
- `reduce` is the only writer. Outside `state/` no code assigns a field.
- `reduce` takes the undo snapshot for every undoable action.
- Direct writes in `editor.rs`, `lib.rs` and `cli.rs` become `Action`s.
- Undo snapshots only the `Document`. After undo an invalid selection falls back to the nearest surviving ancestor, then the first top-level box, then `None`.
- Tests build scenarios through a test constructor, not by assigning fields.

## Technical Design

Structure and rules are fixed in spec 117 (`state/`, Selection, Undo and the selection). The `Action` and getter lists are still to be designed before implementation.
