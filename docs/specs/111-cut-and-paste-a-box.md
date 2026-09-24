# 111: Cut and paste a box

## Story

Doug has "API gateway" at the top with "Auth", "Payments" and "Orders"
underneath, and "Payments" has "Stripe" under it. He decides "Payments" belongs
under "Orders". He selects "Payments" and presses `d`. "Payments" and "Stripe"
vanish. He moves the selection to "Orders" and presses `p`. "Payments" comes
back under "Orders" with "Stripe" still under it, and it is now selected. He
never had to rebuild anything.

## Acceptance Criteria

- Pressing `d` puts the deleted box and all its descendants on a clipboard.
- A later `d` replaces what is on the clipboard.
- Pressing `u` to undo a delete does not empty the clipboard.
- Pressing `p` pastes the clipboard's box and its descendants as the last child
  of the selected box.
- After `p`, the pasted box is selected.
- Doug can press `p` many times to paste the same branch again.
- Pressing `u` right after a paste removes the pasted branch and puts the
  selection back where it was before the paste.
- Pressing `p` with an empty clipboard does nothing.
- If Doug deleted the top box and the canvas is empty, `p` brings the branch
  back as the top box.
- Pressing `3p` (any count) pastes that many copies as the last children of the
  selected box, and the last copy is selected. One `u` removes all the copies
  and puts the selection back where it was before the paste.

## Technical Design

### Clipboard

`State` gets a new field `clipboard: Option<Node>`, next to `history`. It is
not part of `Document`, so it is never saved and never in a history snapshot.
`undo` only swaps `doc` back, so undoing a delete leaves the clipboard alone
with no new code. It holds one branch: a later `d` replaces it, and `None`
means nothing to paste. `p` never takes it, so the same branch pastes again.

### `diagram.rs`: `remove`

`remove` stops computing the selection and returns the node it took out:

```rust
pub(crate) fn remove(boxes: &mut Vec<Node>, path: &Path) -> Node
```

It removes the node at `path` (with its children) from its sibling list, using
`children_at`, and hands it back. Tests: it returns the node with its
descendants, and other top-level boxes are untouched.

### `command_mode.rs`: `Command::Delete`

`delete_box(state, path)`:

1. `let node = remove(&mut state.doc.boxes, &path)` and store it in
   `state.clipboard`, replacing what was there.
2. Work out the new selection inside `delete_box`, from the sibling list after
   the removal (`children_at(boxes, &path.ancestors)`):
   - `path.index < siblings.len()`: a next sibling slid in, keep `path`;
   - else `path.index > 0`: previous sibling, `index - 1`;
   - else pop the last ancestor to get the parent;
   - else the box was the only top-level box: `None`.
   The rule moves out of `remove` and is not extracted into a helper. The
   selection tests that used to live on `remove` move to `handle_key(state,
   "d")` tests.

Everything else about `d` is unchanged: undoable, `min_depth` 1, count ignored.

### `command_mode.rs`: `Command::Paste`

- `parse("p")` maps to `Command::Paste`. New `COMMAND_KEYMAP` entry ("Paste the
  cut box and its descendants as the last child of the selected box"), placed
  after `d`.
- `is_undoable(Paste)` is true, so `reduce` calls `snapshot` before it takes
  `doc.selected`. The snapshot holds the pre-paste doc and selection, so one
  `u` restores both (AC 7). `min_depth(Paste)` is 0, like `NewBox`, so `p` works
  with nothing selected.
- `reduce` gets an arm `(Command::Paste, selected) => paste_box(state,
  selected, count)`. Unlike `d`, the count is used.
- `paste_box(state, selected, count)`:
  - `clipboard` is `None`: return the state unchanged, then
    `drop_snapshot_if_unchanged` pops the entry `reduce` pushed (the same trick
    edit sessions use). Known limitation: `snapshot` also set `dirty`, and the
    pop does not clear it, so an empty `p` still shows the `+` marker.
  - `selected` is `Some(path)`: the target list is
    `children_at(boxes, path.ancestors + [path.index])`.
  - `selected` is `None`: the target list is `doc.boxes`, so `p` after deleting
    the top box brings the branch back as the top box.
  - Loop `count` times: `append(list, node.clone())`. The clipboard is cloned,
    never taken.
  - The selection becomes the last appended copy: `Path { ancestors: target,
    index: last_returned_index }`. Mode stays `Command`.
- `3p` is one command, so it is one snapshot and one undo step. There is no
  snapshot inside the loop.

### README

The command-mode keymap table in `README.md` is checked by a test against
`COMMAND_KEYMAP`. Regenerate it with `UPDATE_README=1 cargo test`.

### Collaborators

- `command_mode::delete_box` uses `diagram::remove` and `children_at`, and
  writes `State.clipboard`.
- `command_mode::paste_box` uses `children_at`, `append` and
  `drop_snapshot_if_unchanged`, and reads `State.clipboard`.
- `state::snapshot` and `state::undo` are unchanged.

### Tests

- `remove`: returns the node with its descendants; other top-level boxes are
  untouched.
- `d`: puts the box and its descendants on the clipboard; a second `d`
  replaces it; selection is next sibling, else previous sibling, else parent,
  else `None`; `u` after `d` restores the box and leaves the clipboard filled.
- `p`: the story (Payments and Stripe under Orders, Payments selected);
  pasting again gives a second copy; `u` right after removes the branch and
  restores the earlier selection; empty clipboard is a no-op and leaves
  history unchanged; after deleting the only top-level box, `p` restores it as
  the top box and selects it; `3p` gives three children, selects the last, and
  one `u` removes all three.
