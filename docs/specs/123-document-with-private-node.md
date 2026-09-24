# Document with a private Node

Step of spec 117 (one-direction architecture). This is a technical spec, not a
user story: it has no user-facing behaviour.

## Problem

`Node` and `Document` have public fields, and `Node` holds its own `children`, so every module walks and edits the tree by hand. `Node.hint` is a rendering flag on a domain type, and the selection sits inside `Document`.

## Acceptance Criteria

- `Document` is `{ root: Tree<Node> }` with a private field and an invisible root.
- `Node` has `label`, `colour`, `filled` and `rounded`, all private, with read methods only. No `children`, no `hint`.
- `Document::tree()` returns `&Tree<Node>` for reads.
- The selection lives in `State`, not in `Document`.
- `file_document.rs` moves into `diagram.rs`.
- The renderer's empty-drawing placeholder no longer needs a `Node`.

## Technical Design

Decisions are fixed in spec 117 (`diagram.rs`, `hint`, Selection). The migration order for the `Path` and `children` call sites is still to be designed before implementation.
