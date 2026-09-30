# state/ module with one reducer

Step of spec 117 (one-direction architecture). This is a technical spec, not a
user story: it has no user-facing behaviour.

## Problem

About 150 places outside `state.rs` read and write `State` fields. Any mode, `editor.rs`, `lib.rs` or `cli.rs` can change the state, so nothing guarantees that a change is undoable or that state changes in one place.

## Acceptance Criteria

- `state.rs` becomes `state/` with per-mode reducers as child modules and one public `reduce(State, Option<&str>) -> State`.
- Outside `state/`, only `state::State` and `state::reduce` are public. `input`, `Action`, `Mode` and the history helpers are private to `state/`.
- `reduce` is the only writer. `editor.rs`, `lib.rs` and `cli.rs` no longer assign a `State` field: they use the `State` constructors and `reduce`.
- `reduce` takes the undo snapshot for every undoable action, in every mode.
- `State` fields stay `pub(crate)`. The fix is the direction of data, not visibility.
- Undo keeps today's behaviour, selection included. The snapshot-only-the-`Document` change and the selection fallback are left to a later spec.
- `editor.rs` tests build scenarios through `State::open` and `reduce`, not by assigning fields.

## Technical Design

Decided in the design session. Structure follows spec 117 (`state/`), with the departures listed below.

### Public surface

Outside `state/` there are two public things: `state::State` and `state::reduce`.

```rust
pub fn reduce(state: State, key: Option<&str>) -> State
```

`None` means the terminal was idle. `Some(key)` is a key press. `Action` is private to `state/`. `editor.rs` never sees one.

`State` fields stay `pub(crate)`, so `render`, `editor.rs` and `Session` read them directly and there are no getters. This departs from spec 117, which made them private. The rule is enforced by direction of data: nothing outside `state/` writes a field.

Constructors (associated functions on `State`, they write nothing):

- `State::default()`: empty. Used by `Session::new` and `editor::load(None)`.
- `State::open(doc, save_to: Option<String>)`: replaces `state::load`, selecting the first box. `cli::export` uses `State::open(doc, None)`.
- `State::new_file(path)`: replaces `state::new_file`.

### Layout

```
state/
  mod.rs          State, Mode, constructors, status_input, reduce  <- the public surface
  input.rs        key -> Action: parse, keymaps, KeyBinding (moved from input.rs)
  command.rs      Command-mode reducer (from command_mode.rs)
  insert.rs       Insert-mode reducer (from insert_mode.rs)
  save_prompt.rs  Save-prompt reducer (from save_prompt_mode.rs)
  history.rs      is_undoable, snapshot, drop-if-unchanged, undo
```

Deleted: `input.rs`, `command_mode.rs`, `insert_mode.rs`, `save_prompt_mode.rs`, `reduce.rs`. `action.rs` moves into `state/` as a private module. This departs from spec 117, which kept `input.rs` outside `state`.

### The reduce flow

1. `reduce(state, key)` maps the key to an `Action` with `input::parse(&state, key)`. `None` maps to `Action::Idle`. The `INTERRUPT` key maps to `Action::Interrupt`. A key with no action returns the state unchanged.
2. It restores `last_selected` (moved from the old `reduce.rs`).
3. It snapshots if `history::is_undoable(&state, &action)`, then dispatches by `action.mode()` to the mode reducer.
4. It drops the snapshot if the `Document` is unchanged.

### New actions

Both are Command-mode and not undoable.

- `Action::Idle`: replaces `hide_idle_cursor`. Sets `last_selected` and clears the selection when in Command mode.
- `Action::Interrupt`: sets `save_to = None` and `running = false`. This replaces the write and the `break` in `editor.rs`, so the loop has one exit, `while state.running`. The reducer stays pure: `editor.rs` still writes the file after the loop.

### Undo snapshots

- `history::is_undoable(&State, &Action)` covers all three modes. It takes `&State` because a digit is undoable only under the colour overlay.
- The mode reducers never call `snapshot` or `drop_snapshot_if_unchanged`. Both become private to `history.rs`.
- Drop-if-unchanged becomes general: after any undoable action, if the `Document` equals the last snapshot, the snapshot is popped.
- Risk: `CommitAndAddChild` snapshots twice today. Pin its current behaviour with tests before moving it.
- Undo is unchanged: it restores the whole `Document`, selection included.

### `editor.rs`

- The loop is `while state.running`: render, `next_key()`, skip resize (it only touches the renderer), otherwise `state = reduce(state, key)`.
- The scroll branch (`selected_box_edges`, `overflow_delta`) is removed. `scroll_x` and `Action::ScrollBy` stay in `state` for now, with no producer in `editor.rs`. Removing them from `state` and `render` is a later step.

### `lib.rs` and `cli.rs`

- `Session::press_key` calls `reduce(state, Some(key))`. `go_idle` calls `reduce(state, None)`.
- `cli::export` uses `State::open(doc, None)` and no longer assigns `state.doc`.

### Tests

- Keep `#[cfg(test)] new_state(boxes, mode, selected)` inside `state/`.
- `editor.rs` tests use `State::open(doc, save_to)` and `reduce`, not field assignment.
- `render` tests may keep assigning display fields (`colour_overlay`, `dirty`, `scroll_x`, `doc`). They set up what to draw and do not exercise the writer rule.

### Considered and rejected

- Private fields with getters: rejected by decision. The direction of data is the fix.
- `input` as a public function beside `reduce`: this would make `Action` public.
- A public `Input { Key, Idle }` enum: this would be a third public type. `Option<&str>` is enough.
- `Action::Load(doc)` for construction: this would need a public way to send an `Action`.
- Doing the undo selection fallback here: it belongs to the spec that moves the selection out of `Document`.
