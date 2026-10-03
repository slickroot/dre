# o/O add a copy of the selected box after/before it

## User Story

Doug is arranging boxes in a column. He lands on one he likes the look of and presses `o`; a fresh copy of it appears right below, empty and selected, and he starts typing its label straight away. Later he wants one above instead, so he presses `O`, and a copy appears above the box he's on, also ready to type.

## Acceptance Criteria

1. In Move mode with a box selected, `o` inserts a new box immediately after the selected box among its siblings — at the next position, not appended at the end.
2. `O` inserts the new box immediately before the selected box among its siblings.
3. The new box copies the selected box's border, fill, and padding, and starts with empty text.
4. The new box becomes selected and dre-flex enters Write mode, so the next keystrokes fill its label.
5. For a top-level box, the new box is inserted among the top-level boxes (after for `o`, before for `O`).
6. In Write mode, `o` and `O` just type the letter and add nothing.
7. The old `o` — dropping an empty text inside the selected box — is gone.
8. Each press is undoable: one `u` removes the new box and restores the previous selection.

## Technical Design

### The two keys are one arm

`o` and `O` are sibling keys, so they become a single arm in `move_key`
(`src/flex/state.rs:242`). Both compute the selected box's parent and its
position within that parent, clone the selected box with empty text, and insert
the clone at the parent's index immediately after (`o`) or before (`O`):

```rust
"o" | "O" => {
    let new_box = copied_box(state.boxes.value(&state.selected));
    let parent = parent_path(&state.selected);
    let index = state.selected.last().copied().unwrap() + usize::from(key == "o");
    state.selected = state.boxes.insert(&parent, index, Tree::new(new_box, vec![]));
    state.mode = FlexMode::Write;
}
```

`Tree::insert` (`types/src/tree.rs:69`) is the existing general primitive; it
returns the new path, which becomes `state.selected` (criterion 4). It shifts
later siblings up, so the copy lands *at the next position*, not appended
(criterion 1) and just before the selected box for `O` (criterion 2). No new
`Tree` method is added.

### `parent_path`

`Tree::parent` (`types/src/tree.rs:38-44`) clamps a top-level path to itself, so
it cannot answer "who holds this box" for a top-level box — whose parent is the
window at `[]`. `move_key` reuses the `parent_path` helper the 222 movement keys
already added (`src/flex/state.rs:301`), rather than introducing a second one:

```rust
fn parent_path(path: &[usize]) -> Vec<usize> {
    if path.len() == 1 {
        Vec::new()
    } else {
        path[..path.len() - 1].to_vec()
    }
}
```

`Tree::insert` accepts the empty parent path, so `o`/`O` on a top-level box
insert among the top-level boxes (criterion 5). `state.selected` is never empty,
so the helper needs no other guard.

### `copied_box` is shared with Enter

The Enter flow already clones the selected box and empties its text
(`src/flex/state.rs:205-208`). That rule is extracted once and reused:

```rust
fn copied_box(b: &FlexBox) -> FlexBox {
    FlexBox {
        text: Some(String::new()),
        ..b.clone()
    }
}
```

It clones **all** style fields — `border`, `filled`, `padding`, `justify` and
`direction` — gives the new box empty text, and callers wrap it in a childless
`Tree::new(new_box, vec![])`. The `\r` arm is rewritten to use `copied_box` and
`parent_path` too, so there is a single copy rule. Criterion 3 is exactly the
`..b.clone()` plus `text: Some(String::new())`.

### Write mode is untouched

`reduce` (`src/flex/state.rs:139-146`) routes every non-`\r` key in `Write` (and
`Replace`) to `write_key` before the `Move` catch-all, so `o`/`O` in Write are
typed by the existing printable-character branch and add no box (criterion 6).
No new mode handling is needed.

### History records the insertion

`o`/`O` stay in the undoable Move list; add `"O"` alongside `"o"`
(`src/flex/history.rs:12`). `history::recorded` (`:18`) snapshots `boxes` and
`selected` **before** `move_key` runs, so one `u` after leaving Write restores
both the tree without the copy and the previous selection (criterion 8). Write
mode still does not record `u`; it types it, which is why the user leaves Write
(Esc or Enter) first, exactly as the existing `\r` flow does.

### The old `o` is deleted

The old arm (`src/flex/state.rs:274-277`) pushed an empty text leaf as a
**child**. It is replaced wholesale by the sibling insert, so
`is_droppable`/`drop_empty_text` keep working unchanged for borderless leaves.
`new_text()` (`:92-98`) had no caller other than the old `o` and its own test, so
it is deleted along with `new_text_is_a_borderless_leaf_with_empty_text`
(criterion 7). `new_box()` and `new_window()` are unchanged.

### No changes to `reduce`, `render` or `run_loop`

The signature `reduce(FlexState, &str)` (`:139`), `FlexScreen::render`
(`src/flex/mod.rs:18`) and `run_loop` (`:59`) are untouched. `new_boxes`
(`src/flex/mod.rs:44-57`) already marks any newly drawn bordered box, so the copy
grows in on screen with no renderer change. Nothing about layout or geometry is
involved.

### Existing tests

State tests that used the old `o` are rewritten around the new sibling semantics
or deleted; in particular:

- `o_in_move_mode_adds_an_empty_text_leaf_and_switches_to_write` (`:387`),
  `o_only_adds_a_text_leaf_to_the_selected_box` (`:476`),
  `o_adds_a_borderless_leaf_and_selects_it` (`:841`),
  `o_on_a_selected_text_nests_an_empty_text_inside_it_and_selects_it`
  (`:1466`), `o_on_a_selected_box_selects_the_new_text` (`:1491`),
  `o_only_adds_a_text_leaf_to_a_selected_box_above_the_bottom` (`:1234`) and
  `typing_after_o_goes_into_the_new_text` (`:403`) assert child text and become
  sibling-copy assertions.
- `o_after_enter_on_non_empty_text_types_into_the_new_sibling` (`:412`),
  `backspace_on_an_empty_new_text_removes_it_and_selects_the_parent` (`:431`),
  `backspace_empties_the_new_text_and_then_removes_it` (`:440`),
  `o_then_enter_leaves_the_tree_as_it_was_with_the_parent_selected` (`:717`),
  `o_then_enter_on_an_inner_box_selects_that_box` (`:724`),
  `enter_on_empty_text_drops_the_box_and_returns_to_move` (`:761`),
  `esc_with_empty_text_leaves_the_same_state_as_enter` (`:786`),
  `esc_after_o_with_text_keeps_the_new_text_and_returns_to_move` (`:821`),
  `backspace_on_an_empty_borderless_box_with_a_previous_sibling_selects_it`
  (`:449`), `u_after_o_typing_and_enter_is_typed_into_the_new_sibling_not_undone`
  (`:1537`) and
  `u_after_backspace_removes_an_empty_box_brings_it_and_its_selection_back`
  (`:1593`) are re-pointed at a box copy (e.g. starting from `["o", "x"]`) or
  dropped when the scenario no longer exists.
- `o_in_write_mode_is_typed_and_adds_no_text` (`:423`),
  `o_and_the_enter_add_flow_never_produce_replace` (`:691`) and
  `u_in_write_mode_is_typed_and_leaves_the_history_alone` (`:1562`) keep their
  meaning (criterion 6).
- `new_text_is_a_borderless_leaf_with_empty_text` (`:1031`) is deleted with
  `new_text()`.

History tests (`src/flex/history.rs:52-63`) gain `"O"` in the undoable list and
the "not undoable" list no longer needs it.

### View tests

The view tests are about rendering, so the five fixtures that drove `o` only to
build a tree are rebuilt directly from `holding`/`outer`/`text`/`new_window`
instead of by keystrokes:

- `an_empty_box_wears_its_own_colours` (`src/flex/view.rs:583`) uses
  `with_text("")` for the box-with-label structure it measures.
- `the_box_is_centred_again_after_a_text_is_added` (`:889`) uses
  `holding(FlexBox::default(), &["Hello", ""])`.
- `in_move_a_box_then_a_text_are_added_to_the_right_in_order` (`:1519`) builds
  the `text("Hello"), inner box, text("World")` row directly rather than
  `["\r", "A", "o", "World"]`.
- `HELLO_WITH_AN_INNER_BOX` (`:1789`) and
  `an_inner_box_added_after_filling_is_one_deeper_than_the_outer_box` (`:1808`)
  become direct fixtures of a box holding `text("Hello")` and a `new_box()`.
- `the_o_flow_leaves_no_replace_highlight` (`:2272`) keeps the `["o"]` sequence
  because it genuinely asserts the new `o` enters Write, not Replace, and so
  shows no highlight.

### Tests

Framework is inline `#[test]` with plain `assert!`/`assert_eq!`, no snapshots.
The new behavior lives in `src/flex/state.rs`:

- `o_inserts_a_copy_after_the_selected_box_among_its_siblings` — three stacked
  boxes, middle selected; `o` gives `Top, Middle, copy, Bottom` and selects
  `[2]`.
- `capital_o_inserts_a_copy_before_the_selected_box_among_its_siblings` — same
  setup; `O` gives `Top, copy, Middle, Bottom` and selects `[1]`.
- `o_and_capital_o_copy_border_fill_padding_justify_and_direction` — style the
  selected box (`f`, `]`, `r`, `s`) first, then assert every field matches and
  `text` is `Some("")` with no children.
- `o_on_a_top_level_box_inserts_among_the_top_level_boxes` — `o` on `[0]`
  selects `[1]`; `O` on `[0]` selects `[0]` and pushes the original to `[1]`.
- `o_and_capital_o_on_a_nested_box_insert_a_sibling_within_that_parent` — the
  copy shares the selected box's parent and never becomes top-level.
- `o_and_capital_o_enter_write_mode_so_the_next_keys_fill_the_new_box` —
  criterion 4.
- `o_and_capital_o_in_write_mode_type_themselves_and_add_no_box` — criterion 6.
- `one_u_after_o_removes_the_copy_and_restores_the_selection` and the `O` twin —
  press the key, leave Write, one `u`; criterion 8.

History tests add `o` and `O` assertions for `undoable(FlexMode::Move, _)`.

### Rejected

- **Push the copy as a child (the old `o`)** — that is the behavior criterion 7
  removes; it mixes "add a label" into the sibling axis.
- **Append the copy at the end of the parent** — fails criterion 1; it would not
  appear "right below" the box.
- **A new `Tree::insert_before`/`insert_after`** — `Tree::insert` already
  expresses the operation; only the parent path was missing, and a local helper
  supplies it.
- **Copy only `border`/`fill`/`padding`** — leaves `justify`/`direction`
  silently reset, so a copied row or spaced box would not look like its source.
- **Deep-clone the subtree** — the story says "empty and ready to type"; copying
  children would duplicate content the user did not ask for.
- **Undo from Write mode** — would break the long-standing rule that `u` types
  in Write and would touch the replace/history tests for no gain.
