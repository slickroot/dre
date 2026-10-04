# Flex: `p` pastes after the selected box

## User Story

Noor is arranging boxes in dre-flex. She cuts a box with `d`, then decides she
wants it back — but not buried inside another box. She selects the box she wants
it to sit *after* and presses `p`. The cut box reappears as its next sibling, on
the same level, with everything nested inside it still nested. She presses `p`
again and another copy lands right after the same anchor, ahead of the first.
She realises she overdid it, presses `u`, and the last paste is gone. Happy, she
carries on drawing!

## Acceptance Criteria

1. In Move mode, with a box on the clipboard and a **real box** selected, `p`
   inserts the clipboard box as the **next sibling** of the selected box —
   immediately after it, at the same depth. Nothing is inserted *inside* the
   selected box.
2. The pasted box keeps its whole subtree, plus its label, border, fill,
   padding, justify and direction.
3. The selection stays on the original selected box (the anchor), so `p` can be
   pressed again.
4. Pressing `p` twice inserts two independent copies, each immediately after
   the anchor, so the copies end up in reverse order: `[anchor, copy₂, copy₁]`.
5. The clipboard is kept, so `p` can paste again.
6. With the canvas `[0]` selected and a box on the clipboard, `p` appends it as
   the canvas's last child (a new top-level box) and the canvas stays selected.
   "After the current" degrades to "append" because the canvas has no siblings.
7. With an **empty clipboard**, `p` leaves the diagram, the selection and the
   history unchanged — it pushes **no** undo snapshot.
8. One `u` after a paste removes the last-pasted box and restores the boxes and
   selection from before the paste; the clipboard is untouched.
9. In Write and Replace modes, `p` is typed as the letter "p" and pastes
   nothing.

## Technical Design

### Decision summary

- **"After the current" is the next sibling.** For a real box, `p` inserts at
  `parent_path(selected)` at `selected.last() + 1`, exactly the shape `o`/`O`
  already use (`state.rs:295`). Insertion shifts later siblings down, so the
  pasted box lands immediately after the anchor and nothing goes inside it.
- **The selection is never reassigned.** `paste` only inserts; the anchor stays
  selected. That is why pressing `p` twice keeps using the same anchor and the
  two copies end up in reverse order (AC 3, 4).
- **The canvas is the one exception.** `is_canvas(selected)` means there is no
  sibling to paste after, so `p` appends the branch as the canvas's last child
  (`push(&[0], branch)`) and the canvas stays selected (AC 6). This preserves
  spec 229's canvas rule.
- **Empty clipboard is decided before history.** A guard arm in `reduce` (next
  to spec 231's canvas guard) short-circuits `p` when `clipboard.is_none()`, so
  nothing is recorded (AC 7). `p` stays in `history::undoable`; the guard simply
  keeps the empty case from ever reaching `recorded`.
- **`paste_clipboard` is split.** Its branch moves into `move_key`'s `"p"` arm
  and the two cases become two small single-purpose helpers.
- This spec is written **against spec 231**: paths are `[]` inert holder ->
  `[0]` canvas -> `[0, i]` user boxes, and `is_canvas(path) == path.len() == 1`.

### `flex::state` — the empty-clipboard guard

`reduce` (`state.rs:142`) gains one arm next to the canvas guard, **above** the
Move catch-all, so the no-op never reaches `history::recorded`:

```rust
(_, FlexMode::Move) if key == "p" && state.clipboard.is_none() => (state, None),
(_, FlexMode::Move) => history::recorded(state, key, |s| move_key(s, key)),
```

- Knows: the selected path and whether a clipboard exists.
- Does: for `p` with an empty clipboard, returns the state untouched with no
  effect and no history entry (AC 7). Applies to the canvas too.
- `p` is **not** in `canvas_self_edit` (`state.rs:157`), so a canvas `p` with a
  non-empty clipboard still falls through to `move_key` and appends (AC 6).

### `flex::state` — the paste arm and its two helpers

`move_key`'s `"p"` arm (`state.rs:326`) becomes:

```rust
"p" => {
    if let Some(branch) = state.clipboard.clone() {
        state = if is_canvas(&state.selected) {
            append_to_canvas(state, branch)
        } else {
            insert_after_selected(state, branch)
        };
    }
}
```

The two helpers replace `paste_clipboard` (`state.rs:358`):

```rust
fn append_to_canvas(mut state: FlexState, branch: Tree<FlexBox>) -> FlexState {
    state.boxes.push(&state.selected, branch);
    state
}

fn insert_after_selected(mut state: FlexState, branch: Tree<FlexBox>) -> FlexState {
    let parent = parent_path(&state.selected);
    let index = state.selected.last().copied().expect("the canvas is handled separately") + 1;
    state.boxes.insert(&parent, index, branch);
    state
}
```

- `append_to_canvas` — knows nothing beyond the state and the branch; appends
  the branch as the canvas's last child and leaves `selected` on the canvas
  (AC 6).
- `insert_after_selected` — knows the selected path; inserts the branch at the
  anchor's index + 1 under its parent and leaves `selected` on the anchor
  (AC 1-4).
- Collaborators: `Tree::push`, `Tree::insert`, `parent_path`, `is_canvas`.

### Removed / renamed items

- `paste_clipboard` (`state.rs:358`) is deleted; its two responsibilities move
  to `append_to_canvas` and `insert_after_selected`.
- `history::undoable` is unchanged: `p` stays in the `FlexMode::Move` list
  (`history.rs:14`); the empty case is prevented by the new `reduce` guard.

### Interaction with other specs

- **231 (canvas):** `p` is a structural key, not a canvas self-edit key, so the
  canvas guard does not block it — the canvas appends (AC 6). The new
  empty-clipboard guard also covers the canvas (AC 7).
- **232 (cut selects the following sibling):** no conflict. Paste only reads the
  *current* selection; 232 only changes which box that is after a cut.
- **229 (p pastes the cut box):** this supersedes its AC 1 ("as the last child of
  the selected box") and its reverse ordering is now explicit. Its rules for the
  clipboard, `u`, and Write/Replace are unchanged.
- **230 (`y`):** unaffected; `y` still fills the clipboard without moving.

### Test plan

`flex::state` (on top of 231's updated paths):

1. AC 1/2 — rewrite
   `p_appends_the_cut_branch_as_the_last_child_preserving_its_contents_and_fields`
   as `p_inserts_the_clipboard_branch_as_the_next_sibling_preserving_its_contents_and_fields`:
   after selecting a middle box and pasting, the branch is the sibling at the
   anchor's index + 1 with its subtree and every `FlexBox` field intact, and it
   is *not* a child of the anchor.
2. AC 4 — `p_twice_inserts_two_copies_immediately_after_the_anchor_in_reverse_order`
   (the second copy sits at `anchor + 1`, ahead of the first).
3. AC 1 — `p_after_a_middle_sibling_inserts_between_two_siblings` and
   `p_after_the_last_sibling_appends_at_that_level`.
4. AC 3/5 — `the_anchor_stays_selected_and_the_clipboard_survives_a_paste`.
5. AC 6 — keep
   `p_with_the_canvas_selected_adds_a_top_level_box_and_keeps_the_canvas_selected`.
6. AC 7 — rewrite
   `p_with_an_empty_clipboard_leaves_the_state_and_adds_one_history_snapshot` as
   `..._leaves_the_state_and_adds_no_history_snapshot`; add
   `p_with_an_empty_clipboard_on_the_canvas_changes_nothing`.
7. AC 8 — keep `u_after_p_restores_the_boxes_and_selection_and_keeps_the_clipboard`.
8. Unit tests for `insert_after_selected` (middle, last, nested) and
   `append_to_canvas`.
9. AC 9 — keep `p_in_write_mode_is_typed_into_the_box_and_leaves_the_padding_alone`
   and `p_in_replace_mode_is_typed_and_pastes_nothing`.

`flex::state` — `y` interaction:

- Rewrite `y_then_p_pastes_a_copy_and_leaves_the_original_in_place`: the copy of
  the yanked box now appears as a sibling after the selection, not as a child,
  and the original is untouched.

`flex::history`:

- Production unchanged. `box_text_and_toggle_keys_are_undoable_in_move_mode`
  still lists `p`; the "no snapshot when the clipboard is empty" guarantee is
  owned by the `flex::state` guard test (6), because `recorded` is never reached
  in that case.
