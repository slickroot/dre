# Editor module with injected collaborators

This is a technical spec, not a user story: it has no user-facing behaviour.

## Problem

`editor.rs` is three free functions. `load` calls `filesystem::read` and `dre_format::read` directly, so its tests write real temp files. `edit` takes its dependencies as closures (`next_key`, `probe`, `output`, `renderer`), and one of its tests counts kitty sprites in real rendered bytes. `open` does the terminal setup, the wiring and the save all together. Nothing in the editor can be replaced, so nothing can be tested without I/O.

## Acceptance Criteria

- `editor.rs` becomes `editor/`, made of objects whose collaborators are traits injected as `Box<dyn Trait>`. No generic parameters.
- `StateStore` loads and saves a `State`. `DreController` reads keys, passes them to an injected `Reducer` and renders. `Editor` calls load, then the controller, then save.
- `editor::open` is the composition root: the only place that builds real implementations, and the only untested function.
- Every editor test is a unit test. Every collaborator is a `mockall` mock. No temp files, no terminal, no real renderer, no real `state::reduce`. Editor tests check which calls an object makes, not what its collaborators do.
- `render/`, `terminal.rs` and `filesystem.rs` do not change and do not know about `editor`.
- Behaviour stays the same, with two accepted differences: the kitty check now runs before the file is read, and a bad file fails after raw mode starts (the `RawScreen` drop restores the terminal before the error is printed).

## Technical Design

Decided in the design session.

### Layout

```
editor/
  mod.rs         Editor, open (the composition root)
  store.rs       trait StateStore, trait Files, FileStateStore
  controller.rs  trait KeySource, trait Screen, trait Reducer, DreController
  terminal.rs    the real implementations: DiskFiles, TerminalKeys, TerminalScreen, StateReducer
```

Traits live with the code that uses them. The real implementations live in `editor/` and depend outward on `filesystem`, `terminal`, `kitty`, `render` and `state`, so spec 117's one-direction rule holds.

### Injection

A collaborator is a trait. Whatever holds it keeps a `Box<dyn Trait>`, which is the Rust spelling of an interface-typed field. There are no generics on the holders, so no `DreController<K, S, R>`.

### Mocks

`mockall` is added as a `[dev-dependencies]` entry. Every collaborator trait is marked `#[cfg_attr(test, mockall::automock)]`, which generates `MockFiles`, `MockStateStore`, `MockKeySource`, `MockScreen` and `MockReducer` in test builds only. `editor/` has no hand-written fakes.

- A test states each expected call with `expect_…()`, `.withf(…)`, `.times(n)` or `.never()`, and `.returning(…)`.
- Call counts are verified when the mock is dropped, so they still hold after the mock is moved into a `Box<dyn Trait>`. The test never needs to look inside a collaborator after `run`.
- Order across collaborators, such as render before the first key, uses a `mockall::Sequence`.

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

pub(crate) trait Reducer {
    fn reduce(&self, state: State, key: Option<&str>) -> State;
}

pub(crate) struct DreController {
    keys: Box<dyn KeySource>,
    screen: Box<dyn Screen>,
    reducer: Box<dyn Reducer>,
}

impl DreController {
    pub(crate) fn run(&mut self, state: State) -> io::Result<State> {
        let mut state = state;
        while state.running {
            self.screen.render(&state)?;
            match self.keys.next_key()? {
                Some(key) if key == terminal::RESIZE => self.screen.resize()?,
                key => state = self.reducer.reduce(state, key.as_deref()),
            }
        }
        Ok(state)
    }
}
```

- Three collaborators: keys, screen and reducer. The output and `probe` move inside `Screen`: the controller never writes bytes and never measures the terminal.
- The reducer is injected. The controller never calls `state::reduce`, so its tests do not run application logic. `Reducer` takes the raw `Option<&str>` key and not an `Action`, so key parsing stays inside `state` and the controller never sees an `Action`.
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
- `StateReducer` implements `Reducer` with `state::reduce(state, key)`. `state/` does not change and does not know about `editor`.

`TerminalRenderer` does not implement `Screen` and does not change. It stays a `Renderer` beside `SvgRenderer`, and its tests keep rendering into a `Vec<u8>`.

### Tests

These are unit tests. Each one tests one object, with every collaborator mocked, and lives in the file it tests. A test checks which calls the object makes and what it does with the results. It never checks what a real collaborator would do.

- `store.rs`: `MockFiles`. `read` returns the file contents, or an `ErrorKind::NotFound` error. The 9 `load` tests move over without temp files. New `save` tests: `write` is called once with `save_to` and the serialised document, and `write` is never called when `save_to` is `None`.
- `controller.rs`: `MockKeySource`, `MockScreen` and `MockReducer`. `MockReducer` is kept minimal: it only proves `reduce` was called with the right key, and its `returning` sets `running = false` when the test needs the loop to stop. It contains no reducer logic. Tests:
  - A frame is rendered before the first key is read (`Sequence`).
  - A key read from `KeySource` is passed to `reduce` (`withf`, `times(1)`).
  - An idle `None` is passed to `reduce` as `None`.
  - The state `reduce` returns is the one rendered next.
  - The loop stops when `reduce` returns `running == false`, and `run` returns that state.
  - `RESIZE` calls `resize` once and `reduce` never.
  - A failed `resize` or `next_key` propagates.
- `mod.rs`: `MockStateStore` and a `DreController` on mocks. `Editor::run` calls `load` with the path, runs the controller, then calls `save` with the final state. A failed `load` never renders and never saves.

Deleted, not moved: the `edit` tests for interrupt clearing `save_to`, quit keeping it, and the sprite-counting idle test. They test `state::reduce` through the controller, and `state/` already covers them (`command.rs` `interrupt_stops_running_and_drops_the_save_path`, the `"q"` tests, and `mod.rs`'s idle-hide and `any_key_after_a_hide_restores_the_selection`). How the cursor is drawn stays covered by the renderer tests.

### Considered and rejected

- Generics on holders (`DreController<K: KeySource, …>`): the type parameters spread to every holder.
- Closures wrapped in structs: they are the injection style being replaced.
- Two loaders chosen by `open` (`NewStateLoader`, `FileStateLoader`): one `load(path: Option<&str>)` is simpler and keeps one fake.
- `DreController` calling `load` itself: the controller would own the whole session and need a fake store in every test.
- `TerminalRenderer` implementing `Screen`: it would own stdout and call `probe`, which breaks its `Vec<u8>` tests and the public `Renderer` trait it shares with `SvgRenderer`.
- Adapters next to what they wrap (`render/terminal.rs`, `terminal.rs`, `filesystem.rs`): those modules would depend on `editor` traits, against spec 117's direction.
- Resize handled inside `next_key`: see `DreController` above.
- A `TerminalScreen` test over a `Box<dyn Write>` buffer: all tests use mocks.
- `DreController` calling `state::reduce` directly: every controller test would run application logic, so the tests would not be unit tests.
- A `Reducer` that takes an `Action`: the controller would have to parse keys.
- Hand-written fakes with `Rc<RefCell<…>>` logs to inspect them after they are boxed: `mockall` checks calls on drop, with no shared state.
- `into_parts()` or other accessors on `DreController` to get the fakes back: production API used only by tests.
