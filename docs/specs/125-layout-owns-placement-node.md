# layout owns PlacementNode

Step of spec 117 (one-direction architecture). This is a technical spec, not a
user story: it has no user-facing behaviour.

## Problem

`layout` takes `&[Node]` and its `PlacementNode::Node` borrows a `&Node`, so the renderers read the domain type, including the `hint` flag.

## Acceptance Criteria

- `layout(&Document) -> Vec<Placement>`. `Node` appears only inside `layout`, through its getters.
- `PlacementNode` is a new type that knows nothing about `Node`. Its box variant carries `colour`, `filled`, `rounded` and `hint`.
- `Placement` has no lifetime parameter.
- `Node` has no `hint` field (left in place by spec 123).
- `layout::placeholder(text)` produces the faded placeholder box for an empty drawing, and the renderer no longer builds a `Node`.
- The output stays a flat list, boxes first. No visible change.

## Technical Design

Shape is fixed in spec 117 (`layout`). Type and field names are still to be designed before implementation.
