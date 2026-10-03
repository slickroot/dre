# Flex: h j k l walk the children of the box you are inside

## User Story

Doug has three boxes stacked down the screen and a fourth nested inside the top
one. He is on the top box and wants the one below it. He does not want to count
children or hold the shape of the tree in his head, and he does not want to
point at the screen either: an earlier attempt that picked the box nearest in a
screen direction kept stealing his move sideways, because a box up and to the
side had a nearer corner than the box directly above.

He wants the rule to be about the container he is inside, not the pixels. If he
is inside a column of boxes, `j` is the next one down and `k` the one above. If
he is inside a row, `l` is the next one right and `h` the one left. The key that
does not belong to the container's axis does nothing. `Enter` goes one level in
and `Backspace` comes one level out, so depth is always deliberate.

## Acceptance Criteria

1. Under a `Column` parent, `j` selects the next sibling and `k` the previous
   sibling.
2. Under a `Row` parent, `l` selects the next sibling and `h` the previous
   sibling.
3. A key that does not match the parent's axis does nothing: `h` and `l` under a
   `Column`, `j` and `k` under a `Row`.
4. Top-level boxes are children of the window, which is a `Column`, so `j` and
   `k` walk them.
5. At the first sibling `k`/`h` does nothing; at the last sibling `j`/`l` does
   nothing.
6. `Enter` in move mode selects the first child, or does nothing if the box has
   no children.
7. `Backspace` in move mode selects the parent, or does nothing if the box is an
   outer box.
8. `Enter` and `Backspace` are move-mode keys only. Typing mode is unchanged.
9. The selection never depends on the window size: the same tree gives the same
   move at every size.
10. Moving the selection is not undoable, exactly as it is not today.

## Technical Design

### No layout, no geometry

An earlier version of this spec threaded a freshly arranged `Layout` from
`FlexScreen::render` down into `state::reduce`, then asked it for the box nearest
in a screen direction. That is rejected. The move is a fact about the tree, not
about the tree at a size, so the whole `layout.rs` extraction, the `Layout`
value, `Heading` and `nearest` are dropped. `render` keeps returning
`io::Result<()>`, `reduce` keeps its `(state, key)` signature, and `run_loop`
(`src/flex/mod.rs:59-74`) is untouched.

### `move_key` reads the parent's `Direction`

`Direction` (`src/flex/state.rs:35`) already says whether a box lays its children
out as a `Row` or a `Column`. The four navigation keys become one arm that reads
the **parent's** direction and picks the pair of tree moves that matches it:

```rust
"j" | "k" | "h" | "l" => {
    let parent = parent_path(&state.selected);
    let axis = state.boxes.value(&parent).direction;
    state.selected = match (axis, key) {
        (Direction::Column, "j") => state.boxes.next(&state.selected),
        (Direction::Column, "k") => state.boxes.previous(&state.selected),
        (Direction::Row, "l") => state.boxes.next(&state.selected),
        (Direction::Row, "h") => state.boxes.previous(&state.selected),
        _ => state.selected.clone(),
    };
}
"\r" => state.selected = state.boxes.child(&state.selected),
"\x7f" => state.selected = state.boxes.parent(&state.selected),
```

`Tree::next` (`types/src/tree.rs:46`) and `Tree::previous` (`:55`) clamp at the
ends of the sibling list, which is criterion 5 with no guarding code.

### Why the parent path is not `Tree::parent`

`Tree::parent` (`types/src/tree.rs:38`) deliberately clamps a top-level path to
itself, because the root has no value to edit. That is exactly wrong here: a
top-level box's parent is the window at `[]`, and the window is a box whose
direction decides the keys. So `move_key` computes the parent path directly:

```rust
fn parent_path(path: &[usize]) -> Vec<usize> {
    if path.len() == 1 {
        Vec::new()
    } else {
        path[..path.len() - 1].to_vec()
    }
}
```

`Tree::value` accepts the empty path (`types/src/tree.rs:79`), so
`state.boxes.value(&parent)` works for the window.

### The window is a Column and stays one

`FlexBox::default` sets `direction: Direction::default()`, which is `Column`
(`src/flex/state.rs:66`), and `FlexBox::window()` (`:74`) keeps that default. The
window can never be selected (there is no key that selects `[]`; `Backspace`
clamps at the outer boxes), so `r`, which toggles the selected box's direction
(`src/flex/state.rs:273`), can never turn the window into a `Row`. Top-level
navigation is therefore always `j`/`k`, which is criterion 4.

### Enter and Backspace stay structural

They never needed geometry. `Tree::child` (`types/src/tree.rs:23`) returns the
same path for a childless box and `Tree::parent` (`:38`) returns the same path
for an outer box, so criteria 6 and 7 need no guard. Both keys already reach
`move_key` only in move mode: `reduce` (`src/flex/state.rs:139-146`) routes
`"\r"` in `Write` to `write_key` before the catch-all move arm, and `"\x7f"` is
handled inside `write_key`. Typing mode is unchanged, which is criterion 8.

### History is untouched

Movement keys are absent from `undoable`'s whitelist
(`src/flex/history.rs:10-16`), and `"\r"` and `"\x7f"` are not listed for
`FlexMode::Move`. So none of the new keys is recorded and criterion 10 holds for
free. The existing
`selection_undo_and_move_keys_are_not_undoable_in_move_mode`
(`src/flex/history.rs:59`) already lists `h`, `j`, `k`, `l` and stays true.

### Existing tests

The `moved` and `typed` helpers keep their call shape; nothing about `reduce`
changes. But the meaning of `l` and `h` changes, so the tests that used them to
descend and climb are re-pointed at `"\r"` and `"\x7f"`:

- `l_in_move_mode_selects_the_first_child` (`:1355`) becomes an `"\r"` test.
- `h_in_move_mode_selects_the_parent` (`:1387`) becomes a `"\x7f"` test.
- The `hello_box_world` (`:1340`) and `world_selected` (`:1431`) helpers build
  navigation out of `["l", "j", ...]` and `["h", ...]`; every caller switches to
  `["\r", "j", ...]` and `["\x7f", ...]`.
- `l_on_a_text_leaf_leaves_the_state_unchanged` (`:1365`),
  `l_on_an_empty_inner_box_leaves_the_state_unchanged` (`:1371`) and
  `l_on_an_inner_box_holding_a_text_selects_that_text` (`:1377`) asserted the old
  child meaning; the first two now assert that `l` is a no-op under a `Column`,
  and the third is rewritten around `"\r"`.
- `j_and_k_move_between_texts_and_boxes_and_stop_at_the_ends` (`:1410`) still
  holds, because the box it walks is a `Column`.
- `j_and_k_after_capital_a_only_move_between_outer_boxes` (`:1317`) still holds,
  because the window is a `Column`.

### Tests

Framework is inline `#[test]` with plain `assert!`/`assert_eq!`, no snapshots.
All in `src/flex/state.rs`, since the rule lives there and no geometry is
involved:

- `j_and_k_move_between_siblings_under_a_column_parent`
- `h_and_l_move_between_siblings_under_a_row_parent`
- `j_and_k_do_nothing_under_a_row_parent`
- `h_and_l_do_nothing_under_a_column_parent`
- `top_level_boxes_walk_with_j_and_k`
- `sibling_moves_stop_at_the_ends`
- `enter_selects_the_first_child_and_stays_on_a_childless_box`
- `backspace_selects_the_parent_and_stays_on_a_top_level_box`
- `enter_and_backspace_type_themselves_in_write_mode`
- `moving_the_selection_is_not_undoable`

### Rejected

- **Layout in `reduce` (the previous design)** — an arranged `Layout` threaded
  from `render` into `reduce`, with a `Heading` and a `nearest` that compared
  top-left corners. It made a box up and to the side win over the box directly
  above whenever its corner was nearer, which is the behaviour that prompted this
  rewrite, and it dragged a `layout.rs` module split and a `state ↔ layout` cycle
  in behind it.
- **Overlap or weighted-distance scoring** — keeps the geometry and the
  `render`-returns-`Layout` plumbing; it only trades one surprising diagonal for
  another.
- **`j`/`k` always siblings and `h`/`l` always parent/child** — the pre-222
  behaviour. It ignores the container's axis, so `h`/`l` and `j`/`k` mean the
  same thing at every level and the direction on screen is lost.
- **Falling back to a sibling move when the key misses the axis** — makes all
  four keys do the same thing, which erases the reason to bind them to the axis
  at all.
