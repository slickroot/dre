# Flex: `J` and `K` move the selected box among its siblings

## User Story

Noor is arranging boxes in dre-flex. She's in Move mode with one of the boxes selected among its siblings in a column. She presses `J`, and the box swaps places with the sibling below it, staying highlighted so she can keep nudging it down. When it reaches the bottom, `J` leaves it there, so she presses `K` and walks it back up until it's at the top, where `K` leaves it. The whole time, everything nested inside the box travels with it. She changes her mind, presses `u`, and the old order comes back. Happy, she carries on drawing!

## Acceptance Criteria

1. In Move mode, with a box selected whose parent lays out as a Column, `J` swaps it with its next sibling, moving it one position down.
2. `K` swaps it with its previous sibling, moving it one position up.
3. The selected box's whole subtree of nested boxes moves with it.
4. The selection travels with the box, so the same box stays highlighted in its new position.
5. On the last sibling, `J` changes nothing; on the first sibling, `K` changes nothing.
6. When the parent lays out as a Row, `J` and `K` change nothing.
7. When the canvas is selected (no box highlighted), `J` and `K` change nothing.
8. In Write and Replace modes, `J` and `K` are typed as the letters "J" and "K" and reorder nothing.
9. One `u` after a move restores the previous sibling order and the selection.

## Technical Design

### Decision summary

- `J` and `K` are **sibling-reorder keys**, distinct from the existing lowercase
  `j`/`k` selection movement (`state.rs:261`). `J` swaps the selected node with
  its **next** sibling (down); `K` swaps with its **previous** sibling (up).
- The exchange is a **new `Tree::swap` primitive** in the `types` crate, not an
  inline `remove`/`insert` in flex. It swaps two nodes that share a parent, so
  the whole subtree travels intact.
- The reorder is guarded by three early returns in a new named helper
  `reorder_selected`: empty selection (canvas, AC 7), parent not `Column`
  (AC 6), and the sibling boundary. The boundary no-op comes for free because
  `Tree::next`/`Tree::previous` return the **same path** at the ends and
  `Vec::swap(i, i)` is a no-op (AC 5).
- The **canvas counts as a Column parent**: `parent_path([i]) == []` and
  `boxes.value(&[])` is the root `FlexBox::window()`, whose `direction` is
  `Column`, so top-level boxes reorder among themselves.
- **Any selected node reorders**, bordered box or borderless text leaf — the
  only guard is `selected.is_empty()`.
- `"J" | "K"` are added to the Move arm of `history::undoable`, so `recorded`
  snapshots before the swap and one `u` restores order **and** selection
  (AC 9). Following spec 227's `d`-on-canvas precedent, a no-op `J`/`K`
  (boundary, Row, canvas) still pushes a snapshot; AC 5/6/7 mean no *visible*
  change, not no history entry.
- Write and Replace are matched before the Move catch-all (`state.rs:131`), so
  AC 8 falls out with no extra code.

### `types::Tree` — the new primitive

Add to `types/src/tree.rs`, beside `insert` (`:69`) / `remove` (`:74`):

```rust
pub fn swap(&mut self, path: &[usize], other: &[usize]) {
    let (&last, parent) = path.split_last().expect("the root cannot be swapped");
    let (&other_last, other_parent) = other.split_last().expect("the root cannot be swapped");
    assert_eq!(parent, other_parent, "swap needs two siblings");
    self.get_mut(parent).children.swap(last, other_last);
}
```

- Knows: the two paths and the tree.
- Does: resolves the shared parent and `Vec::swap`s the two child slots. The
  subtrees are moved whole, so nested boxes come along (AC 3).
- `path == other` is a valid no-op, which is what makes the boundary case
  fall out of `next`/`previous`.
- Panics on `[]` and on paths with different parents, matching the assert
  style of `position`/`remove`.

### `flex::state` — the helper and the key arm

Add `reorder_selected` next to `sibling_or_else_parent` (`state.rs:145`) /
`parent_path` (`state.rs:304`):

```rust
fn reorder_selected(mut state: FlexState, key: &str) -> FlexState {
    if state.selected.is_empty() {
        return state;
    }
    let parent = parent_path(&state.selected);
    if state.boxes.value(&parent).direction != Direction::Column {
        return state;
    }
    let target = if key == "J" {
        state.boxes.next(&state.selected)
    } else {
        state.boxes.previous(&state.selected)
    };
    state.boxes.swap(&state.selected, &target);
    state.selected = target;
    state
}
```

- Knows: selection, the tree, and the parent's layout direction.
- Does: guards canvas / Row / boundary, swaps the selected node with its
  neighbour, and moves `selected` to the node's new path (`target`).
- Collaborators: `parent_path` (`:304`) for the canvas-as-`[]` rule,
  `Tree::value`, `Tree::next`/`Tree::previous` (`types/src/tree.rs:46`/`:55`)
  for the neighbour and boundary no-op, and the new `Tree::swap`.

`move_key` (`state.rs:240`) gains one arm before the `_ => {}` catch-all:

```rust
"J" | "K" => state = reorder_selected(state, key),
```

`FlexState`, `FlexMode`, `FlexEffect`, `write_key`, `reduce`, and the
`PartialEq` impl are unchanged.

### `flex::history`

Add `"J" | "K"` to the Move arm of `undoable` (`history.rs:12`):

```rust
FlexMode::Move => matches!(key, "a" | "o" | "O" | "i" | "s" | "r" | "f" | "]" | "d" | "J" | "K"),
```

`Snapshot` and `undo` are unchanged; `recorded` then snapshots tree + selection
before `move_key`, so AC 9 holds.

### `flex::view`

No production change. The moved node keeps the same `FlexBox` value and is
still selected (`selected == target`), so the existing highlight check
(`view.rs:257`) follows it automatically.

### Dependencies / collaborators

- `types::Tree::swap` — new; `types::Tree::next` / `previous` — reused for the
  neighbour and the boundary no-op.
- `parent_path` (`state.rs:304`) — not `Tree::parent`, which clamps top-level
  paths to themselves instead of returning `[]` (the canvas).
- `history::recorded` / `undoable` — snapshot policy only.
- No new crate dependencies; no reducer changes because Write/Replace precede
  the Move catch-all.

### Test plan

`types/src/tree.rs` (new):

- `swap_exchanges_two_top_level_siblings` and `swap_exchanges_two_nested_siblings`.
- `swap_keeps_each_subtrees_own_children` — a swapped node brings its subtree.
- `swap_with_the_same_path_changes_nothing`.
- `swap_panics_on_the_root_path` and `swap_panics_on_paths_with_different_parents`.

`flex::state` (new, mapped to ACs):

1. AC 1/3/4 — `j_swaps_the_selected_box_with_the_next_sibling_and_follows_it`:
   under a Column parent, `J` moves the box down one, its subtree travels, and
   `selected` becomes the new path.
2. AC 2 — `k_swaps_the_selected_box_with_the_previous_sibling`.
3. AC 5 — `j_on_the_last_sibling_changes_nothing` /
   `k_on_the_first_sibling_changes_nothing`: `boxes` and `selected` unchanged
   (history length grows by one per the decision above).
4. AC 6 — `j_and_k_under_a_row_parent_change_nothing`.
5. AC 7 — `j_and_k_on_the_canvas_change_nothing`.
6. AC 8 — `j_and_k_in_write_mode_are_typed` (appends to the label);
   `j_and_k_in_replace_mode_replace_the_text`.
7. AC 9 — `one_u_after_j_restores_the_order_and_the_selection`.
8. `j_reorders_top_level_boxes` (canvas counts as Column) and
   `j_reorders_a_selected_text_leaf`.

`flex::history`:

- Add `"J"`, `"K"` to the `box_text_and_toggle_keys_are_undoable_in_move_mode`
  list; the not-undoable list is unchanged (it holds lowercase `j`/`k` only).

### Merge note

Specs 226 (`a`/`A`) and 227 (`d`) are already merged to `main`, so the base
already has `cut_selected` (`state.rs:328`), the `clipboard` field, and a Move
`undoable` list of `"a" | "o" | "O" | "i" | "s" | "r" | "f" | "]" | "d"`. This
spec only **adds** a `move_key` arm and `"J" | "K"` list entries and removes
nothing, so it rebases cleanly on top of them.
