# State fields are private

Step of spec 117 (one-direction architecture). This is a technical spec, not a
user story: it has no user-facing behaviour.

## Problem

Spec 117 makes `State`'s fields private, so only `state::reduce` can change a
`State`. Spec 122 left them `pub(crate)` ("the fix is the direction of data,
not visibility"). No production code outside `state/` writes a field today,
but any module could, and some tests do.

## Acceptance Criteria

- Every field of `State` is private. Only `state/` and its child modules can
  read or write them.
- Code outside `state/` reads `State` through getters and changes it only
  through `state::reduce`.
- Tests outside `state/` build their scenarios with `#[cfg(test)]` builders in
  `state/`, never by assigning to a field.
- Behaviour is unchanged.

## Technical Design

### Where fields are used outside `state/` today

Production, reads only:

- `doc`: `lib.rs` (`Session::document`, `Session::extent`), `render/svg.rs`
  and `render/terminal.rs` (`layout(state.doc.tree())`), `editor/store`
  (saving).
- `selected`: `render/svg.rs` and `render/terminal.rs` (`with_cursor`).
- `running`: `lib.rs` (`Session::is_running`), `editor/controller` (the loop).
- `save_to`: `editor/store`.

Tests:

- Writes: `render/terminal.rs` (`state.dirty = true`), `editor/mod.rs` and
  `editor/controller` (`state.pending_count = Some(count)`),
  `editor/controller` (`state.running = false` in `stopped`).
- Reads: `pending_count` (`editor/mod.rs`, `editor/controller`), `doc`,
  `selected`, `save_to`, `new_file` (`editor/store`), `selected` (`lib.rs`),
  `running` (`editor/controller`).

### Getters

```rust
impl State {
    pub(crate) fn doc(&self) -> &Document;
    pub(crate) fn selected(&self) -> Option<&[usize]>;
    pub(crate) fn is_running(&self) -> bool;
    pub(crate) fn save_to(&self) -> Option<&str>;

    #[cfg(test)] pub(crate) fn pending_count(&self) -> Option<usize>;
    #[cfg(test)] pub(crate) fn is_new_file(&self) -> bool;
}
```

- Getters exist only for what is read outside `state/`. `mode`, `clipboard`,
  `last_selected`, `history` and `dirty` get none. The status line already
  reads mode, dirty and new-file through `status_input()`.
- `pending_count` and `is_new_file` are only read by tests, so they are
  test-only.
- `is_running` and `is_new_file`, not `running` and `new_file`: the
  constructor `State::new_file(path)` already takes that name, and `Session`
  already calls it `is_running`.
- Callers that need an owned selection, such as `with_cursor`, call
  `.map(<[usize]>::to_vec)`.

### Test builders

`#[cfg(test)]` builder methods on `State`, next to `new_state`, in the style of
spec 124's `Node` builders:

```rust
impl State {
    #[cfg(test)] pub(crate) fn with_pending_count(self, count: usize) -> State;
    #[cfg(test)] pub(crate) fn with_running(self, running: bool) -> State;
    #[cfg(test)] pub(crate) fn with_dirty(self, dirty: bool) -> State;
}
```

- `editor/mod.rs` and `editor/controller`'s `marked(count)` become
  `State::default().with_pending_count(count)`.
- `editor/controller`'s `stopped(state)` becomes `state.with_running(false)`.
- `render/terminal.rs`'s dirty-marker test uses `.with_dirty(true)`.
- They exist in test builds only, so production code still cannot set a field.

### Inside `state/`

Nothing changes. The reducers and `state/`'s own tests are child modules and
keep using the fields directly.

### Considered and rejected

- Keeping `pub(crate)` fields, as spec 122 did: any module can still write
  `State`, and the one-writer rule depends on everyone following it.
- Driving tests into a state by sending keys through `reduce` (for example
  digits for a pending count): correct, but long fixtures, and the tests would
  depend on keymaps they are not about.
- Getters for every field: most are never read outside `state/`.
