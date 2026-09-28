# One selection representation

## Refactoring

Selection is currently represented twice in the render scene: the selected
`Box` carries `selected: true`, and `with_glow` adds a separate `Glow`
placement. The terminal renders the `Glow`, while SVG reconstructs the glow
from the `Box` flag. This refactoring makes `Glow` the sole selection
representation so both renderers consume the same scene data.

## Technical Design

### Placement model

- Remove `selected: bool` from `PlacementNode::Box`.
- Keep `PlacementNode::Glow { colour, rounded }` as the only selection marker.
- `layout::diagram`, `place`, `visit`, and their `emit` helper no longer accept
  or derive a `selected` argument. Layout emits ordinary boxes regardless of
  editor selection.
- `view::body` remains responsible for calling `with_glow` after layout and
  caret composition. `with_glow` continues to locate the selected label and
  its containing box, then prepends one glow placement with the box colour,
  rounding, and one-cell margin.

### Renderer behavior

- The terminal box renderer always paints the normal box edge colour;
  selection no longer changes the box border colour.
- The terminal continues to draw `Glow` placements at the glow layer.
- SVG `paint` processes each placement in scene order with one `match` over
  `PlacementNode`. Every variant that can render has an explicit arm,
  including `Glow`; no placement is silently ignored by a chain of `if let`
  passes.
- The SVG `Glow` arm calls the existing glow-rectangle writer using the
  placement’s colour, roundedness, and geometry. Glow output remains before
  labels/content when supplied by the existing scene ordering.
- SVG document definitions are detected from placement variants: arrow marker
  definitions when an `Arrow` exists, and glow filter definitions when a
  `Glow` exists. No definition is inferred from box state.
- Existing box fills, labels, arrows, carets, cursors, footer boxes, and LEDs
  retain their rendering behavior.

### Tests

- Update all placement constructors and pattern matches to omit the removed
  `Box.selected` field.
- Replace layout tests asserting selected flags with tests asserting that
  `layout::diagram` emits no selection state and that `with_glow` adds exactly
  one `Glow` for the selected box and none without a selection.
- Update terminal tests so an ordinary box is the only box sprite and a
  selected scene is represented by an additional `Glow`; verify the box edge
  is not selection-brightened.
- Update SVG tests to construct selection with a `Glow` placement. Verify the
  glow geometry, colour, rounding, filter definition, and fill/glow/label
  ordering still hold. Verify a `Glow` placement is rendered and an ordinary
  box alone emits no glow/filter.
- Run the full Rust test suite; compilation must prove no `Box.selected`
  representation or selection argument remains in the placement pipeline.
