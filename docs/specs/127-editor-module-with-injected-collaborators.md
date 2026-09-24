# Editor module with injected collaborators

This is a technical spec, not a user story: it has no user-facing behaviour.

## Problem

`editor.rs` is three free functions. `load` calls `filesystem::read` and `dre_format::read` directly, so its tests write real temp files. `edit` takes its dependencies as closures (`next_key`, `probe`, `output`, `renderer`), and one of its tests counts kitty sprites in real rendered bytes. `open` does the terminal setup, the wiring and the save all together. Nothing in the editor can be replaced, so nothing can be tested without I/O.

## Acceptance Criteria

- `editor.rs` becomes `editor/`, made of objects whose collaborators are traits injected as `Box<dyn Trait>`. No generic parameters.
- `StateStore` loads and saves a `State`. `DreController` reads keys, calls `reduce` and renders. `Editor` calls load, then the controller, then save.
- `editor::open` is the composition root: the only place that builds real implementations, and the only untested function.
- Every editor test is a unit test with fakes. No temp files, no terminal, no real renderer.
- `render/`, `terminal.rs` and `filesystem.rs` do not change and do not know about `editor`.
- Behaviour stays the same, with two accepted differences: the kitty check now runs before the file is read, and a bad file fails after raw mode starts (the `RawScreen` drop restores the terminal before the error is printed).

## Technical Design

Decided in the design session.

### Layout

```
editor/
  mod.rs         Editor, open (the composition root)
  store.rs       trait StateStore, trait Files, FileStateStore
  controller.rs  trait KeySource, trait Screen, DreController
  terminal.rs    the real implementations: DiskFiles, TerminalKeys, TerminalScreen
```

Traits live with the code that uses them. The real implementations live in `editor/` and depend outward on `filesystem`, `terminal`, `kitty` and `render`, so spec 117's one-direction rule holds. Each file's `#[cfg(test)]` holds its own fakes.

### Injection

A collaborator is a trait. Whatever holds it keeps a `Box<dyn Trait>`, which is the Rust spelling of an interface-typed field. There are no generics on the holders, so no `DreController<K, S>`.

### `StateStore` (`store.rs`)

```rust
pub(crate) trait StateStore {
    fn load(&self, path: Option<&str>) -> io::Result<State>;
    fn save(&self, state: &State) -> io::Result<()>;
}

pub(crate) trait Files {
    fn read(&self, path: &str) -> io::Result<String>;
    fn write(&self, path: &str, contents: &str) -> io::Result<()>;
}

pub(crate) struct FileStateStore {
    files: Box<dyn Files>,
}
```

- One object owns the `dre_format` + `file_document` conversion in both directions, so load and save cannot drift apart.
- `load` keeps today's three-way branch: `None` gives `State::default()`, a `NotFound` read gives `State::new_file(path)`, and a successful read is parsed into `State::open(doc, Some(path))` or fails with `filesystem::invalid(path)`.
- `save` writes `state.doc` to `state.save_to`, and does nothing when `save_to` is `None`.
- A separate `StateSaver` was rejected: load and save share the same format and path.

### `DreController` (`controller.rs`)

```rust
pub(crate) trait KeySource {
    fn next_key(&mut self) -> io::Result<Option<String>>;
}

pub(crate) trait Screen {
    fn render(&mut self, state: &State) -> io::Result<()>;   // owns the output, flushes
    fn resize(&mut self) -> io::Result<()>;                  // re-probes and resizes itself
}

pub(crate) struct DreController {
    keys: Box<dyn KeySource>,
    screen: Box<dyn Screen>,
}

impl DreController {
    pub(crate) fn run(&mut self, state: State) -> io::Result<State> {
        let mut state = state;
        while state.running {
            self.screen.render(&state)?;
            match self.keys.next_key()? {
                Some(key) if key == terminal::RESIZE => self.screen.resize()?,
                key => state = reduce(state, key.as_deref()),
            }
        }
        Ok(state)
    }
}
```

- Two collaborators, not four. The output and `probe` move inside `Screen`: the controller never writes bytes and never measures the terminal.
- `Screen` takes no `Write` argument. `Renderer::render(&mut self, …, out: &mut impl Write)` has a generic method, so it cannot be a `dyn` trait.
- Resize stays as one branch in the controller. `next_key` cannot handle it: it does not hold the renderer, and nothing it could return is neutral to `reduce`. `None` means idle, and a `RESIZE` key parses to `CancelCount` in Command mode (`state/input.rs`).
- `DreController` never sees a `StateStore`. It gets a `State` and returns one.

### `Editor` and `open` (`mod.rs`)

```rust
pub(crate) struct Editor {
    store: Box<dyn StateStore>,
    controller: DreController,
}

impl Editor {
    pub(crate) fn run(&mut self, path: Option<&str>) -> io::Result<()> {
        let state = self.store.load(path)?;
        let state = self.controller.run(state)?;
        self.store.save(&state)
    }
}
```

- `DreController` is held as a concrete struct: there is one, and its collaborators are already faked.
- `open(file: Option<String>) -> io::Result<ExitCode>` stays the entry point called by `lib::run`. It does the procedural terminal setup, which is startup plumbing and not a collaborator: `kitty::require`, `terminal::probe`, the `RawScreen` guard and `install_resize_pipe`. It builds the real implementations and calls `editor.run(file.as_deref())`.
- `_raw` is declared before `editor`, so on any error `editor` drops first, then `RawScreen` restores the terminal, then `lib::run` prints the error.
- A bad file: `load` returns `InvalidData`, the controller never runs, `save` is never called, the file is untouched, and the user sees today's message after a possible one-frame flash of the alternate screen. A later spec may catch this error in `Editor::run` and offer to start a new file.

### Real implementations (`editor/terminal.rs`)

Thin adapters of a few lines each, with no logic and no tests:

- `DiskFiles` implements `Files` with `filesystem::read` and `filesystem::write`.
- `TerminalKeys { fd, resize_fd }` implements `KeySource` with `terminal::poll_read(fd, resize_fd, IDLE_TIMEOUT_MS)`.
- `TerminalScreen { renderer: TerminalRenderer, out: Stdout }` implements `Screen`. `render` calls `renderer.render(state, &mut out)` and flushes. `resize` calls `renderer.on_resize(terminal::probe()?)`.

`TerminalRenderer` does not implement `Screen` and does not change. It stays a `Renderer` beside `SvgRenderer`, and its tests keep rendering into a `Vec<u8>`.

### Tests

All unit tests with fakes, each in the file it tests:

- `store.rs`: `FakeFiles` backed by `HashMap<String, String>`, where a missing key reads as `NotFound`. The 9 `load` tests move over without temp files. New `save` tests: writes the document to `save_to`, writes nothing when `save_to` is `None`, and round-trips through `load`.
- `controller.rs`: `ScriptedKeys` (a `Vec<Option<String>>` that errors when it runs out) and `RecordingScreen` (clones every rendered `State` and counts `resize` calls). They replace the `edit` tests: interrupt clears `save_to`, quit keeps it, a frame is rendered before the first key is read, `RESIZE` calls `resize` once and never `reduce`, and a failed `resize` propagates.
- The sprite-counting idle test becomes a `RecordingScreen` test: the frame after `None` has no selection, and the frame after the next key has it back. How the cursor is drawn stays covered by the renderer tests.
- `mod.rs`: `Editor::run` with a fake store and a `DreController` on fakes. It loads, runs, then saves the final state. A failed load neither renders nor saves.

### Considered and rejected

- Generics on holders (`DreController<K: KeySource, …>`): the type parameters spread to every holder.
- Closures wrapped in structs: they are the injection style being replaced.
- Two loaders chosen by `open` (`NewStateLoader`, `FileStateLoader`): one `load(path: Option<&str>)` is simpler and keeps one fake.
- `DreController` calling `load` itself: the controller would own the whole session and need a fake store in every test.
- `TerminalRenderer` implementing `Screen`: it would own stdout and call `probe`, which breaks its `Vec<u8>` tests and the public `Renderer` trait it shares with `SvgRenderer`.
- Adapters next to what they wrap (`render/terminal.rs`, `terminal.rs`, `filesystem.rs`): those modules would depend on `editor` traits, against spec 117's direction.
- Resize handled inside `next_key`: see `DreController` above.
- A `TerminalScreen` test over a `Box<dyn Write>` buffer: all tests use fakes.
