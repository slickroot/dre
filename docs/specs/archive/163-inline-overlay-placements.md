# Overlays are built where their placement is, not found by search

This is a technical refactoring spec. It changes how the selection glow and
the edit caret are constructed. It does not change rendered output.

## Problem

`with_glow` and `with_caret` (`src/layout.rs`) each take a fully-built
`Vec<Placement>` and search it for the `Label` whose `path` matches the
selected/editing path, then derive the overlay's geometry from whatever
`Placement` they find nearby. This is a second full pass over data that was
just built, to recover context (which node, and its already-known box
rectangle) that `place`'s `emit` had in hand a moment earlier and threw away.

The footer's `path: vec![]` sentinel (spec 160, already in a PR) is one
symptom of the same shape: `with_cursor` needs *something* to search for, so
a placement that isn't a tree node gets a fake tree path. This spec doesn't
touch the footer or `with_cursor` — 160 already removes that search for the
footer by emitting its cursor inline while building the footer's columns.
This spec does the equivalent for the diagram side: `with_glow` and
`with_caret`, which 160 explicitly leaves untouched ("`with_cursor` itself is
untouched and keeps serving the diagram, where `path` really does identify a
tree node").

Once both this spec and 160 have landed, nothing reads `Label::path` any
more, so it comes out too.

Note: spec 161 (file split of `layout.rs` into `view`/`layout::tree`) assumes
`with_glow`, `with_cursor`, and `with_caret` continue to exist and only move
files. After this spec, `with_glow` and `with_caret` no longer exist, so 161
needs to move `with_cursor` only (once 160 lands) and drop the other two from
its list.

## Acceptance Criteria

- `with_glow` and `with_caret` no longer exist. The selection glow and the
  edit caret are built inline, in `emit`, while it builds the `Box` and
  `Label` placements for the node they belong to.
- `Label` no longer has a `path` field (once this spec and 160 have both
  landed — this spec removes the diagram-side reader; 160 removes the
  footer-side reader).
- `diagram`'s public signature collapses `editing: Option<&[usize]>` and the
  separately-threaded caret index into one `editing: Option<(&[usize],
  usize)>`. `view::body` no longer calls `with_caret` or `with_glow`;
  `layout::diagram` alone returns the fully-finished placement list.
- `diagram` no longer reorders its output into "boxes first, then the rest"
  after building it — both renderers already dispatch strictly by
  `PlacementNode` variant (SVG's `paint` buckets into per-variant strings;
  the terminal's `paint` assigns `BOX_Z`/`GLOW_Z`/`CONTENT_Z`/`INK_Z` per
  variant), so list order has never driven render order.
- Rendered output (terminal and SVG), including glow and caret geometry,
  colour, and timing, is unchanged. The full Rust test suite passes.

## Technical Design

### `edit_room` and `is_selected` take the same context `emit` gets

`edit_room(path: &[usize], editing: Option<(&[usize], usize)>) -> i64`
compares `editing.map(|(p, _)| p)` against `path`, same as today except for
the tuple. `is_selected` is unchanged (`selected: Option<&[usize]>` was never
tied to the caret index).

`measure_columns` destructures only the path half of `editing` for its width
calculation; it doesn't need the index.

### Glow is built from the box `emit` is already constructing

`emit` already knows, for the node it's building, `x`, `y`, `width`,
`BOX_HEIGHT`, and `selected: bool`. When `selected` is true, `emit` pushes a
`PlacementNode::Glow` placement right after the `Box` placement, using that
same rect plus `GLOW_MARGIN` — the exact geometry `with_glow` used to recover
by scanning for a `Box` whose bounds contained the selected `Label`'s
position. No search, no `Label::path` comparison.

### Caret is built from the label `emit` is already constructing

`emit` gains the caret index via its `editing: Option<(&[usize], usize)>`
parameter (passed down through `visit`). When `editing`'s path matches the
node's `path`, `emit` pushes a `PlacementNode::Caret` placement right after
the `Label` placement, at `start + index` (the same `x` calculation
`with_caret` used to do after finding the label by path search), `y` = the
label's `middle`. This is the same geometry, computed once, where the label
itself is computed, instead of recovered afterward by search.

### `diagram`'s signature

```rust
pub(crate) fn diagram<'a>(
    tree: &'a Tree<Node>,
    editing: Option<(&[usize], usize)>,
    selected: Option<&[usize]>,
) -> Vec<Placement<'a>>
```

`diagram` calls `measure_columns` and `place` with this same `editing`, and
returns `place`'s output directly — no `boxes_first`/`rest` partition step.

`view::body` becomes:

```rust
pub fn body(state: &State, area: Area) -> Scene<'_> {
    let editing = match state.mode() {
        Mode::Insert { cursor } => state.selected().map(|path| (path, *cursor)),
        _ => None,
    };
    vec![(
        area,
        centre(
            layout::diagram(state.doc().tree(), editing, state.selected()),
            area,
        ),
    )]
}
```

No `with_caret`/`with_glow` calls remain in `view.rs`.

### `Label` loses `path`

`Label { text: Cow<'a, str> }` (plus whatever spec 160 adds, e.g. `colour`).
`emit` stops passing `path.to_vec()` into the `Label` it builds. Every test
that constructs a `Label` or asserts on `label.path` is updated to drop the
field.

### Tests

- Replace `with_glow`/`with_caret` unit tests (which called them as
  standalone functions on a pre-built placement vec) with assertions against
  `diagram`'s/`place`'s output directly: a selected node's placements include
  a `Glow` at the expected margin-expanded rect; an edited node's placements
  include a `Caret` at the expected `x`/`y` for each typed index; an
  unselected/non-edited node's placements include neither.
- Keep existing coverage: glow geometry (margin, colour, rounded flag caret
  x advancing with typed index and staying within the widened box, caret
  absence outside insert mode, glow absence without a selection.
- Add/keep a test that `diagram`'s output order is boxes-then-arrows-etc. in
  natural build order (i.e., whatever `place` produces), not a specific
  asserted reordering — or drop the "boxes before labels and arrows" test if
  it no longer reflects an invariant anything depends on.
- Update `render/mod.rs` test imports and call sites that currently import
  `with_caret`/`with_glow` to call `diagram` with the new `editing` tuple
  shape instead.
