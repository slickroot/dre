# layout owns PlacementNode

Step of spec 117 (one-direction architecture). A technical spec with two
deliberate user-facing removals: the empty-drawing hint and the numbered colour
palette (spec 103).

## Problem

Data should flow one way: `diagram` → `layout` → `render`. Today the renderers
reach back into the domain:

- `PlacementNode::Node(&Node)` hands the domain type to the renderers, which
  read `.colour`, `.filled`, `.rounded` and `.hint` off it (`sprite_key` and
  `outline_box` in `terminal.rs`, `rect` in `svg.rs`).
- The terminal renderer builds a `Node` itself (`hint_node()`) to show "press b
  to add a box" on an empty drawing.
- The colour overlay's `selected_colour` reads `Node.colour` through
  `state.doc` directly.

## Acceptance Criteria

- No production code under `src/render/` imports `crate::diagram`.
- `PlacementNode`'s box variant is `Box { colour, fill, rounded }`, plain data
  computed by `layout`. It holds no `Node`.
- `layout` reads `Node` only through `label()`, `colour()`, `filled()` and
  `rounded()`.
- The empty-drawing hint is gone: an empty drawing shows an empty canvas and the
  status line.
- The numbered colour palette (spec 103) is gone: `c` and `C` only cycle, and a
  digit is always a count.
- No renderer test goes through `layout` or `reduce`.

## Technical Design

### The flow

The renderers keep `Renderer::render(&mut self, state: &State, out)`. They read
`State`, call `layout(state.doc.tree())` and `with_cursor(.., state.selected)`,
and draw the placements. `render` depends on `state` and `layout`, never on
`diagram`. No `View` type and no new module: the renderer is the view.

`layout` keeps taking `&Tree<Node>`. `Document::tree()` is already the read
path and every caller passes it.

### `diagram.rs`

- `Node` gets read methods `label() -> &str`, `colour() -> Option<u8>`,
  `filled() -> bool`, `rounded() -> bool`. Fields stay `pub(crate)` until spec
  124 makes them private.
- `Node.hint` is deleted.

### `layout.rs`

- `PlacementNode::Node(&'a Node)` becomes
  `PlacementNode::Box { colour: Option<u8>, fill: Option<u8>, rounded: bool }`.
  `layout` computes `fill` once as `filled.then_some(colour).flatten()`, the
  rule `sprite_key`, `outline_box` and `svg::rect` each repeat today.
- The enum keeps the name `PlacementNode`. `Placement` and `Label` keep their
  `'a`: `Label.text` still borrows the label, which ties the renderers to
  nothing.
- `Label.hint` is deleted.
- Every read of a `Node` goes through the getters.

### `render/`

- `terminal.rs`: `sprite_key`, `outline_box` and `draw_box` read the `Box`
  variant. `SpriteKey::Box` drops `hint`. `render_diagram` becomes
  `with_cursor(layout(state.doc.tree()), state.selected.clone())`, the same as
  `SvgRenderer`. `HINT_TEXT`, `hint_node()`, and the `use crate::diagram::Node`
  and `use types::Tree` imports are deleted.
- `svg.rs`: `rect(placement, node: &Node)` takes the `Box` variant's fields.
- `font.rs`: `GlyphSource::glyph(ch)` loses its `hint` parameter. The cache key
  is the `char` alone, and the ink is always opaque.

### Removing the colour overlay (spec 103)

- `state/`: `State.colour_overlay` is deleted, with its sets in `cycle_colour`,
  `cycle_siblings_colour` and `reduce`. `accumulate_digit` only accumulates the
  count. `history.rs` drops the `Action::Digit(_) => state.colour_overlay`
  undoable case.
- `terminal.rs`: `render_colour_overlay`, `swatch_canvas`, `selected_colour`,
  `PALETTE_ROWS` and the `OVERLAY_*` constants are deleted.

### Tests

- Renderer tests build what the renderer reads, directly:
  - draw-level tests build `Placement`s by hand, with `PlacementNode::Box`;
  - `render(&state)` tests build a `State` with `new_state` and `diagram`'s test
    builders (and, after spec 124, the `Document` edit methods). Never `reduce`.
- Renderer tests that lay out a tree and re-derive layout's arithmetic
  (`layout::width`, `layout::centre` in `terminal.rs` around lines 1229 to 1347,
  `svg.rs:307`, and the `layout(&nodes)` calls in both files) are rewritten over
  hand-built placements or deleted. Anything they check about positions belongs
  in `layout.rs`'s tests.
- Deleted with the hint: `a_hint_node_produces_a_hint_label`,
  `a_hinted_glyph_is_faded_compared_to_a_plain_glyph`,
  `sprite_key_differs_by_hint`, `an_empty_canvas_renders_the_hint_box`,
  `a_canvas_with_a_box_does_not_show_the_hint`,
  `no_cursor_is_drawn_over_the_hint_even_with_a_selection`,
  `a_hint_boxs_edge_is_fainter_than_a_normal_boxs_edge`. One new test: an empty
  drawing draws no placements.
- Deleted with the overlay: the `colour_overlay` tests in `state/command.rs`
  and `state/history.rs`, and the swatch and overlay tests in `terminal.rs`. One
  new test: a digit after `c` is a count, not a colour.

### Order

Each step is its own commit and leaves the build green.

1. Remove the colour overlay.
2. Remove the hint.
3. Add the `Node` getters and read through them in `layout`.
4. `PlacementNode::Box` with `fill` computed in `layout`. The renderers stop
   importing `diagram`, and their tests are rewritten.

### Considered and rejected

- `layout(&Document)`: no decoupling gain over `&Tree<Node>`.
- A `View` type or `view.rs` between `state`, `layout` and `render`: the
  renderer already is the view.
- `layout::placeholder(text)` or a `State`-owned placeholder: the hint is
  removed instead.
- `State::selected_colour()`: the overlay is removed instead.
- Renderer tests building `State` by replaying keys through `reduce`: end to
  end; the renderer never calls `reduce`.
- Owning `Label.text` to drop `Placement`'s lifetime: not needed for the flow.

### Runs before spec 124

Spec 124 (edits are `Document` methods) makes `Node`'s fields private. Doing
125 first means the renderers no longer read or build a `Node`, so 124 needs no
temporary constructor or `hint()` getter.
