# Split layout into view and tree modules

This is a technical refactoring spec. It changes the ownership and file
boundaries of layout code without changing the rendered diagram, footer,
selection glow, caret, or cursor behaviour.

## Problem

`src/layout.rs` is 1,223 lines and currently owns four different jobs:

- placement data types consumed by renderers;
- footer construction;
- view overlays and editing indicators;
- tree measurement and recursive tree placement.

The first three are view concerns. The last is tree geometry. Keeping them in
one module makes the renderer-facing API appear to be the same thing as the
tree layout algorithm and forces unrelated tests and callers through one file.

## Acceptance Criteria

- `src/layout.rs` is replaced by the directory module `src/layout/mod.rs` and
  `src/layout/tree.rs`.
- `view` is the sole owner and renderer-facing source of placement types and
  view composition helpers.
- Tree measurement and recursive placement live in `layout::tree`.
- Rendered output and all existing layout behaviour are unchanged.
- The full Rust test suite passes.

## Technical Design

### Module structure

Use the directory form of the Rust module:

```text
src/
  layout/
    mod.rs
    tree.rs
```

`src/layout/mod.rs` declares the crate-visible child module:

```rust
pub(crate) mod tree;
```

It contains no placement types, footer code, overlays, or tree implementation.
There are no layout-level re-exports of the tree API.

`view` calls `crate::layout::tree::diagram` and owns the scene-level
composition around it. `layout::tree` directly constructs and returns
`crate::view::Placement` values. The sibling-module dependency is supported
by Rust and avoids introducing an intermediate placement model solely for this
file split.

### `view` ownership

Move these definitions from `layout.rs` into `view.rs`, preserving their
fields, derives, names, and visibility:

- `Sides`, `ALL_SIDES`, and `NO_SIDES`;
- `Label`, `Arrow`, `Caret`, `Cursor`;
- `PlacementNode` and `Placement`;
- `BOX_HEIGHT`, `BORDER`, `GLOW_MARGIN`;
- `FOOTER_ROWS`, `LED_WIDTH`, and `LED_LABEL_GAP`;
- the footer builder `footer`;
- the overlay helpers `with_glow`, `with_cursor`, and `with_caret`.

Keep the existing `view` re-exports/API available to callers, but make the
definitions themselves live in `view`; renderers must import these symbols
from `crate::view` rather than `crate::layout`.

The existing `view::editor` and `view::body` functions continue to compose
scenes in the same order. `body` obtains diagram placements from
`layout::tree::diagram`, then applies the caret, centering, and selection glow
as it does today. `editor` obtains the footer from the view-owned footer
builder and keeps the existing right alignment.

### `layout::tree` ownership

Move the tree-specific implementation into `src/layout/tree.rs`:

- `interior`, `width`, `height`, and the label-centering helper;
- `edit_room`, `measure_columns`, and `place`;
- the tree `diagram` function;
- tree-only constants such as `GAP_HEIGHT`, `GAP_WIDTH`, `SIDE_PADDING`,
  `ROW_PITCH`, `HALF_PITCH`, and `LEAF_STRIDE`.

The tree module imports placement types and view-owned styling constants from
`crate::view` as needed. Its public-to-the-crate API is limited to the tree
entry point used by `view`; helpers and tree-only constants remain private
unless an existing renderer/test call site requires a deliberate
crate-visible interface. No behavior or placement ordering changes are
allowed during the move.

`diagram` continues to return an empty vector for a tree without a first node,
measures columns with editing-room expansion, places boxes/labels/arrows using
the existing recursive algorithm, and emits boxes before non-box placements.

### API migration

Update all renderer and test imports that currently use `crate::layout` for
placement data or view constants to use `crate::view`. In particular, migrate
the terminal and SVG renderers, render-module tests, and their placement test
helpers. Calls to tree layout go through `crate::layout::tree` from `view`.

Do not add compatibility re-exports from `layout` after the migration; the
purpose of the split is to make `view` the single renderer-facing ownership
boundary.

### Tests

Move tests with the implementation:

- placement, footer, and overlay tests move to `view.rs`;
- tree measurement, centering, recursive placement, arrow, and diagram tests
  move to `layout/tree.rs`;
- renderer tests remain in their renderer modules and update imports only.

Retain the existing assertions, including exact placement coordinates,
ordering, dimensions, paths, footer alignment, cursor/caret positions, glow
geometry, and empty-tree behavior. Add no new visible behavior. The complete
test suite is the final verification that the module split preserved output.
