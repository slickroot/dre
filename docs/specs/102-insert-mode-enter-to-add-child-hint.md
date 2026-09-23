# 101: Insert mode "Enter to add a child" hint

## User Story

Sofia is editing a box's label in Insert mode. Immediately, before she types anything, she sees a faded hint box positioned exactly where a new child box would appear, reading "Enter to add a child". She keeps typing her label, and the hint stays visible the whole time. She presses Enter, and a child box is added there as promised.

## Acceptance Criteria

- While in Insert mode editing a box's label, a hint is shown at the position where a new child box would be created.
- The hint reads "Enter to add a child".
- The hint appears immediately upon entering Insert mode, before any text is typed.
- The hint remains visible as the user types the label.

## Technical Design

All three ways of entering Insert mode (`b` new child, `i` edit label, `I` rename) leave `state.doc.selected` pointing at the node being edited, and pressing Enter always appends a new child to that same node (`add_child_box`). So the hint always represents "a new child of the box currently selected."

We reuse the existing `Node.hint: bool` faded-rendering pipeline (already used for the empty-diagram "press b to add a box" hint) instead of inventing a separate position calculation. A synthetic hint node is appended as a real, temporary child of the selected node so the existing layout algorithm places it exactly where a real child would land.

- `state.rs` gains `insert_hint_box(state: &mut State)` and `remove_hint_box(state: &mut State)`:
  - `insert_hint_box`: if `state.mode == Mode::Insert` and `state.doc.selected` is `Some(path)`, appends `Node { label: "Enter to add a child".into(), hint: true, ..Default::default() }` to the children of the node at `path`. No-op otherwise.
  - `remove_hint_box`: if `state.mode == Mode::Insert` and `state.doc.selected` is `Some(path)`, unconditionally pops the last child of the node at `path`. No safety check — trusts the insert/remove pairing.
- Because `Renderer::render` takes `&State` (not `&mut State`), the mutation happens in the main loop, not inside the renderer: call `insert_hint_box(&mut state)` immediately before `renderer.render(&state, ...)`, and `remove_hint_box(&mut state)` immediately after. This keeps the feature renderer-agnostic (works for both `TerminalRenderer` and `SvgRenderer`) and avoids cloning the document tree — the tree is mutated in place for the duration of one render call and left exactly as it was before.
- No changes needed to `layout()`, `draw_box`, or the glyph-fading code — they already handle `hint: true` nodes (faded box edges, faded glyphs) via the existing empty-diagram hint feature.
