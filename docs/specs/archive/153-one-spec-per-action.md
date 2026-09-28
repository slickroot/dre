# One spec per action

## Story

Each `Action` has three facts: which mode it belongs to, whether it is undoable, and how deep the selection has to be. These facts live in three separate matches. `Action::mode()` (`src/state/action.rs`) is exhaustive. `is_undoable` (`src/state/history.rs`) uses `matches!`, so a new action is silently **not undoable**. `min_depth` (`src/state/command.rs`) ends in `_ => 1`, so a new action silently **needs a selection**. When adding an action it is easy to forget the two non-exhaustive ones, and the compiler won't say anything. Doug won't see a difference, apart from the one history change below.

`recorded()` also hard-codes two more per-action rules: `EditLabel` keeps its snapshot even when nothing changed, and `Commit` drops one. That is a fourth place to remember.

## Acceptance Criteria

- Every action's mode, undoability and minimum depth are declared together in one exhaustive match with no wildcard arm. Adding an `Action` variant fails to compile until its spec is written.
- `Action::mode()`, `is_undoable` and `min_depth` are deleted.
- `recorded()` names no actions.
- Every undoable action leaves exactly one undo step, even if it changed nothing. Editing a label from `Label` to `Label` is an undo step.
- Everything else behaves as before.

## Out of scope

- No change to key bindings or to what any action does.
- The `COMMANDS` test constant in `src/state/command.rs` stays as it is.

## Technical Design

### `ActionSpec`

`src/state/action.rs` gains:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ActionSpec {
    pub(crate) mode: ActionMode,
    pub(crate) undoable: bool,
    pub(crate) min_depth: usize,
}

impl Action {
    pub(crate) fn spec(&self) -> ActionSpec { /* one exhaustive match, no `_` arm */ }
}
```

`Action::mode()` is deleted and its callers use `action.spec().mode`. The spec is data only. It knows nothing about `State`.

### The table

| Action | mode | undoable | min_depth |
|---|---|---|---|
| `Undo`, `Quit` | Command | no | 0 |
| `NewBox`, `Paste` | Command | yes | 0 |
| `NewSibling`, `Delete`, `EditLabel`, `RenameLabel`, `CycleColour`, `CycleSiblingsColour`, `ToggleFill`, `ToggleSiblingsFill`, `ToggleRounded`, `ToggleSiblingsRounded` | Command | yes | 1 |
| `SelectChild`, `SelectNext`, `SelectPrevious` | Command | no | 1 |
| `SelectParent` | Command | no | 2 |
| `Idle`, `Interrupt`, `OpenNamePrompt`, `Digit(_)`, `CancelCount` | Command | no | 0 |
| `CommitAndAddChild` | Insert | yes | 0 |
| `Commit`, `InsertKey(_)` | Insert | no | 0 |
| `NameAppend(_)`, `NameBackspace`, `NameConfirm`, `NameCancel` | NamePrompt | no | 0 |

Notes:
- `Idle`, `Interrupt`, `OpenNamePrompt`, `Digit` and `CancelCount` return from `command::reduce` before the depth check. Today they fall into `_ => 1`, which doesn't matter. Their spec says 0 because they don't need a selection.
- `min_depth` means nothing for Insert and NamePrompt actions. They get 0.
- `Commit` stops being special. It was only there to close the snapshot `EditLabel` opened.

### History: no unchanged check

`drop_snapshot_if_unchanged` is deleted. `recorded()` becomes:

```rust
pub(super) fn recorded(state: State, action: &Action, reduce: impl FnOnce(State) -> State) -> State {
    let state = if action.spec().undoable { snapshot(state) } else { state };
    reduce(state)
}
```

Consequences, accepted:
- `EditLabel` followed by `Commit` with no change leaves one snapshot, so one `u` is a no-op.
- `p` with an empty clipboard, or `0p`, leaves a no-op undo step. Autosave fires because `history.len()` changed (`src/state/mod.rs`), and it rewrites the same content.

### `command::reduce`

`if depth < min_depth(command)` becomes `if depth < command.spec().min_depth`.

### Tests

- `history.rs` tests move from `is_undoable(&a)` to `a.spec().undoable`. The cases stay the same, plus `Commit` is not undoable.
- `action.rs`'s `actions_map_to_their_mode` moves to `.spec().mode`.
- `command.rs`'s `a_command_below_its_minimum_depth_leaves_the_document_unchanged` reads `command.spec().min_depth`.
- `committing_an_edit_label_without_a_change_leaves_history_unchanged` (`src/state/mod.rs`) flips to: committing an unchanged edit leaves one snapshot, and one undo restores the document.
- New: a paste with an empty clipboard leaves one snapshot.
- `commit_and_add_child_without_a_change_keeps_the_edit_and_the_commit_snapshots` already expects 2 and stays.
