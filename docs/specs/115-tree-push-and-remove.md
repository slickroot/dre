# Tree push and remove

## Problem

Spec 114 gave `Tree<T>` in `src/tree.rs` its constructors and navigation, and
left every edit for later specs, one at a time. This is the first edit spec.
Adding a box and deleting a box are still done by hand on `.children` in
`diagram.rs` (`append` and `remove`). This is a technical spec, not a user
story: it has no user-facing behaviour, and no other module changes yet.
It also moves `Tree` out of the `dre` crate into a library crate of its own,
because it is generic and is not expected to change much after this.

## Acceptance Criteria

- A new workspace member `types/` (crate `types`) is added to `members` in the
  root `Cargo.toml`. `Tree` moves from `src/tree.rs` into it and is exported at the
  crate root as `types::Tree`.
  `src/tree.rs` is removed. Nothing in `dre` or `web` uses `Tree` yet, so no
  caller changes and none needs a `path` dependency until a later spec
  moves one onto it.
- `parent` stops being a free function. It becomes a method,
  `tree.parent(&path)`, like `child`, `next` and `previous`. It returns a path
  and returns the same path for a top-level box. It panics on `[]` and when
  `path` does not address an existing box.
- `tree.push(&parent, child)` adds `child` as the last child of the box at
  `parent` and returns the path of the new child.
- `parent` may be `[]`, which pushes a new top-level box under the invisible
  root.
- `tree.remove(&path)` removes the box at `path` together with its subtree and
  returns that subtree. Its later siblings move down by one.
- `push` panics when `parent` does not address an existing box.
- `remove` panics on `[]` and when `path` does not address an existing box.
- Neither method knows about the selection. The caller works out the new
  selection itself.
- The unit tests in `types/src/tree.rs` are the specification of these methods.

## Technical Design

### Decisions

- `Tree` becomes a library crate in the workspace, named `types`, in `types/`.
  It is a `path` dependency of the crates that use it, and it is not published.
  The crate boundary enforces the spec 114 rule that the fields of `Tree` are
  private. Publishing to crates.io is left open: it would need no code change,
  only a README, docs and a stable interface.
- `parent` moves onto `Tree` so that all of the interface is methods on one
  type and there is one rule for bad paths. This replaces the spec 114 decision
  that `parent` is a free function returning a sub-slice. The cost is that it
  needs a tree and now allocates a `Vec`, like the other three.
- `types` holds only `Tree` for now. `Node`, `Document` and the other diagram
  types stay in `dre`. They carry the diagram's meaning and change often, which
  is the opposite of what this crate is for. They can move in later specs, once
  their callers are on `Tree`.
- The crate has no dependencies. `Tree<T>` is generic and knows nothing about
  `Node`, the selection or the terminal.
- The methods are called `push` and `remove`, one word each, like `child`,
  `next` and `previous`. `push` is on the parent, so it takes the parent's path
  and the tree to add. It does not take an index: it always appends. Inserting
  at an index comes in its own spec when a caller needs it.
- `push` returns the path of the new child, `[..parent, len_before]`. The
  caller can select it without counting children itself. It follows the rule
  that `Tree` never knows the selection, but hands back what the caller needs to
  choose one.
- `remove` returns the removed subtree instead of dropping it. Cut and paste
  (spec 111) needs it, and a caller that only deletes ignores it.
- `remove` does not return a new selection. Before the edit, the caller picks
  it from `next`, `previous` and `parent`, as spec 114 decided.
- A path that does not address a box is a caller bug, so both methods panic, as
  the navigation functions do. `[]` is valid for `push` (the invisible root) and
  invalid for `remove` (the root cannot be removed).
- Both are built on the private `get_mut`. `push` walks to the parent and
  pushes on its children. `remove` walks to the path without its last step and
  removes that last step from its children. Neither adds a second tree walk.

### Interface

```rust
impl<T> Tree<T> {
    pub fn parent(&self, path: &[usize]) -> Vec<usize>;
    pub fn push(&mut self, parent: &[usize], child: Tree<T>) -> Vec<usize>;
    pub fn remove(&mut self, path: &[usize]) -> Tree<T>;
}
```

### Test list

The three `parent` tests replace the free-function `parent` tests from spec 114.

Each is a unit test in `types/src/tree.rs`, on `Tree::root(vec![...])` of `&str`
leaves. Because fields are private, tests check the result through `get`.

- `parent` of a nested path is the path without its last step.
- `parent` of a top-level path is the same path.
- `parent` panics on `[]` and on a path that addresses no box.
- `push` under a box adds the child last and leaves the other children alone.
- `push` returns the path of the new child.
- `push` under `[]` adds a top-level box.
- `push` under a leaf gives it its first child, and `child` now moves into it.
- `push` keeps the pushed subtree's own children.
- `push` panics on a parent path that addresses no box.
- `remove` returns the removed box, with its subtree.
- `remove` takes the box out and later siblings move down by one.
- `remove` at any depth leaves the other branches alone.
- `remove` of the only child leaves its parent with no children.
- `remove` panics on `[]`, on an index past the last sibling and on a path that
  goes through a box with too few children.
