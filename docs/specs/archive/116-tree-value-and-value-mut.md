# Tree value and value_mut

## Problem

Specs 114 and 115 gave `Tree<T>` in `types/src/tree.rs` its constructors,
navigation, `push` and `remove`. The fields are private and `get` and `get_mut`
are private, so no caller can read or change the `T` stored in a box. Every
caller that moves onto `Tree` needs this: layout reads a box's label and
position, and the edit commands change them. This is a technical spec, not a
user story: it has no user-facing behaviour, and no other module changes yet.

## Acceptance Criteria

- `tree.value(&path)` returns `&T`, the value of the box at `path`.
- `tree.value_mut(&path)` returns `&mut T`, so a caller can replace the value (`*tree.value_mut(&p) = x`) or edit it in
  place (`tree.value_mut(&p).label.push('x')`).
- Both panic on `[]` and when `path` does not address an existing box.
- Both are public, in `types::Tree`. No dependency is added, and `dre` and `web`
  do not change.
- The unit tests in `types/src/tree.rs` are the specification of these methods.

## Technical Design

### Decisions

- Two methods, `value` and `value_mut`, one word each like the rest of the
  interface, and named for what they return: the `T`, not the `Tree`.
- `[]` panics. The invisible root's value is `T::default()`, a placeholder that
  no caller should read or edit, so asking for it is a caller bug. This is the
  same rule as `child`, `next`, `previous` and `remove`, and it means `value`
  needs no `T: Default` bound.
- A path that addresses no box panics, as everywhere else in `Tree`.
- `value_mut` is public. Considered and rejected for now: keeping it private and
  exposing `get(&path) -> &T` and `set(&path, T)`. `set` replaces the whole
  value, so changing one field means clone, edit and `set` back. `value_mut`
  edits in place. Narrowing the interface can come later, when the callers show
  what they need.
- `get` and `get_mut` stay private and keep returning `&Tree<T>` and
  `&mut Tree<T>`. They are still the only functions that walk the tree. `value`
  and `value_mut` assert the path is not empty, then call them and return the
  `value` field.
- `value_mut` gives access to the `T` only. It cannot add, remove or reorder
  children, so `push` and `remove` stay the only structural edits.
- Neither method knows about the selection.

### Interface

```rust
impl<T> Tree<T> {
    pub fn value(&self, path: &[usize]) -> &T;
    pub fn value_mut(&mut self, path: &[usize]) -> &mut T;
}
```

### Test list

Each is a unit test in `types/src/tree.rs`, on `Tree::root(vec![...])` of `&str`
leaves.

- `value` returns the value of a top-level box.
- `value` returns the value of a nested box.
- `value` of a box with children returns its own value, not a child's.
- `value_mut` replaces the value of the box at a path and leaves every other
  box alone.
- `value_mut` edits the value in place (a `Tree<String>` with `push_str`).
- `value` and `value_mut` each panic on `[]`, on an index past the last sibling
  and on a path that goes through a box with too few children.
