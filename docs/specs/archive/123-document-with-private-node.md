# Document with a private Node

Step of spec 117 (one-direction architecture). This is a technical spec, not a
user story: it has no user-facing behaviour, except the undo change spec 117
allows (undo no longer restores the selection).

## Problem

`Node` and `Document` have public fields, and `Node` holds its own `children`, so every module walks and edits the tree by hand. Paths are a custom `Path { ancestors, index }` struct instead of `Tree`'s `&[usize]`, and the selection sits inside `Document`.

## Acceptance Criteria

- `Document` is `{ root: Tree<Node> }` with an invisible root.
- `Node` has `label`, `colour`, `filled`, `rounded` and `hint`. No `children`.
- `Document::tree()` returns `&Tree<Node>` for reads.
- `Path` is gone. Paths are `Vec<usize>` / `&[usize]` everywhere.
- The selection lives in `State`, not in `Document`. Undo restores the drawing
  only. If the selection no longer addresses a box after undo, it moves to the
  nearest ancestor that still exists, or to nothing.
- `file_document.rs` moves into `diagram.rs`.

## Technical Design

This step changes the representation only. Closing off writes is left to
later steps.

### What stays open, and who closes it

| Temporary in 123 | Closed by |
| --- | --- |
| `Node` fields stay `pub(crate)` | 124 (edits are `Document` methods) |
| `Document.root` is `pub(crate)`, so call sites can use `push`/`remove`/`value_mut` directly | 124 |
| `Node.hint` stays, set by the renderer's placeholder and read by `layout` | 125 (`layout::placeholder(text)`) |
| `diagram.rs` depends on `FileBox` | later, as in spec 117 |

### `diagram.rs`

- `Document { pub(crate) root: Tree<Node> }`, built with `Tree::root(children)`.
  The top-level boxes are the root's children.
- `Document::tree(&self) -> &Tree<Node>` is the read path for `layout`, the
  renderers and the store.
- `Node { label, colour, filled, rounded, hint }`, all `pub(crate)`.
- `children_at`, `at`, `append` and the free `remove` are deleted in step 4.
  Call sites use `Tree` instead.
- The test helpers `node` / `node_with_children` become builders of
  `Tree<Node>` (`Tree::leaf`, `Tree::new`).

### The selection and undo

- `State.selected: Option<Vec<usize>>`. `Document` has no selection.
- `history: Vec<Document>` holds the drawing only. The "did anything change?"
  check compares only the `Document`, so moving the cursor never costs an undo
  step.
- The fallback runs once, in `undo`: while the path is non-empty and
  `!tree.contains(path)`, drop its last index. An empty path means `None`. It
  cuts the slice itself rather than calling `Tree::parent`, because `parent`
  expects a path that exists. The "first top-level box" step from spec 117 is
  dropped.
- Tests that assert undo restores the selection (for example
  `state/command.rs` around line 1569) are rewritten to the new rule. New
  tests cover: the box still exists (selection kept), the selected box was
  created by the undone edit (parent selected), a top-level box undone
  (nothing selected).

### Migration order

Each step is its own commit and leaves the build and tests green.

1. **Wire up and move.** `dre` depends on the `types` crate.
   `file_document.rs` moves into `diagram.rs`. No behaviour change.
2. **Selection into `State`.** `Document.selected` becomes
   `State.selected: Option<Path>`. History snapshots hold only the drawing.
   Undo gets the fallback. This is the only behaviour change, isolated here.
3. **`Path` becomes `Vec<usize>`.** Still on `Vec<Node>`: `children_at`, `at`
   and `remove` take `&[usize]`. This is the big mechanical rewrite (about 240
   uses, mostly `state/command.rs`), done while the tree shape stays the same.
4. **`Vec<Node>` becomes `Tree<Node>`.** `Document { root }`, `children`
   removed from `Node`, the free functions deleted. Call sites use `Tree`'s
   `push`, `remove`, `value`, `value_mut`, `walk` and navigation. The file
   conversion builds and reads `Tree<Node>`.

Step 3 comes before step 4 so that by the time the storage changes, every
call site already uses `Tree`'s path language. Step 4 then only changes how
the tree is stored and walked.
