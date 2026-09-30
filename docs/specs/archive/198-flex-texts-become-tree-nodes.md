# Flex: texts become tree nodes

## Refactoring Goal

A box's texts and its inner boxes live in two places today: `FlexBox.texts` and the `Tree<FlexBox>` children. Nothing records how they interleave, so spec 196 (one row, in the order they were added) and spec 199 (select a text with `l`) cannot be built. After this refactor every child of a box is a tree node, `Text` or `Box`, in the order it was added. There is no user story: nothing changes on screen or on the keyboard.

## Technical Design

Decisions:

- **Nothing user-visible changes.** Every existing test keeps its meaning. Tests that read `texts` are rewritten to read the new nodes, and no assertion about behaviour is relaxed.
- **Data model** (`src/flex/state.rs`):
  ```rust
  pub(crate) enum FlexNode { Text(String), Box(FlexBox) }
  pub(crate) struct FlexBox { width: FlexWidth, justify: Justify, filled: bool } // no more `texts`
  ```
  `FlexState.boxes` becomes `Tree<FlexNode>`. `FlexNode` implements `Default` as an empty `Box`, which `Tree::root` needs.
- **`new_box()`** builds `Tree::new(Box(FlexBox::default()), vec![Tree::leaf(Text(""))])`. The first box and every box added with `a` or `A` use it.
- **`selected` is still always a box path.** `j`/`k` are unchanged. `s` does not change `selected`. Texts become selectable in spec 199.
- **Keys:**
  - `s` appends a `Text("")` node as the last child of the selected box and switches to WRITE.
  - `A` appends `new_box()` as the last child of the selected box.
  - WRITE typing and backspace act on the selected box's last `Text` child.
- **Queries:**
  - `outer_boxes()` keeps its name and yields only `Box` nodes. The root's children are always boxes.
  - New `FlexState::texts_of(path) -> Vec<&str>` returns the `Text` children of a box, in order. The view and the `mod.rs` tests use it.
- **`view.rs` keeps today's layout.** It reads a box's texts with `texts_of` and counts inner boxes by filtering children to `Box` nodes. `place_outer_box` is otherwise untouched. Spec 196 replaces it.
- **`src/flex/mod.rs`:** the four `texts[0]` assertions become `texts_of(&[index])[0]`.

### Testing plan

1. `new_box()` and `FlexNode`. `a_default_box_has_exactly_one_empty_text` and `starts_in_write_mode_with_exactly_one_empty_box` are rewritten against the new nodes.
2. `s`, `A` and WRITE typing are rewritten to read `texts_of`. They pass with the same expectations.
3. The view tests run unchanged, apart from the helper that builds a box with a given text.

### Out of scope

- Any change to layout, keys or colours.
- Selecting a text (spec 199).
