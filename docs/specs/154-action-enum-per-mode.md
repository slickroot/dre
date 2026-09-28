# One Action enum per mode

## Story

`command::reduce`, `insert::reduce` and `name_prompt::reduce` each take the whole `Action` enum and end in `_ => state` (`insert.rs`, `name_prompt.rs`) or `_ => state` inside a `(command, selection)` match (`command.rs`). Add a new `Action` variant, forget its arm in the mode it belongs to, and the build still succeeds: the wildcard swallows it and it silently does nothing.

The wildcard in `command::reduce` is hard to remove as things stand, because the same match also has to catch combinations like `(Action::NewSibling, None)` that can't happen once `min_depth` has already turned them away — the tuple shape forces a catch-all.

`ActionSpec.mode` (introduced in spec 153) exists only to route an `Action` to the right `reduce`. If the type of the action already told you its mode, `mode` wouldn't need to exist, and neither would `min_depth`'s awkward "means nothing for Insert and NamePrompt" case, since only Command actions have a minimum selection depth in the first place.

## Acceptance Criteria

- `Action` becomes three exhaustive enums, one per mode — `CommandAction`, `InsertAction`, `NamePromptAction` — wrapped by a slim `Action { Command(CommandAction), Insert(InsertAction), NamePrompt(NamePromptAction) }`.
- `command::reduce`, `insert::reduce` and `name_prompt::reduce` each take their own mode's action enum, not `Action`. Adding a variant to `CommandAction`, `InsertAction` or `NamePromptAction` fails to compile until every match on that enum handles it — no `_` wildcard arm on the action itself in any of the three `reduce` functions.
- `ActionMode` and `ActionSpec.mode` are deleted. Nothing computes or compares a mode at runtime — the compiler already knows it from which enum a value is.
- `min_depth` exists only for `CommandAction`. `InsertAction` and `NamePromptAction` have no `min_depth` concept at all — not a field that's ignored, no field.
- `undoable` is knowable for every action, in every mode, without a wildcard: `CommandAction` and `InsertAction` each declare it exhaustively; `NamePromptAction` is never undoable (no name-prompt action mutates `doc`).
- Everything else behaves as before: same key bindings, same undo/depth/mode behaviour as left by spec 153.

## Out of scope

- No change to key bindings or to what any action does.
- The `COMMANDS` test constant in `src/state/command.rs` stays as it is, adjusted only for its new element type (`CommandAction` instead of `Action`).
- No change to how `min_depth` or `undoable` behave for existing actions — this is a type-shape change, not a behaviour change.

## Technical Design

### The three enums

`src/state/action.rs`:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CommandAction {
    Undo, NewBox, NewSibling, Delete, Paste,
    SelectParent, SelectChild, SelectNext, SelectPrevious,
    EditLabel, RenameLabel, CycleColour, CycleSiblingsColour,
    ToggleSiblingsFill, ToggleSiblingsRounded, ToggleFill, ToggleRounded,
    Quit, Idle, Interrupt, OpenNamePrompt, Digit(u8), CancelCount,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InsertAction {
    Commit, CommitAndAddChild, InsertKey(TextKey),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum NamePromptAction {
    NameAppend(char), NameBackspace, NameConfirm, NameCancel,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Action {
    Command(CommandAction),
    Insert(InsertAction),
    NamePrompt(NamePromptAction),
}
```

`ActionMode` is deleted along with every place that named it.

### Undoability, without a mode

```rust
impl CommandAction {
    pub(crate) fn undoable(&self) -> bool { /* one exhaustive match, no `_` */ }
}
impl InsertAction {
    pub(crate) fn undoable(&self) -> bool { /* one exhaustive match, no `_` */ }
}
impl Action {
    pub(crate) fn undoable(&self) -> bool {
        match self {
            Action::Command(a) => a.undoable(),
            Action::Insert(a) => a.undoable(),
            Action::NamePrompt(_) => false,
        }
    }
}
```

`history::recorded` calls `action.undoable()` instead of `action.spec().undoable`. `ActionSpec` (from spec 153) is deleted along with `Action::spec()`.

### `min_depth`, only where it means something

```rust
impl CommandAction {
    pub(crate) fn min_depth(&self) -> usize { /* one exhaustive match, no `_` */ }
}
```

`command::reduce` takes `CommandAction` and reads `command.min_depth()` directly. There is no `min_depth` anywhere on `InsertAction`, `NamePromptAction` or the outer `Action`.

### Three separate, exhaustive `reduce`s

`command::reduce(state: State, command: CommandAction) -> State`, `insert::reduce(state: State, command: InsertAction) -> State`, `name_prompt::reduce(state: State, command: NamePromptAction) -> State`.

`insert::reduce` and `name_prompt::reduce` match on their (now 3- and 4-variant) enums exhaustively — no `_ => state` / `_ => {}` arm.

`command::reduce`'s depth guard (`if depth < command.min_depth() { return state; }`) still runs before the per-action dispatch, so by the time dispatch happens every action that needs a selection has one. The dispatch itself should match on `CommandAction` alone (not the `(CommandAction, Option<Vec<usize>>)` tuple used today) and pull the now-guaranteed path out of `state.selected` inside each arm, so the match is exhaustive over `CommandAction` with no wildcard. (`Undo`, `NewBox`, `Paste`, `Quit` have `min_depth` 0 and already handle a possibly-absent selection internally, same as today.)

### Dispatch in `state/mod.rs`

```rust
fn apply(mut state: State, action: action::Action) -> State {
    if let Some(selected) = state.last_selected.take() {
        state.selected = Some(selected);
    }
    history::recorded(state, &action, |state| match action {
        Action::Insert(a) => insert::reduce(state, a),
        Action::NamePrompt(a) => name_prompt::reduce(state, a),
        Action::Command(a) => command::reduce(state, a),
    })
}
```

This match is exhaustive over the three outer variants; nothing here decides based on a `mode` value.

### `input::parse`

`command_parse`, `insert_parse` and `name_prompt_parse` (`src/state/input.rs`) already only ever produce one mode's actions. They keep returning their own action type (`Option<CommandAction>`, `Option<InsertAction>`, `Option<NamePromptAction>`) and `parse` wraps the result in `Action::Command`/`Action::Insert`/`Action::NamePrompt` at the single point where the three converge:

```rust
pub(crate) fn parse(state: &State, key: &str) -> Option<Action> {
    match &state.mode {
        Mode::Command => command_parse(key).map(Action::Command),
        Mode::Insert { .. } => insert_parse(key).map(Action::Insert),
        Mode::NamePrompt { .. } => name_prompt_parse(key).map(Action::NamePrompt),
    }
}
```

`state::reduce`'s `Action::Idle` / `Action::Interrupt` construction (for the no-key and Ctrl-C cases) becomes `Action::Command(CommandAction::Idle)` / `Action::Command(CommandAction::Interrupt)`.

`COMMAND_KEYMAP` (`src/state/input.rs`) is typed `&[KeyBinding<CommandAction>]` instead of `&[KeyBinding<Action>]`.

### Tests

- Every test that builds an `Action::Foo` for a command action now builds `CommandAction::Foo` (and passes it to `command::reduce` / wraps it as `Action::Command(...)` where the outer type is needed, e.g. calls through `state::reduce`/`handle_key`). Same rewrite for Insert and NamePrompt actions.
- `command.rs`'s `COMMANDS` constant becomes `[CommandAction; 18]`.
- `command.rs`'s `a_command_below_its_minimum_depth_leaves_the_document_unchanged` reads `command.min_depth()`.
- `history.rs`'s undoability tests call `.undoable()` (on `CommandAction`/`InsertAction`, or on the wrapping `Action` where that's what's under test) instead of `.spec().undoable`.
- No test should need `ActionSpec` or `ActionMode` — grep for both after the change; if either name still appears anywhere in `src/`, something wasn't deleted.
