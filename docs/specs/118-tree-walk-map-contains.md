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

Interface is fixed in spec 117 (`types::Tree` additions). The test list is still to be written before implementation.
