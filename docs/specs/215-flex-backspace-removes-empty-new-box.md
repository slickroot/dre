# Flex: Backspace removes an empty new box

## User Story

Mina is in Write mode on a box she just created with `o`, empty, having changed her mind. She presses Backspace. The box disappears, and she lands in Move mode on the previous sibling box — or, if it was the first child, on its parent.

## Acceptance Criteria

- Pressing Backspace while the current box's text is already empty, and the box is borderless with no children (the same condition used for dropping an empty box on Enter), removes the box.
- After removal, the previous sibling under the same parent becomes selected, if one exists.
- If the removed box was the first child (no previous sibling), its parent becomes selected instead.
- Mode switches to Move after the removal.
- Pressing Backspace on a bordered box with empty text (e.g. one entered via `i`) leaves the box in place — unchanged from today, just clears/keeps text as before.
- Backspace on non-empty text still just removes the last character, unchanged from today.

## Technical Design

- Extract the "box can be dropped" condition (currently inlined in `drop_empty_text`) into a shared helper `fn is_droppable(b: &FlexBox, children: &[Vec<usize>]) -> bool { !b.border && children.is_empty() }` in `src/flex/state.rs`. Both `drop_empty_text` (Enter) and the new Backspace branch call it.
- Add a helper `fn sibling_or_else_parent(boxes: &Tree<FlexBox>, path: &[usize]) -> Vec<usize>` in `src/flex/state.rs`: if `path.last() == Some(&0)` (first child, no previous sibling), return `boxes.parent(path)`; otherwise return `boxes.previous(path)`. This keeps `write_key` free of path arithmetic.
- In `write_key`'s `"\x7f"` arm, check whether text is already empty (`state.selected_mut().text.as_deref() == Some("")`) and the box `is_droppable`. If so:
  - Record history via a new `pub(super) fn record(state: &mut FlexState)` in `src/flex/history.rs` (pushes a `Snapshot` the same way `recorded` does today), called as `history::record(&mut state)` before mutating.
  - Compute `let next = sibling_or_else_parent(&state.boxes, &state.selected);` *before* removing (removal shifts sibling indices).
  - `state.boxes.remove(&state.selected); state.selected = next;`
  - Set `state.mode = FlexMode::Move;`
  - Otherwise (text non-empty, or box is bordered/has children), fall back to the existing behavior: pop the last character if present (a no-op when text is already empty on a non-droppable box, matching "leaves the box in place... just clears/keeps text as before").
- History recording is done directly inside `write_key`'s branch (not via `history::recorded`/`undoable`), since whether this Backspace is undo-worthy depends on runtime state (emptiness + droppability), not just the key — `reduce` continues to route only Move-mode keys through `history::recorded`.
