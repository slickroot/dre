# Flex: removing a box selects the following sibling

## User Story

Doug is arranging boxes in dre-flex. One of them isn't working out, so in Move
mode he selects it and presses `d`. The box and everything inside it disappear,
and the box that used to sit right after it slides down into its slot and becomes
selected. If the box he cut was the last of its siblings, the box that held it
becomes selected — or the canvas, if it sat at the top. The same happens when he
backs out of a fresh empty box or drops it with Enter: focus always lands on
whatever took its place, never on the whole canvas unless there is nothing left.
He changes his mind, presses `u`, and it all comes back. Happy, he carries on
drawing!

## Acceptance Criteria

1. In Move mode with a box selected, `d` cuts it. If it has a **following
   sibling**, that sibling — the box that shifts down into the cut box's slot —
   becomes selected.
2. If the cut box was the **last** sibling, its **parent** becomes selected. For
   a top-level box (`[0, i]`) the parent is the canvas `[0]`.
3. Write-mode `\x7f` on an empty borderless box removes it and selects the
   following sibling, falling back to the parent when it was the last sibling.
4. Enter/Esc on an empty borderless box drops it and selects the following
   sibling, falling back to the parent when it was the last sibling.
5. `d` on the canvas changes nothing and pushes no undo snapshot (spec 231).
6. In Write mode `d` is typed into the text and removes nothing.
7. One `u` after a cut restores the box, everything nested inside it, and the
   selection it had before the cut.

## Technical Design

### Decision summary

- **Removal owns the selection rule.** A single `FlexState::remove_selected`
  removes `self.selected` and *always* sets the new selection to the **following
  sibling, else the parent**. No caller chooses where focus goes.
- **All three removal paths share it**: `d` (cut), write-mode `\x7f` on an empty
  borderless box, and `drop_empty_text` (Enter/Esc on an empty borderless box).
- The "following sibling" is the child now sitting at the removed box's index,
  because removal shifts later siblings down. If that index is past the end, the
  parent is selected. The index is read *before* removal, so after removal the
  child at that index is the box that took the slot.
- This spec is written **against spec 231**: paths are `[]` inert holder ->
  `[0]` canvas -> `[0, i]` user boxes, and the parent of a top-level box is the
  canvas `[0]`.
- `sibling_or_else_parent` (the previous-preferring helper) is **deleted**.
- Clipboard and history are unchanged: `d` still keeps the removed subtree on the
  clipboard, and `y` does **not** use `remove_selected` (it removes from a clone
  and must not move the selection).

### The primitive

`FlexState` gains one method:

```rust
fn remove_selected(&mut self) -> Tree<FlexBox> {
    let parent = parent_path(&self.selected);
    let index = self.selected.last().copied().expect("the canvas is never removed");
    let removed = self.boxes.remove(&self.selected);
    let children = self.boxes.children(&parent);
    self.selected = children.get(index).cloned().unwrap_or(parent);
    removed
}
```

- Knows: the selected path and the tree.
- Does: detaches the selected subtree (children included), remembers the index it
  occupied, then selects the child now at that index — the following sibling — or
  the parent when the removed box was the last sibling.
- Collaborators: `Tree::remove`, `Tree::children`, `parent_path`
  (`types::Tree` and `state.rs`).

### Call sites

**`cut_selected`** (`state.rs:354`) becomes just the clipboard write:

```rust
fn cut_selected(mut state: FlexState) -> FlexState {
    state.clipboard = Some(state.remove_selected());
    state
}
```

**`move_key`'s `"d"` arm** (`state.rs:317`) drops its `if !state.selected.is_empty()`
guard — spec 231 already guards the canvas in `reduce` — and calls `cut_selected`.

**`write_key`'s `"\x7f"` drop branch** (`state.rs:217`) replaces the
`sibling_or_else_parent` + `remove` + selected assignment with a single
`state.remove_selected()`, keeping `history::record(&mut state)` before it and
`state.mode = FlexMode::Move` after.

**`drop_empty_text`** (`state.rs:161`) replaces its `parent`/`remove`/selected
assignment with `state.remove_selected();` in the droppable branch.

### Removed / renamed items

- `sibling_or_else_parent` (`state.rs:153`) and its two unit tests
  (`state.rs:458`, `:464`) are deleted.

### Interaction with spec 231

- `remove_selected` is never reached with the canvas `[0]` selected: 231's
  `canvas_self_edit` guard blocks `d` on the canvas, the canvas never enters Write
  mode (so no backspace/Enter drop), and `parent_path([0])` is the inert `[]`,
  which is never passed to `Tree::remove`.
- This supersedes 231's design note that `cut_selected` keeps
  `selected = parent_path(&selected)`: the fallback is now following-else-parent,
  and the parent is only chosen when there is no following sibling.

### Test plan

`flex::state` (on top of 231's updated paths):

1. `remove_selected` unit tests, replacing the deleted
   `sibling_or_else_parent_returns_*` pair:
   - removes a middle sibling and selects the one that shifted into its slot;
   - removes the last sibling and selects the parent;
   - removes a top-level box last-in-line and selects the canvas `[0]`.
2. AC 1/2 — `d_cuts_and_selects_the_following_sibling` (middle of three ->
   following) and `d_on_the_last_sibling_selects_the_parent` (top-level last ->
   canvas `[0]`; nested last -> its parent box).
3. AC 3 — `backspace_drops_an_empty_box_and_selects_the_following_sibling`;
   rewrite `backspace_on_an_empty_borderless_box_with_a_previous_sibling_selects_it`
   for the following sibling.
4. AC 4 — `enter_drops_an_empty_box_and_selects_the_following_sibling`; update
   `enter_on_an_empty_borderless_copy_drops_it_and_returns_to_move`,
   `esc_with_empty_text_...` and `o_then_enter_...` expectations to
   following-else-parent.
5. AC 5 — keep 231's `self_edit_keys_on_the_canvas_change_nothing` (`d` included).
6. AC 6 — `d_in_write_mode_is_typed_and_removes_nothing` (unchanged).
7. AC 7 — `u_after_d_restores_the_box_and_its_contents_and_selection` (unchanged).

`flex::history`: no production change; `d` stays in the Move `undoable` list.
