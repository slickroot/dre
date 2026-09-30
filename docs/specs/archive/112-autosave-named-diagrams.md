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

### Approach

Autosave is an effect returned as data. `state::reduce` stays pure and returns `(State, Vec<Effect>)`. A new `EffectExecutor` collaborator on `DreController` runs the effects. This replaces the earlier idea of an `AutosavingReducer` decorator over a `StateStore`, and it follows the option spec 002 and spec 055 left open ("an effect returned as data").

### Detecting a change

Spec 110 made every change undoable, so the history stack is the record of changes. Each altering action pushes a snapshot, undo pops one, and `drop_snapshot_if_unchanged` cancels a push that changed nothing. `State` gains a private `saved_len: usize`, starting at 0, which is the history length when the file was last written. The diagram is unsaved when `history.len() != saved_len`. Moving the selection never touches history, so it never triggers a save.

We don't compare `history.last()` or whole documents. `history.last()` is the state from before the change, not what's on disk, and `Document` includes `selected`.

Equal length really does mean unchanged. History is an unbounded `Vec`, and between saves it only grows, because a change made in Command mode saves immediately and undo is only reachable in Command mode. Undo shrinks the history, so `!=` catches it. A no-op edit pushes and drops its snapshot, so the length returns to `saved_len`.

### Components

- `Effect` (`state/effect.rs`): `enum Effect { Save }`, a unit variant. Derives `Debug` and `PartialEq`.
- `state::reduce` (`state/mod.rs`): returns `(State, Vec<Effect>)`. At the end of a reduce, when `save_to` is `Some`, the mode isn't `Insert` and `history.len() != saved_len`, it sets `saved_len = history.len()` and returns `vec![Effect::Save]`. Otherwise it returns no effects. It stays pure.
- `Reducer::reduce` and `StateReducer` (`editor/controller/reducer.rs`): return `(State, Vec<Effect>)`. No `io::Result`, since reducing does no I/O.
- `EffectExecutor` (`editor/controller/effects.rs`): `fn execute(&self, effects: Vec<Effect>, state: &State) -> io::Result<()>`, mockable like the other collaborators. The real `StoreEffectExecutor` owns a `Box<dyn StateStore>` and matches on the effect: `Effect::Save => self.store.save(state)?`. `StateStore::save` already takes `&State` and reads `save_to`, so `FileStateStore` is untouched.
- `DreController` (`editor/controller/mod.rs`): gains a fourth collaborator, `Box<dyn EffectExecutor>`. In `run`: `let (next, effects) = self.reducer.reduce(state, key); self.executor.execute(effects, &next)?; state = next;`. The `?` propagates a failed save out of the loop.
- `bootstrap::run` (`editor/bootstrap.rs`): builds `StoreEffectExecutor::new(Box::new(FileStateStore::new(Box::new(DiskFiles))))`. `FileStateStore` has no state, so this instance is independent of the one `Editor` owns.
- `Editor::run` (`editor/mod.rs`): unchanged. Its final `store.save` stays, so `q` still saves and the "Save as:" flow is untouched. Command-mode changes are already saved by then, so that last save rewrites the same content. Ctrl-C still clears `save_to`, so the reduce that handles it emits no `Save` and the file stays as the last change left it.
- Remove `State.dirty` (`state/mod.rs`, `state/history.rs`): nothing reads it since the status line was dropped. Done in a separate first commit, along with its two tests, `with_dirty` and any `status_input` use.

### Decisions

- **Executor limit:** the executor is a dispatcher with one port per `Effect` variant and no logic beyond the `match`. An effect that needs more than one collaborator gets its own handler struct, and the executor holds that handler as its one field.
- **No feedback loop:** `execute` returns `io::Result<()>`, not actions. The reducer marks the state as saved when it emits `Save`, so the executor has nothing to report. When a status-line warning on a failed save is built (a later card), the signature becomes `io::Result<Vec<Action>>` and the controller feeds the actions back into `reduce`.
- **`saved_len` over repairing `dirty`:** `dirty` is set in `snapshot` but never cleared, stays `true` after a dropped snapshot, and isn't set by undo. Keeping it in sync would need changes in several places, while `saved_len` follows the history length by construction.
- **Typing a label:** while the mode is `Insert`, no `Save` is emitted. The first reduce that ends outside `Insert` emits it, because the check is `history.len() != saved_len`. Esc returns to Command, so it saves. Enter commits the label but starts the next child, which is `Insert` again, so Enter alone doesn't save. The committed label is written by the next save, on Esc or on any later change.
- **New files:** `dre new.dre` is autosaved too, since `save_to` is set. The first change creates the file. Autosave only checks `save_to`, not `new_file`.
- **No file:** `save_to` is `None`, so no `Save` is emitted and `saved_len` doesn't move. `q` still shows the "Save as:" prompt.
- **Write failure:** the error propagates out of `execute`, the controller and `Editor::run`, and the editor exits with the message. A silent failure would break the promise of autosave.
- **Redo:** the codebase has undo but no redo. Redo will be autosaved for free once it exists, because it will change the history length.

### Tests

The reducer tests are pure and assert on the returned effects. No mock store is needed.

- A key that changes the history length returns `[Effect::Save]`.
- A selection move returns no effects.
- Keys typed in `Insert` return no effects, and leaving `Insert` returns `[Save]` once.
- Enter that starts the next child (still `Insert`) returns no effects. The following Esc returns `[Save]` once, covering every box typed in the chain.
- Undo returns `[Save]`. A no-op edit (snapshot dropped) returns no effects.
- A second change after a save returns `[Save]` again, so `saved_len` is updated.
- With `save_to` set to `None`, no effects are ever returned.
- Ctrl-C after a change returns no effects.

`StoreEffectExecutor`, over a `MockStateStore`:

- `execute(vec![Effect::Save], &state)` calls `save` once with that state.
- `execute(vec![], &state)` doesn't call `save`.
- A `save` that returns an error makes `execute` return that error.

`DreController`, with `MockReducer` and `MockEffectExecutor`:

- The effects returned by `reduce` are passed to `execute` together with the new state.
- An error from `execute` stops the loop and is returned.
- Existing tests keep working with `MockReducer` returning `(state, vec![])` and a permissive `MockEffectExecutor`.
