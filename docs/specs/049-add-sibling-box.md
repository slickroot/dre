# Add a Sibling Box

## User Story

As someone editing a diagram, I want to press `s` while a box is selected to add a new sibling box, so that I can grow the tree sideways without navigating back to the parent and pressing `b`.

## Acceptance Criteria

- Pressing `s` while a box with a parent is selected appends a new empty box to the end of that parent's children (same append logic as `b` uses today).
- Pressing `s` while a top-level box (no parent) is selected appends a new empty top-level box to the end of the top-level list.
- The newly created sibling becomes the selected box, and the editor immediately enters label-editing mode so you can start typing right away.
- Pressing `s` does nothing if no box is currently selected.

## Technical Design

Reuse the existing `grow()` helper (`dre/state.py`) unchanged — it already appends a new empty `Box` to whatever list of boxes lives at a given `Path` (top-level `boxes` when the path is empty, or a box's `children` otherwise) and returns the updated tree plus the `Path` to the new box.

Add `NEW_SIBLING = "s"` to the `Command` enum, next to `NEW_BOX = "b"`.

Handle it in `handle_command` as:

```python
if command == Command.NEW_SIBLING:
    if not state.selected:
        return state
    boxes, selected = grow(state.boxes, state.selected[:-1])
    return replace(state, boxes=boxes, mode="insert", selected=selected)
```

Key points:
- The sibling's parent list is `state.selected[:-1]` — the path to the *parent* of the selected box. Slicing off the last element naturally yields `()` when the selected box is top-level, so `grow()` appends to the top-level `boxes` tuple in that case, and to the parent's `children` otherwise — no branching needed, same logic `grow()` already uses internally.
- Unlike `NEW_BOX`, `NEW_SIBLING` must guard on `if not state.selected: return state` before slicing. `NEW_BOX` has no such guard because it also serves to create the very first box in an empty document (where `selected == ()` by default); `NEW_SIBLING` has no equivalent case, so the guard implements acceptance criterion 4 directly.
- Setting `mode="insert"` in the same `replace(...)` call enters label-editing mode immediately, mirroring `NEW_BOX`. No call to `enter_insert()` is needed since the new box's label already defaults to `PAD`.
