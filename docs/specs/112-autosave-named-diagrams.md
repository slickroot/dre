## Story

Bob opens `plans.dre`, adds a box called "Orders" and presses Enter. The file on disk is updated right away. He renames a box, deletes another and undoes the delete, and the file follows each time. Then his terminal crashes. He runs `dre plans.dre` again and everything is there. Later he presses `q` and DRE quits, saving one last time.

## Acceptance Criteria

- After I open an existing diagram (`dre plans.dre`), the file is saved after every action that alters the diagram: adding, renaming, deleting or moving a box, changing colour or fill, and undo or redo.
- While I'm typing a label, nothing is saved until I confirm it with Enter.
- Moving the selection doesn't save anything.
- If DRE or the terminal crashes, running `dre plans.dre` again shows the diagram as of my last change.
- `q` still saves and quits, without a prompt.
- Ctrl-C quits and leaves the file as my last change left it.
- A diagram started without a file still shows the "Save as:" prompt on `q`, and isn't autosaved. That's a later card.

## Technical Design

### Detecting a change

Spec 110 made every change undoable, so the history stack is the record of changes. Each altering action pushes a snapshot, undo pops one, and `drop_snapshot_if_unchanged` cancels a push that changed nothing. The history length therefore differs from its value at the last save exactly when the diagram has changed. Moving the selection never touches history, so it never triggers a save.

We don't compare `history.last()` or whole documents. `history.last()` is the state from before the change, not what's on disk, and `Document` includes `selected`.

### Components

- `State::history_len()` (`state.rs`): a read-only accessor, since `history` is private.
- `edit()` (`editor.rs`): gains a `save: impl FnMut(&State) -> io::Result<()>` parameter, like `next_key` and `probe`, and keeps a local `saved_len`, starting at 0 after load. After each `handle_key`, when `save_to` is `Some`, the mode isn't `Insert` and `state.history_len() != saved_len`, it calls `save(&state)?` and sets `saved_len`.
- `open()` (`editor.rs`): passes a `save` closure that runs `filesystem::write(path, dre_format::write(file_document::from_document(&state.doc)))`. The final write after the loop stays, so `q` still saves. Ctrl-C still clears `save_to`, so it writes nothing more and the file stays as the last change left it.

### Decisions

- **Typing a label:** while the mode is `Insert`, nothing is saved. The save happens when Esc or Enter returns the mode to Command, or when Enter starts the next child, which pushes a snapshot.
- **New files:** `dre new.dre` where the file doesn't exist is autosaved too. The first change creates the file. Autosave only checks `save_to`, not `new_file`.
- **No file:** `save_to` is `None`, so nothing is autosaved, and `q` still shows the "Save as:" prompt.
- **Write failure:** the error propagates out of `edit()` and the editor exits with the message. A silent failure would break the promise of autosave. A status-line warning while editing continues is a later card.
- **Redo:** the codebase has undo but no redo. Redo will be autosaved for free once it exists, because it will change the history length.

### Tests

At the `edit()` level, using a scripted `next_key` and a recording `save` closure. They check only whether `save` was called. There is no per-action test.

- A key that changes the history length calls `save` once.
- A selection move doesn't call `save`.
- Keys typed in `Insert` don't call `save`, and leaving `Insert` calls it once.
- With `save_to` set to `None`, `save` is never called.
- A `save` that returns an error makes `edit()` return that error.
- Ctrl-C after a change leaves `save_to` as `None` and doesn't call `save` again.
