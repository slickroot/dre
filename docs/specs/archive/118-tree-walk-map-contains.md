# Tree walk, map and contains

Step of spec 117 (one-direction architecture). This is a technical spec, not a
user story: it has no user-facing behaviour.

## Problem

Readers of the drawing (layout, the file format, the renderers) need to visit every box, convert a tree to another type, and check that a path exists. `Tree` has none of these, so callers would walk children by hand or need `Node` exposed.

## Acceptance Criteria

- `tree.walk()` yields `(path, &T)` in pre-order and never yields `[]`.
- `tree.map(f)` returns a `Tree<U>` of the same shape, with `f` applied to every value. The root value is `U::default()` and `f` never sees it.
- `tree.contains(&path)` is true when the path addresses an existing box. `[]` is not a box.
- All three are public in `types::Tree`. No other module changes.
- The unit tests in `types/src/tree.rs` are the specification.

## Technical Design

Interface is fixed in spec 117 (`types::Tree` additions). All changes are in
`types/src/tree.rs`. Existing methods keep their behaviour.

### Decisions

- **`walk` is eager.** A private recursive helper pushes every
  `(Vec<usize>, &T)` into a `Vec` in pre-order, skipping the root, and `walk`
  returns `vec.into_iter()`. Each item already allocates its path, and layout
  walks the whole tree, so a lazy iterator would save little. The return type
  `impl Iterator` lets us go lazy later without touching callers.
- **`map` uses a private helper.** `map_box(&self, f: &impl Fn(&T) -> U) ->
  Tree<U>` applies `f` to a box's own value and recurses into its children.
  `map` returns `Tree::root(children mapped with map_box)`, so the root is
  `U::default()` and `f` never sees the root's value.
- **`contains` has its own loop.** A private `try_get(&self, path) ->
  Option<&Tree<T>>` walks the path with `children.get(index)` and returns
  `None` on the first miss. `contains` is `!path.is_empty() &&
  self.try_get(path).is_some()`. It never panics. `position` and `get` are not
  changed to use `try_get`. Doing so can be a later cleanup.

### Tests

Written first, in `types/src/tree.rs`, on the existing `sample()` tree (root,
`a` with `a0` and `a1`, and `b`).

`walk`
1. Yields `[0]`, `[0,0]`, `[0,1]`, `[1]` in pre-order, with values `a`, `a0`,
   `a1`, `b`.
2. Never yields `[]`, checked on `sample()`.
3. Yields nothing for a root with no children.
4. Reaches a deep chain (three levels) with the full path for each box.

`map`
5. Keeps the shape: same paths and children counts, checked with `walk` on both
   trees.
6. Applies `f` to every value, for example `"a1"` becomes `2` with `len`.
7. The root becomes `U::default()`, and `f` never sees the root's value. A
   closure that panics on `""` proves it, since the root's value is `""`.
8. The empty root maps to an empty root.

`contains`
9. True for top-level, nested and childless boxes.
10. False for `[]`.
11. False for an index past the last sibling (`[2]`, `[0, 2]`).
12. False, without panicking, for a path through a box with too few children
    (`[0, 2, 0]`) and for a path below a leaf (`[1, 0]`).
