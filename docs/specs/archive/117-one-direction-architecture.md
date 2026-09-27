# One-direction architecture

## Problem

`State` is a struct that everyone edits. Its fields are all `pub(crate)`, and
they are touched from outside `state.rs` about 150 times: `doc` 108, `save_to`
14, `running` 8, `scroll_x` 7, `mode` and `colour_overlay` 6 each. The mode
modules decide how state changes, `editor.rs`, `lib.rs` and `cli.rs` write
fields directly, and `state.rs` imports the mode modules that import it back.
Spec 110 (every change is undoable) only had to be written because a mode could
change the drawing and forget to snapshot.

`diagram.rs` makes everything public, even `Node`. `Node` holds its own
`children`. `Document` holds the selection. `palette()` and the status line
builder live in modules they have nothing to do with. `Node.hint` is a
rendering detail of the empty-drawing placeholder stored on a domain type.

This is a technical spec, not a user story: it has no user-facing behaviour.
It is the umbrella design. The steps listed at the end are separate specs, each
small enough to merge alone with the build green.

## Acceptance Criteria

- Only `state::reduce` changes a `State`. No other module assigns to a field of
  `State`, and none can, because the fields are private.
- Only `diagram.rs` reads a `Node`'s fields or changes a `Document`. Everything
  else uses getters and named methods.
- Dependencies run one way and have no cycles:
  `input` → `state` → `diagram` → `types::Tree`, and `layout` → `diagram`,
  `render` → `layout`. `state` does not import the mode modules or `input`.
- Behaviour is unchanged for the user, except one deliberate change: undo no
  longer restores the selection (see Undo and the selection).
- Each step below is its own spec. This spec is done when the last of them is
  merged.

## Technical Design

### The rule

One direction: **key → `Action` → `reduce(State, Action) -> State` → view.**
The reducer is the only writer. Everything else reads.

### Modules and what each one is

| Module | Is | Knows about |
|---|---|---|
| `types::Tree<T>` | the structure: values, children, paths (`Vec<usize>`), navigation | nothing |
| `diagram.rs` | the domain: `Node`, `Document`, and every edit and query on a drawing | `Tree` |
| `palette.rs` | the app's colours: `PALETTE`, `FOREGROUND`, `BACKGROUND`, `palette()` | nothing |
| `state/` | the editing session and its one writer, `reduce` | `diagram` |
| `input.rs` | keys to `Action`s: keymaps, `parse`, dispatch by `Mode`. Read-only on `State` | `state` (read), `Action` |
| `status_line.rs` | presentation: `StatusLine`, `Segment`, `Style`, `StatusInput`, `status_line(&StatusInput)` | `palette`. Never `State` |
| `layout` | the drawing to placements: `layout(&Document) -> Vec<Placement>` | `diagram` (getters), `Tree::walk` |
| `render/` | placements to pixels and SVG | `layout`, `palette`. Never `Node` or `Document` |
| `editor.rs` | the loop and the I/O: read a key, `input`, `reduce`, render, save on exit | all of the above |

### `types::Tree` additions

- `walk(&self) -> impl Iterator<Item = (Vec<usize>, &T)>`: pre-order, never
  yields `[]`, the same rule the navigation methods follow.
- `map<U: Default>(&self, f: impl Fn(&T) -> U) -> Tree<U>`: same shape, `f` on
  every value. The invisible root becomes `U::default()`, so `f` never sees it.
- `contains(&self, path: &[usize]) -> bool`: does the path address a box.

### `diagram.rs`

- `Document { root: Tree<Node> }`, private field, invisible root. The top-level
  boxes are the root's children, so `Tree`'s navigation, `push`, `remove` and
  `walk` work on the whole drawing. Not `Vec<Tree<Node>>`: that would leave the
  top level outside `Tree`'s rules.
- `Node` holds one box's own properties: `label`, `colour`, `filled`,
  `rounded`. No `children`, no `hint`. Fields are private. It is a public type
  with read methods only: `label()`, `colour()`, `filled()`, `rounded()`. It has
  no setters and no public constructor.
- `Document` exposes `tree(&self) -> &Tree<Node>` for reads (`walk`, `map`,
  navigation, `contains`) and named edit methods for writes: `set_label`,
  `set_colour`, `cycle_colour`, `toggle_fill`, `toggle_rounded`, `add_child`,
  `add_sibling`, `remove`, `paste`, and so on, each taking a `&[usize]` path.
  Nobody outside gets a `&mut Node` or a `&mut Tree<Node>`.
- `file_document.rs` moves into `diagram.rs` for now, so the conversion between
  `Document` and the file format can build a `Node` directly. This is a known
  coupling: `diagram.rs` depends on `FileBox`. It is to be undone later, not
  the target.
- The drawing helpers in `state.rs` (`colour_row`, `add_child_box`,
  `blank_box`, `next_colour`) move here as `Document` methods.
- The clipboard is `Option<Tree<Node>>`. `Document::remove` returns the
  detached subtree, and `Document::paste(&parent, &subtree)` puts a clone back.

### `hint`

`Node.hint` goes away. Only the renderer's `hint_node()` ever set it, for the
faded placeholder shown when the drawing is empty. It is a fact about a
placement, not about a box. `layout` gets its own entry point for that case,
`layout::placeholder(text)`, and `hint` lives only on layout's output.

### `layout`

- `layout(&Document) -> Vec<Placement>`. `Node` appears only inside `layout`,
  through its getters. It reads the drawing with `walk` and `map`.
- `PlacementNode` is a layout-owned type that knows nothing about `Node`. Its
  box variant carries plain data (`colour`, `filled`, `rounded`, `hint`).
  `Placement` no longer borrows from the `Document`, so its `'a` goes away.
- Output stays a flat, draw-ordered list: boxes first, then labels, arrows and
  the cursor.

### `state/`

```
state/
  mod.rs          State (private fields), Mode, Action, reduce  ← the only writer
  command.rs      the Command-mode reducer
  insert.rs       the Insert-mode reducer
  save_prompt.rs  the Save-prompt reducer
  history.rs      snapshot, undo, drop-if-unchanged, the selection fallback
```

- The reducers are child modules of `state`, so they can see its private fields
  and nobody else can. Outside `state/`, `State` has getters and `reduce`.
- `reduce` takes the undo snapshot itself, from the `is_undoable` rule that
  `command_mode` has today. A mode can no longer change the drawing and forget
  to make it undoable.
- Everything that writes a field by hand today becomes an `Action`. That
  includes `editor.rs` (`save_to = None`, `running`), `lib.rs`, `cli.rs` and the
  colour-overlay branch in `handle_key`, which writes into the drawing directly.
- Tests that set a field to build a scenario use a test constructor in
  `state/`, not field assignment.
- The reducers stay pure. Saving stays in `editor.rs`, after the loop: the
  reducer sets `running = false` and `editor.rs` writes the file.

### `input.rs`

- `parse(&State, key) -> Option<Action>`, dispatching on `state.mode()`. It
  reads `State` (mode, pending count, colour overlay) and never writes it.
- The keymaps and each mode's `parse` move here from `command_mode.rs`,
  `insert_mode.rs` and `save_prompt_mode.rs`. Only the `reduce` halves stay,
  inside `state/`. The digit-count and colour-overlay branches of `handle_key`
  become Command-mode actions.
- The idle-cursor restore (`last_selected`) becomes part of `reduce`.

### Selection

The selection is not part of `Document`. `State` holds it as
`Option<Vec<usize>>`, next to `mode`, `clipboard` and `scroll_x`. The caller
works out where the selection will be before an edit, from `Tree` navigation,
as spec 114 decided.

### Undo and the selection

- `history` is a `Vec<Document>`. A snapshot holds the drawing only, not the
  selection. This is a deliberate change: undo used to restore the selection.
- The "did anything change?" check compares only the `Document`, so moving the
  cursor never costs an undo step.
- After undo, if the selection no longer addresses a box, it falls back to the
  nearest ancestor that still exists (dropping the last index), then to
  `None` if no ancestor exists. The check uses
  `Tree::contains`. It runs once, in `undo`, so everything else can assume the
  selection is valid.

### `status_line.rs` and `palette.rs`

- `status_line.rs` holds `StatusLine`, `Segment`, `Style`, `ModeLabel`,
  `StatusInput` and `status_line(&StatusInput)`, moved out of `state.rs`. It is
  presentation and never imports `State`. `state.rs` builds the `StatusInput`
  (see spec 120).
- `palette.rs` holds `PALETTE`, `FOREGROUND`, `BACKGROUND` and `palette()`.
  `Node.colour` stays an `Option<u8>`, an index into it, so `diagram.rs` does
  not import it.

### Considered and rejected

- `Document { boxes: Vec<Tree<Node>> }`: the top level would sit outside
  `Tree`'s sibling and stay-put rules.
- `Node` fields public, or a public `Node` builder: any caller could then build
  or change boxes without going through `Document`.
- A view type for reads (`BoxView`): it would be `Node` with a new name.
- `Document` as a type alias for `Tree<Node>`: `push` and `remove` would be
  public on the drawing.
- Named transitions on `State` (`state.quit()`, `state.scroll_by()`): still many
  writers, only behind method names. The rule is one writer, `reduce`.
- A flat `reduce` with no child modules: one very large file, and no split by
  mode.
- Snapshotting the selection for undo: rejected by decision. The fallback above
  covers the invalid selection it leaves behind.
- `walk()` on `Document`: `walk` and `map` belong to `Tree` and work on any
  tree.

### Steps

Each step is its own spec and lands with the build green. The order is
deliberate: extractions first, then the `Action` seam, then the data types.

1. Spec 118: `Tree` additions: `walk`, `map`, `contains`.
2. Spec 119: extract `palette.rs`.
3. Spec 120: extract `status_line.rs`.
4. Spec 121: `Action` and `input.rs`. Key parsing moves out of the mode modules.
5. Spec 122: `state/` with private fields and the single `reduce`.
6. Spec 123: `Document { root: Tree<Node> }`, private `Node` with getters, `hint`
   removed, `file_document.rs` into `diagram.rs`.
7. Spec 124: edits move onto `Document` methods and the reducers call them.
8. Spec 125: `layout(&Document)` with its own `PlacementNode`.
9. Spec 130: `State`'s fields become private, with getters and test builders.

Steps 1 to 3 are mechanical and can share one short design session, or go
straight to implementation. Steps 4 to 8 each get a short design session right
before they are implemented, so each design uses what the earlier steps showed.
