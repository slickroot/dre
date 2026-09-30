## Story

Doug runs `dre docs/plans.dre`. He sees his diagram, and in the bottom-right corner of the screen a small label saying `plans`. The big coloured bar is gone. He always knows which diagram he's working on, and the diagram has more room.

## Acceptance Criteria

- The 3-row bottom bar is gone.
- When a file is open, its name appears in the bottom-right corner on a single row. The label takes only the width of the name, not the whole width of the window.
- The name is the file name without the folder and without `.dre`. For `docs/plans.dre` it shows `plans`.
- When no file is open, the corner stays empty.
- The bottom row is reserved for the name. The diagram is centred in the rows above it and cropped there, so it never draws into that row.
- The terminal, the SVG export and the web view all share the same screen layout, so they all lose the bar and reserve the bottom row. The SVG export shows the name when the state has a file path. The web view has none, so its corner stays empty.

## Technical Design

### Decisions

- `render::editor` stays the one place that composes the screen, shared by the terminal and SVG renderers (and so the web view). There is no terminal-only path.
- The footer is 1 row: `FOOTER_ROWS = 1`. `FOOTER_COLOUR` and the filled-box footer go away.
- If a name is wider than the window, nothing special happens. It is not in scope.

### Components

- **`state::diagram_name(path: &str) -> &str`** (pure function in `src/state`). It strips the folder and the `.dre` extension: `docs/plans.dre` gives `plans`. It knows nothing about rendering.
- **`layout::footer(name: Option<&str>) -> Vec<Placement>`**.
  - `None` returns `vec![]`, so the corner stays empty.
  - `Some(name)` returns one `PlacementNode::Label(Label { text: name, path: vec![] })` at `x: 0, y: 0`, `width: name.chars().count()`, `height: 1`.
  - An empty `path` belongs to no diagram node. Selections are never empty, so `with_cursor` can never match it.
- **`render::align_right(placements, area)`**, next to `centre`. It moves the placements so their right edge meets the right edge of `area` (`x = area.col + area.cols - width`, `y = area.row`). It is a plain subtraction with no clamping.
- **`render::editor`**:
  - `stack([None, Some(FOOTER_ROWS)], window)` gives the body and the 1-row footer.
  - The body is unchanged: `centre(with_cursor(diagram, selected), body)`. It is cropped to the body area, so it never draws into the last row.
  - The footer is `align_right(footer(state.save_to.as_deref().map(diagram_name)), foot)`.
  - The `fill` helper is no longer needed and can be deleted if nothing else uses it.

### Collaborators

- `State.save_to` is the only input. It is `Some` for an opened file and for a new file with a path, so a new file shows its name straight away. It is `None` with no file.
- `TerminalRenderer` and `SvgRenderer` are untouched apart from `FOOTER_ROWS`. Both already draw `Label` placements and already crop each area. The SVG window without a canvas is still `diagram height + FOOTER_ROWS`, which is now 1.

### Tests (outside-in)

- `diagram_name`: `docs/plans.dre` gives `plans`; `plans.dre` gives `plans`; a name without `.dre` is unchanged.
- `footer`: `None` is empty; `Some("plans")` is one label of width 5 and height 1 with an empty path.
- `align_right`: the right edge meets the area's right edge on the area's row, including in an area with a non-zero `col` and `row`.
- `editor`: the areas are the body and a 1-row footer; with a path, the last placement is the name label in the bottom-right corner; without one, the footer has no placements.
- Terminal: the diagram is centred in the rows above the last one, and a box overhanging the body does not draw into the last row (update the existing footer tests to `FOOTER_ROWS = 1`).
- SVG: update the footer tests (`footer_bar`, the stacked-areas tests) so they expect the label and not the filled box.
