# Rustify the state reducer

## User Story

As a maintainer learning Rust, I want `src/state.rs` rewritten around Rust's ownership model instead of the copy-on-every-change style inherited from the original Python `state.py`, so that the reducer reads as a pipeline of small free functions, the `rewrite`/clone-closure machinery disappears entirely, and undo becomes a real multi-level history rather than a single stashed snapshot.

## Acceptance Criteria

- `rewrite` is deleted, along with all eight `|node: &Node| { let mut n = node.clone(); ...; n }` closures that call it. No function in `state.rs` takes a `&dyn Fn` callback.
- `at` is the single tree-traversal helper, with signature `fn at<'a>(boxes: &'a mut [Node], path: &[usize]) -> &'a mut Node`. There is no second read-only variant; every call site reads or writes through the one function.
- Every command handler is a free function of the form `fn name(state: State) -> State`, taking `State` **by value**. No `impl State` block gains behavioural methods; the only `impl` blocks on `State`/`Node`/`Document` are `Default` (and derives).
- `handle_key` becomes `fn handle_key(state: State, key: &str) -> State` — the `&` is dropped from the state parameter, so no clone is needed to produce the next state.
- Key decoding is split from state transition: `fn parse(key: &str) -> Option<Command>` is pure and independently testable, and `fn reduce(state: State, command: Command) -> State` never sees a keystroke. An unrecognised key returns the state untouched.
- A new `Document { boxes: Vec<Node>, selected: Vec<usize> }` type holds everything undo restores. `State` becomes `{ doc: Document, history: Vec<Document>, mode: Mode, running: bool }`.
- The `before: Option<Box<State>>` field is gone. Undo is a stack: `snapshot` pushes `state.doc.clone()`, `undo` pops and assigns. Pressing `u` repeatedly walks back through the whole session's history.
- History is unbounded — no cap, no eviction, no `VecDeque`.
- The set of undoable commands is unchanged: `NewBox`, `CycleColour`, `CycleFill`, `ToggleRounded`, `CycleSiblingsColour`, `RenameLabel`. `NewSibling` and `EditLabel` remain non-undoable, exactly as today.
- The nine scattered `if state.selected.is_empty()` / `len() <= 1` guards are replaced by one `min_depth(command) -> usize` table checked once at the top of `reduce`. Command handlers contain no selection guards.
- `mode: String` becomes a private `Mode` enum with variants `Command` and `Insert`. The field is not `pub(crate)` — nothing outside `state.rs` reads it.
- `State::new`'s five-argument constructor is deleted in favour of `State::default()`, whose `running` is `true`. All three call sites in `writer.rs` pass identical arguments today.
- Paths and indices are `usize`, not `i64`. Every `as usize` / `as i64` cast in `state.rs` disappears.
- `render.rs` and `layout.rs` are not modified. `writer.rs` changes only where it names `state.boxes`/`state.selected` (now under `.doc`), constructs a `State`, and calls `handle_key`.
- Behaviour is unchanged except for multi-level undo. `colour`/`fill` keep their `i64` + `PLAIN = -1` representation; converting them to `Option<u8>` is deferred to its own spec.

## Technical Design

### Why the current shape exists

`state.rs` is a faithful transliteration of the Python `state.py`. In Python, sharing a mutable tree between the reducer and the renderer is a correctness hazard, so the only safe discipline is *never mutate; always return a new value with the change applied*. `rewrite` is the tool that implements that discipline, and the whole module is built around it.

Rust enforces at compile time that nothing else can observe a value while it is being modified, so the discipline is defending against a bug the language already prevents. What remains is the cost: changing one `i64` currently clones the selected node, rebuilds every node on the path from the root, copies the top-level `Vec`, and is preceded by a full `state.clone()` — on every keystroke.

Notably, this already-expensive design does *not* buy cheap undo: undo is hand-rolled via an explicit `before` snapshot. The module pays for immutability and still snapshots manually.

### Ownership, not methods

The fix is not to convert free functions into `&mut self` methods — that would trade a functional design for an object-oriented one. It is to change how the state is passed:

```rust
fn cycle_colour(mut state: State) -> State {
    let node = at(&mut state.doc.boxes, &state.doc.selected);
    node.colour = next_colour(node.colour);
    state
}
```

`&State` means "lend me this, I may not touch it" — which forces a borrower to fabricate a copy in order to return a changed version. `State` (no `&`) means "this is mine now", which permits modification because no one else holds it. Handing ownership over is a *move*, not a copy, and costs nothing.

The result is a pure interface — `State` in, `State` out, indistinguishable from a rebuilding reducer — over an implementation that mutates in place. Mutation is confined to function bodies and is invisible to callers.

### The single traversal helper

```rust
fn at<'a>(boxes: &'a mut [Node], path: &[usize]) -> &'a mut Node {
    let mut node = &mut boxes[path[0]];
    for &i in &path[1..] {
        node = &mut node.children[i];
    }
    node
}
```

This replaces both the old cloning `at` (which returned an owned, deep-copied subtree merely so callers could read `.label` or `.children.len()`) and all of `rewrite`.

Rust's standard library convention is to offer `get`/`get_mut` pairs, because a library must serve callers it has never met and `&T`/`&mut T` are distinct types. That does not apply here: all four call sites live inside functions that own the state, and a `&mut` reference can be read through. A read-only variant would be dead weight.

Taking `&mut self.doc.boxes` alongside `&self.doc.selected` borrows two disjoint fields, which the compiler accepts. This is why `at` stays a free function over `&mut [Node]` rather than becoming a method — `self.at(&self.selected)` would not compile.

`rewrite`'s callback parameter (`f: &dyn Fn(&Node) -> Node`) is the last artifact of the copying design: it exists only to manufacture a replacement node for a caller that cannot modify one. Given edit access, the callback has no remaining purpose, so `at` hands back the node directly. (It also removes a dynamic-dispatch indirection.)

`colour_row` stops calling `rewrite` once per sibling — a full tree copy per sibling — and becomes a direct iteration:

```rust
let parent = &state.doc.selected[..state.doc.selected.len() - 1];
let siblings = if parent.is_empty() {
    &mut state.doc.boxes
} else {
    &mut at(&mut state.doc.boxes, parent).children
};
let first = siblings[0].colour;
let new = if siblings.iter().all(|b| b.colour == first) { next_colour(first) } else { 0 };
for sibling in siblings.iter_mut() {
    sibling.colour = new;
}
```

### `Document` and the undo stack

Storing history as `Vec<State>` would be recursive — each stored `State` carries its own history. The existing code avoids this by blanking the field by hand (`snapshot.before = None`), which works but relies on remembering.

Splitting out the unit of undo removes the possibility rather than managing it:

```rust
struct Document {
    boxes: Vec<Node>,
    selected: Vec<usize>,
}

struct State {
    doc: Document,
    history: Vec<Document>,
    mode: Mode,
    running: bool,
}

fn snapshot(mut state: State) -> State {
    state.history.push(state.doc.clone());
    state
}

fn undo(mut state: State) -> State {
    if let Some(previous) = state.history.pop() {
        state.doc = previous;
    }
    state
}
```

`Document` has no `history` field, so `Vec<Document>` cannot recurse.

Excluding `mode` and `running` from the snapshot is behaviour-preserving. `undo` is only reachable from command mode, and snapshots are only taken in command mode, so restoring `mode` is always a no-op; no undoable command alters `running`.

History is unbounded. A `Node` measures 72 bytes (half of which is the `String` and `Vec` handles, independent of contents), so a 50-box drawing snapshots at roughly 4KB. Thousands of edits cost single-digit megabytes for the lifetime of one session. A cap is one line at the push site if it ever becomes warranted, and needs no design change.

For context, this is the problem git solves with content-addressed objects: identical subtrees hash to the same object and are stored once, so a commit allocates new objects only along the path to the change. `rewrite` implements git's path-rebuild without git's sharing — all of the work, none of the benefit. The equivalent here would be `Rc<Node>` children, which is rejected: it would push a sharing concept into the `Node` type that `render.rs` and `layout.rs` consume, to save megabytes that never materialise.

### One guard table

`at` indexes `path[0]` and panics on an empty path — a reachable state, since the app starts with nothing selected. Rather than nine scattered guards, each command declares the selection depth it requires:

```rust
fn min_depth(command: Command) -> usize {
    match command {
        Command::Undo | Command::NewBox | Command::Quit => 0,
        Command::SelectParent | Command::CycleSiblingsColour => 2,
        _ => 1,
    }
}
```

checked once, before dispatch:

```rust
fn reduce(state: State, command: Command) -> State {
    if state.doc.selected.len() < min_depth(command) {
        return state;
    }
    let state = if is_undoable(command) { snapshot(state) } else { state };
    match command {
        Command::Undo => undo(state),
        Command::NewBox => new_box(state),
        // ... one arm per command
    }
}
```

These values reproduce today's guards exactly: depth 0 for `Undo`/`NewBox`/`Quit` (currently unguarded), depth 2 for `SelectParent`/`CycleSiblingsColour` (currently `len() <= 1`), depth 1 for the remaining nine (currently `is_empty()`).

Making `at` return `Option<&mut Node>` was considered and rejected: it trades nine guards for roughly twelve unwrap sites, each of which must still decide what "no box" means.

`insert` uses `at` without a guard. This is safe because insert mode is only entered by `NewBox`, `NewSibling`, `EditLabel`, or `RenameLabel`, all of which guarantee a selection. That invariant is currently true by accident and should carry a comment.

### Mode, construction, indices

`mode: String` compared against `"insert"`/`"command"` becomes:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Mode {
    #[default]
    Command,
    Insert,
}
```

`writer.rs` never reads `mode`, so the field becomes fully private, and `handle_key` dispatches on it by `match` rather than string equality.

`State::new(Vec::new(), true, "command".to_string(), Vec::new(), None)` is the only form used at all three `writer.rs` call sites, so the constructor is replaced by a `Default` impl with `running: true`. The tests in `state.rs` build `State { .. }` literals directly and are unaffected by its removal, since they are a child module with access to private fields.

Paths become `Vec<usize>` because `at` indexes with `usize`; this removes every cast in the module.

### Call-site impact

`writer.rs` only:

- `frame` (line 112): `state.boxes` / `state.selected` become `state.doc.boxes` / `state.doc.selected`.
- Lines 120, 187, 199: `State::new(...)` becomes `State::default()`.
- Line 128: `state = handle_key(&state, &key)` becomes `state = handle_key(state, &key)`.
- Line 123: `while state.running` is unchanged.

`render.rs` and `layout.rs` read `Node`'s fields, which keep their names, types and visibility. Neither file changes.

### Test impact

The existing suite is the safety net for this refactor and its assertions should be preserved.

- The `new_state` helper gains the `Document` nesting.
- The three direct `at` tests pass `&mut boxes` and dereference the result: `assert_eq!(*at(&mut boxes, &[1]), node("b"))`.
- `parse` becomes independently testable without constructing a `State`, as does each command handler.
- New coverage is required for multi-level undo: several undoable commands followed by repeated `u`, walking back to the initial document, and a further `u` on empty history leaving the state unchanged.

### Out of scope

`colour` and `fill` remain `i64` with the `PLAIN = -1` sentinel. Modelling them as `Option<u8>` — where `None` means plain and the compiler forces the plain case to be handled — is the right representation and would shrink `Node` from 72 to 56 bytes, but `PLAIN` is referenced roughly thirty times across `render.rs` and its tests. That change lands in its own follow-up spec so it does not move in the same commit as the reducer rewrite.
