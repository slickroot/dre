# 066 - Fill a box and its siblings at once

## Story

Sam has a box with several children. He selects one of the children, presses
`F`, and the whole row takes the same fill together instead of him filling each
box one at a time.

## Acceptance Criteria

- `F` on a box whose siblings all share its fill advances the whole row one
  step along the palette.
- `F` on a box whose siblings have mixed fills sets the whole row to the first
  fill colour in the palette.
- Repeated `F` keeps stepping the row along the palette together, wrapping
  around to no fill after the last one.
- `F` fills only the selected box and the boxes sharing its parent.
- `F` on a top-level box does nothing.
- `F` with nothing selected does nothing.

## Technical Design

This is the fill twin of the archived colour story 028. Fill already steps the
same 5-colour palette as colour — single-box `f` calls
`next_colour(node.fill)` (`src/command_mode.rs`) — so the design mirrors
`colour_row` exactly, field-for-field on `fill`. No new abstraction; `colour_row`
and `fill_row` stay as two small sibling helpers.

- Add a pure helper `fill_row(boxes: &mut Vec<Node>, path: &Path)` in
  `src/state.rs`, right beside `colour_row`:
  - `siblings = children_at(boxes, &path.ancestors)` (never empty: the selected
    box is always among them).
  - Let `first_fill = siblings[0].fill`; `uniform = siblings.iter().all(|b| b.fill == first_fill)`.
  - `new_fill = if uniform { next_colour(first_fill) } else { Some(0) }` —
    uniform rows step one slot (wrapping to `None` via `next_colour`), mixed
    rows jump to the first fill colour.
  - `for sibling in siblings.iter_mut() { sibling.fill = new_fill; }`.
- In `src/command_mode.rs`:
  - Add `Command::CycleSiblingsFill`.
  - `parse` maps `"F"` to it; `writer.rs` reads single bytes so `F` arrives
    distinct from `f`.
  - New reducer `cycle_siblings_fill(state, path)`: call
    `fill_row(&mut state.doc.boxes, &path)`, keep `state.doc.selected = Some(path)`,
    and never touch `colour`.
  - `reduce` handles `(Command::CycleSiblingsFill, Some(path))`; a `None`
    selection falls through to the existing `(_, None) => state` no-op.
  - `min_depth(CycleSiblingsFill) = 2` — one guard covers both the "nothing
    selected" (depth 0) and "top-level box" (depth 1) no-ops, exactly like
    `CycleSiblingsColour`.
  - Add `CycleSiblingsFill` to `is_undoable` so `u` restores the whole row with
    a single snapshot.
  - Bump the test-only `COMMANDS: [Command; 14]` array to 15 and add the
    variant so the min-depth guard test covers it.
- Tests:
  - `state.rs` unit tests for `fill_row`, mirroring `colour_row`: uniform
    siblings advance together; mixed siblings all become the first fill colour;
    stepping `PALETTE_SIZE + 1` times wraps the row back to no fill.
  - `command_mode.rs` reducer tests: `F` on a top-level box is a no-op; `F`
    with nothing selected is a no-op; `F` on uniformly filled children advances
    the whole row and leaves colour untouched; `parse_maps_known_keys` gains the
    `"F"` assertion.
- No changes to `writer.rs`, `render.rs`, `dre_format.rs`, or `layout.rs` —
  key reading, palette rendering, and persistence already handle `fill`
  independently of `colour`.