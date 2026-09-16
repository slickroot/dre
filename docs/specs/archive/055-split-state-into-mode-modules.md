# Split state into mode modules

## User Story

As a maintainer, I want `src/state.rs` split so that each editor mode owns its own module — its vocabulary of commands, and the rules about when those commands apply — so that adding a command means editing the module for the mode that command belongs to, and so that the shape of the code says what a user can actually do.

## Acceptance Criteria

- `src/state.rs` is split into three modules: `state.rs`, `command_mode.rs`, `insert_mode.rs`. `main.rs` declares all three.
- `state.rs` keeps only what every mode shares: the types (`Node`, `Document`, `State`, `Mode`), the constants (`PLAIN`, `PALETTE_SIZE`, `PAD`), the tree and palette primitives (`at`, `grow`, `colour_row`, `next_colour`), the undo stack (`snapshot`, `undo`), and the router `handle_key`.
- `command_mode.rs` owns command mode's vocabulary: the `Command` enum, `parse`, `min_depth`, `is_undoable`, `enter_insert`, the fourteen command functions, and `reduce`.
- `insert_mode.rs` owns insert mode's vocabulary: a `Command` enum with `Commit`, `Backspace` and `Append(char)`, a `parse`, a `reduce`, and `drop_last_chars`.
- Each mode module exposes the same pair — `parse(key: &str) -> Option<Command>` and `reduce(state: State, command: Command) -> State`. The two `Command` types are distinct and namespaced by their module; neither is re-exported.
- `handle_key` becomes a router with no behaviour of its own: it matches on `state.mode`, calls that mode's `parse`, and dispatches to that mode's `reduce`. An unrecognised key returns the state untouched, in either mode.
- `min_depth` and `is_undoable` live in `command_mode.rs` and are defined over `command_mode::Command` only. Insert-mode commands have no depth requirement and are not individually undoable.
- `state.mode` and the `Mode` enum become `pub(crate)`; `Mode`'s variants are constructed from `command_mode.rs`. `history` stays private to `state.rs` — `command_mode::reduce` reaches the undo stack only through `snapshot` and `undo`.
- Behaviour is unchanged. Every key does exactly what it does today, including the `PAD` trailing space, the unconditional cursor placement in `layout::with_cursor`, and the existing undoable set.
- `layout.rs`, `render.rs` are not modified. `writer.rs` changes only its import line.
- Every existing test is preserved and moves to the module that owns the code it exercises. No assertion is weakened or deleted.
- A `#[cfg(test)] pub(crate) fn new_state(boxes: Vec<Node>, mode: Mode, selected: Vec<usize>) -> State` in `state.rs` replaces the test-local helper, so both mode modules' tests can build a `State` without `history` becoming public.

## Technical Design

### Why modes, and not state-versus-commands

The first cut considered was `state.rs` (the nouns) and `commands.rs` (the verbs). Tracing the next few stories through it — delete a node, delete a row, colour a row's background — showed it holding up well: each is a `Command` variant, a `parse` row, a `min_depth` row, an `is_undoable` row and a mutator, all landing in one file while `Document` never moves.

What broke it was the definition of "command". If `commands.rs` means *everything that mutates*, it swallows `at`, `grow`, `colour_row` and `next_colour` — tree surgery and palette arithmetic, which are the vocabulary commands are written in, not commands themselves. If instead it means *what the user can name*, then those primitives fall out, and so does typing a character: no user says "I pressed the insert-a-character command."

Taking the user's point of view seriously leads somewhere more specific. `b` means "new box" in command mode and "the letter b" in insert mode. The same keystroke is a different command depending on the mode, because **the mode is the interpreter**. There is no single command vocabulary to put in a single file; there is one per mode.

So a mode is three things bound together:

1. **A vocabulary** — which keys mean something, and what.
2. **A state** — what it must remember while it is active.
3. **Its exits** — which of its commands hand control to another mode.

What distinguishes a mode from a mere command set is that it is a *total* interpretation of the keyboard: it decides what every key means, including "nothing", and exactly one mode is active at a time. That is why `parse` returns `Option` and why it must be per-mode — "unrecognised" is a different answer in command mode than in insert mode.

| mode | vocabulary | own state | exits to |
|---|---|---|---|
| Command | fourteen named verbs, each with a depth requirement and an undoability | none | Insert, via `NewBox`, `NewSibling`, `EditLabel`, `RenameLabel` |
| Insert | `Commit`, `Backspace`, `Append(char)` | where the text cursor is | Command, via `Commit` |

This spec implements column one — the vocabularies — and moves each into its own module. Column two, the per-mode state, is spec 056.

### The three modules

**`state.rs`** keeps what both modes share and neither owns:

- Types: `Node`, `Document`, `State`, `Mode`, with their `Default` impls.
- Constants: `PLAIN`, `PALETTE_SIZE`, `PAD`.
- Tree and palette primitives: `at`, `grow`, `colour_row`, `next_colour`.
- The undo stack: `snapshot`, `undo`.
- The router: `handle_key`.

**`command_mode.rs`** owns command mode's vocabulary: `Command`, `parse`, `min_depth`, `is_undoable`, `reduce`, `enter_insert`, and the fourteen command functions `new_box` through `quit`.

`enter_insert` moves here rather than staying in `state.rs`, because it is command mode's *exit* — the transition belongs to the mode being left. It is used only by `edit_label` and `rename_label`.

**`insert_mode.rs`** owns insert mode's vocabulary: `Command`, `parse`, `reduce`, and `drop_last_chars`.

### A symmetric interface

Both mode modules expose the same pair:

```rust
pub(crate) fn parse(key: &str) -> Option<Command>;
pub(crate) fn reduce(state: State, command: Command) -> State;
```

The two `Command` types are unrelated and are never re-exported, so `command_mode::Command` and `insert_mode::Command` stay distinct at the type level. A command-mode variant cannot be handed to insert mode's reducer; the compiler rejects it. The shared name is the point — it says these are the same kind of thing in different modes.

This makes `handle_key` a router with no logic of its own:

```rust
pub(crate) fn handle_key(state: State, key: &str) -> State {
    match state.mode {
        Mode::Command => match command_mode::parse(key) {
            Some(command) => command_mode::reduce(state, command),
            None => state,
        },
        Mode::Insert => match insert_mode::parse(key) {
            Some(command) => insert_mode::reduce(state, command),
            None => state,
        },
    }
}
```

Adding a mode is then one arm, one module, and no change to any existing mode.

### Insert mode gains a vocabulary

This is the one addition beyond a pure move, and it is what makes `insert_mode.rs` a mode module rather than a single function in a file. Today `insert` decodes and acts in one pass:

```rust
if key == "\x1b" { ...leave insert mode... }
if key == "\x7f" { ...delete a character... }
if key >= "\x20" && key <= "\x7e" { ...append a character... }
```

Split along the same seam spec 054 drew for command mode — decode separately from act:

```rust
pub(crate) enum Command {
    Commit,
    Backspace,
    Append(char),
}

pub(crate) fn parse(key: &str) -> Option<Command> {
    match key {
        "\x1b" => Some(Command::Commit),
        "\x7f" => Some(Command::Backspace),
        _ => match key.chars().next() {
            Some(c) if ('\x20'..='\x7e').contains(&c) => Some(Command::Append(c)),
            _ => None,
        },
    }
}
```

`parse` becomes independently testable without constructing a `State`, exactly as `command_mode::parse` already is. The printable test moves from string comparison to a `char` range, which is equivalent for the single-byte keys the reader produces but states the intent directly.

`reduce` keeps today's three behaviours verbatim, `PAD` included:

```rust
pub(crate) fn reduce(mut state: State, command: Command) -> State {
    // Insert mode is only entered by NewBox, NewSibling, EditLabel or RenameLabel,
    // all of which guarantee a selection.
    let node = at(&mut state.doc.boxes, &state.doc.selected);
    let label = node.label.clone();
    match command {
        Command::Commit => {
            node.label = drop_last_chars(&label, 1);
            state.mode = Mode::Command;
        }
        Command::Backspace => node.label = format!("{}{PAD}", drop_last_chars(&label, 2)),
        Command::Append(c) => node.label = format!("{}{c}{PAD}", drop_last_chars(&label, 1)),
    }
    state
}
```

The `PAD` arithmetic is untouched here deliberately. It is load-bearing: `layout::with_cursor` places the text cursor at `placement.x + placement.width - 1`, the last cell of the selected label, so the trailing space is what reserves a cell for the cursor to occupy. Removing it means giving the cursor a home in `Mode::Insert` and making `with_cursor` conditional, which touches `layout.rs` and `render.rs`. That is spec 056.

### Guards stay with command mode

`min_depth` and `is_undoable` are defined over `command_mode::Command` and live in `command_mode.rs`. They are not general command properties — they are command mode's. Insert-mode commands have no depth requirement (insert mode is only reachable with a selection), and none of them is individually undoable: a whole insert session is undone by the `RenameLabel` or `NewBox` snapshot that opened it.

`reduce` keeps the shape spec 054 gave it — one guard check, one snapshot decision, one dispatch:

```rust
pub(crate) fn reduce(state: State, command: Command) -> State {
    if state.doc.selected.len() < min_depth(command) {
        return state;
    }
    let state = if is_undoable(command) { snapshot(state) } else { state };
    match command { /* one arm per command */ }
}
```

### Visibility

`state.rs` widens to `pub(crate)`: `Document`, `State`, `Mode` and its variants, `PALETTE_SIZE`, `PAD`, `at`, `grow`, `colour_row`, `next_colour`, `snapshot`, `undo`, and the `State.mode` field. `Node` and `PLAIN` already are.

`history` stays private. `snapshot` and `undo` remain in `state.rs`, and `command_mode::reduce` calls them rather than touching the field, so the undo stack keeps its invariant — it is a stack you push and pop, not a `Vec` anyone can rewrite. This is the one piece of encapsulation the split preserves for free.

Spec 054 recorded that `mode` is private because "nothing outside `state.rs` reads it". That rationale was about `writer.rs`, which still does not read it. Widening it to `pub(crate)` is forced by `command_mode` needing to construct `Mode::Insert`, and is the price of organising by mode. Sealing `Mode` behind a setter function was considered and rejected: it buys a compile-time guard inside a single four-file binary crate, at the cost of an indirection on every mode transition, and spec 056 will move mode transitions into the `Mode` type itself where the invariant can be expressed directly.

### Module cycle

`state.rs` calls into `command_mode` and `insert_mode`, and both call back into `state.rs`. Rust permits this — modules are namespaces within one crate, not separate compilation units, so there is no ordering constraint and no forward declaration.

It is worth naming, though, because it is the cost of keeping `handle_key` in `state.rs`. The alternative is a fourth module owning the router, which was considered and rejected as a file for nine lines. If a third mode arrives with `Prompt` and the router grows a mode-transition table, that judgement is worth revisiting.

### Test impact

The suite is the safety net for this move; every assertion is preserved and travels with the code it exercises.

- **`state.rs`**: `Node` defaults and equality, the three `at` tests, `grow`, `colour_row`, `next_colour`, `snapshot`/`undo` including multi-level undo, and `handle_key` routing — that an unrecognised key is a no-op in either mode, and that the mode decides which vocabulary applies.
- **`command_mode.rs`**: `parse`, `min_depth` (the `COMMANDS` fixture and the below-minimum-depth test), `is_undoable`, and the per-command tests.
- **`insert_mode.rs`**: `parse` for the three key classes and their boundaries, plus typing, backspace and escape behaviour.

The helper at `state.rs:510` builds a `State` literal, which works today only because the tests are a child module of `state.rs`. With `history` staying private, tests in the mode modules cannot construct one — functional update syntax (`..Default::default()`) does not help, since Rust treats it as construction and rejects it while any field is private.

So the helper moves to `state.rs` as a crate-visible test fixture:

```rust
#[cfg(test)]
pub(crate) fn new_state(boxes: Vec<Node>, mode: Mode, selected: Vec<usize>) -> State {
    State { doc: Document { boxes, selected }, history: Vec::new(), mode, running: true }
}
```

`#[cfg(test)]` is active for the whole crate under `cargo test`, so both mode modules' tests can call it. Its twenty-four call sites change only their path.

### Out of scope

Per-mode state — `Mode` as a data-carrying enum, the text cursor moving into `Mode::Insert`, deleting `PAD`, making `with_cursor` conditional, and splitting `Document` into `Diagram { boxes }` and `Snapshot { diagram, selected }` so that saving writes a diagram and not a cursor position — is **spec 056**. It touches `layout.rs` and `render.rs`; this spec touches neither.

Multi-line labels are planned but are not this spec, and barely touch it: insert mode's `Command` gains a `Newline` variant alongside `Append(char)`, and nothing else in the vocabulary changes. They do constrain one 056 decision, recorded here so it is not made by accident — the text cursor should be stored as a flat character index into the label, not as a column. A flat index is identical to a column for today's single-line labels and keeps working once labels contain newlines. The layout consequences are a separate story again: `layout::height` already takes a `&Node` and ignores it (`layout.rs:48`, currently `#[allow(dead_code)]`), and `layout::interior` counts every character in the label, so a newline would count toward the box's *width*.

Save and open are a later story still, and carry an open question this spec does not prejudge: every command today is a pure `State -> State`, and `writer::run` can only paint a frame and read a key. Writing a file needs either an effect returned as data, a pending-effect field on `State`, or an injected storage dependency. Choosing that is the save story's job.
