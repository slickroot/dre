## Story

Bob opens `plans.dre`, adds a box called "Orders" and presses Esc. The file on disk is updated right away. He renames a box, deletes another and undoes the delete, and the file follows each time. Then his terminal crashes. He runs `dre plans.dre` again and everything is there. Later he presses `q` and DRE quits, saving one last time.

## Acceptance Criteria

- After I open an existing diagram (`dre plans.dre`), the file is saved after every action that alters the diagram: adding, renaming or deleting a box, changing colour or fill, and undo.
- While I'm typing a label, nothing is saved until I leave Insert mode with Esc. Pressing Enter to start the next box doesn't save by itself; the next Esc or change does.
- Moving the selection doesn't save anything.
- If DRE or the terminal crashes, running `dre plans.dre` again shows the diagram as of my last change.
- `q` still saves and quits, without a prompt.
- Ctrl-C quits and leaves the file as my last change left it.
- `dre new.dre`, where the file doesn't exist yet, is autosaved too. The first change creates the file.
- If a save fails, DRE exits and shows the error, instead of carrying on unsaved.
- A diagram started without a file still shows the "Save as:" prompt on `q`, and isn't autosaved. That's a later card.

## Technical Design

### Detecting a change

Spec 110 made every change undoable, so the history stack is the record of changes. Each altering action pushes a snapshot, undo pops one, and `drop_snapshot_if_unchanged` cancels a push that changed nothing. The history length therefore differs from its value at the last save exactly when the diagram has changed. Moving the selection never touches history, so it never triggers a save.

We don't compare `history.last()` or whole documents. `history.last()` is the state from before the change, not what's on disk, and `Document` includes `selected`.

### Components

- `State::history_len()` (`state/mod.rs`): a read-only accessor, since `history` is private.
- `Reducer::reduce` (`editor/controller/reducer.rs`): returns `io::Result<State>` instead of `State`, because a save can fail. `DreController::run` propagates it with `?`.
- `AutosavingReducer` (`editor/controller/reducer.rs`): implements `Reducer` as a decorator over an inner `Box<dyn Reducer>` and a `Box<dyn StateStore>`, and keeps a `saved_len`, starting at 0. After the inner `reduce`, when `save_to` is `Some`, the mode isn't `Insert` and `state.history_len() != saved_len`, it calls `store.save(&state)?` and sets `saved_len`.
- `bootstrap::run` (`editor/bootstrap.rs`): passes `AutosavingReducer::new(Box::new(StateReducer), Box::new(FileStateStore::new(Box::new(DiskFiles))))` to `DreController`, which keeps its three collaborators. `FileStateStore` has no state, so this second instance is independent of the one `Editor` owns.
- `Editor::run` (`editor/mod.rs`): unchanged. Its final `store.save` stays, so `q` still saves. Ctrl-C still clears `save_to`, so the decorator saves nothing more and the file stays as the last change left it.

### Decisions

- **Typing a label:** while the mode is `Insert`, nothing is saved. The save happens on the first reduce that ends outside `Insert`, and it is automatic because the check is `history_len != saved_len`. Esc returns the mode to Command, so it saves. Enter commits the label but starts the next child, which is `Insert` again, so Enter alone doesn't save. The committed label is written by the next save, on Esc or on any later change. No special case is needed.
- **New files:** `dre new.dre` where the file doesn't exist is autosaved too. The first change creates the file. Autosave only checks `save_to`, not `new_file`.
- **No file:** `save_to` is `None`, so nothing is autosaved, and `q` still shows the "Save as:" prompt.
- **Write failure:** the error propagates out of `reduce`, the controller and `Editor::run`, and the editor exits with the message. A silent failure would break the promise of autosave. A status-line warning while editing continues is a later card.
- **`state.dirty`:** left alone. Nothing reads it since the status line was dropped, so autosave doesn't clear it.
- **Redo:** the codebase has undo but no redo. Redo will be autosaved for free once it exists, because it will change the history length.

### Tests

At the `AutosavingReducer` level, wrapping the real `StateReducer` (or a mock inner `Reducer`) over a `MockStateStore`. They check only whether `save` was called. There is no per-action test.

- A key that changes the history length calls `save` once.
- A selection move doesn't call `save`.
- Keys typed in `Insert` don't call `save`, and leaving `Insert` calls it once.
- Enter that starts the next child (still `Insert`) doesn't call `save`. The following Esc calls it once, covering every box typed in the chain.
- With `save_to` set to `None`, `save` is never called.
- A `save` that returns an error makes `reduce` return that error.
- Ctrl-C after a change leaves `save_to` as `None` and doesn't call `save` again.

`DreController`'s tests keep working with `MockReducer` returning `Ok(state)`, plus one test that an error from `reduce` stops the loop and is returned.
