# 107: Delete a box

## Story

Doug has a diagram with "API gateway" at the top and "Auth", "Payments" and
"Orders" underneath. "Payments" is outdated, so he selects it and presses `d`.
"Payments" and anything under it vanish, and the selection lands on "Orders".
He changes his mind and presses `u`, and "Payments" comes back.

## Acceptance Criteria

- Pressing `d` in command mode deletes the selected box and all its descendants.
- The selection moves to the next sibling. If there is none, it moves to the
  previous sibling. If the box was an only child, it moves to the parent.
- Deleting the root box empties the canvas, like a fresh `dre`.
- Pressing `u` right after a delete brings back the box and its descendants,
  with the selection where it was before the delete.

## Technical Design

### Rule

One rule, no special case for the root: delete the selected box and its
descendants. `doc.boxes` can hold several top-level boxes (`s` on a top-level
box, `b` with nothing selected), so a top-level box is just a box whose parent
list is `doc.boxes`. The canvas only empties when the last top-level box goes.

### `diagram.rs`: `remove`

A pure function next to `append`:

```rust
pub(crate) fn remove(boxes: &mut Vec<Node>, path: &Path) -> Option<Path>
```

- Removes the node at `path` (with its children) from its sibling list, using
  `children_at`.
- Returns the new selection:
  - a next sibling exists (the box that slid into `path.index`): same `path`;
  - else a previous sibling exists: `path` with `index - 1`;
  - else the box was an only child: the parent (`ancestors` popped, last
    ancestor becomes `index`);
  - else it was the only top-level box: `None`.
- Unit-tested against plain `Vec<Node>`, no `State` needed.

### `command_mode.rs`: `Command::Delete`

- `parse("d")` maps to `Command::Delete`. New `COMMAND_KEYMAP` entry ("Delete
  the selected box and its descendants"), placed after `s`.
- `is_undoable(Delete)` is true. `reduce` calls `snapshot` before it takes
  `doc.selected`, so the history entry holds the box, its descendants and the
  selection. `u` then restores all three with no new undo code (AC 4).
- `min_depth(Delete)` is 1 (falls into the `_` arm). With no selection, `d`
  returns before the snapshot, so nothing is pushed to history.
- New handler `delete_box(state, path)`, same shape as `toggle_fill`: calls
  `remove(&mut state.doc.boxes, &path)` and stores the result in
  `state.doc.selected`. Mode stays `Command`.
- The count prefix is ignored: `3d` acts like `d`, same as `b`, `s` and `c`.
  `reduce` already takes `pending_count`, so it is cleared.

### README

The command-mode keymap table in `README.md` is checked by a test against
`COMMAND_KEYMAP`. Regenerate it with `UPDATE_README=1 cargo test`.

### Collaborators

- `diagram::remove` uses `children_at`.
- `command_mode::delete_box` uses `remove`.
- `state::snapshot` and `state::undo` are unchanged.

### Tests

- `remove`: next sibling, previous sibling when last, only child moves to the
  parent, only top-level box gives `None`, descendants vanish, other top-level
  boxes are untouched.
- `handle_key(state, "d")`: the story (Payments deleted, selection on Orders),
  then `u` brings back Payments with its children and the selection.
- `d` with nothing selected is a no-op and leaves history empty.
- `3d` deletes one box.
