# Flex: text is a borderless leaf box

## Refactoring Goal

`dre-flex` has two kinds of node: `FlexNode::Text(String)` and `FlexNode::Box(FlexBox)`. The playground has just one. A "text" there is a box with no border and no children. Most of the coming flex stories assume the playground's model, so every one of them would otherwise have to handle both kinds of node.

After this refactor, the tree is `Tree<FlexBox>`. Every box can show its own text, and a text is a borderless box with no children. Nothing changes on screen except the behaviour listed under Findings 4 and 5.

## Findings

Problems in today's `src/flex` that this refactor fixes. Each one is removed, not moved somewhere else.

1. **Code everywhere checks which kind of node it has.** `measure`, `arrange`, `paint`, `scene`, `new_boxes`, `outer_boxes`, `texts_of` and the `p` key all match on `FlexNode` (44 uses in `state.rs`, 26 in `view.rs`). *Fixed by:* one node type, `FlexBox`. `FlexNode` is deleted.
2. **A selected text sends its keys to the parent box.** `selected_box_path` swaps a selected `Text` for its parent, and `selected_text_path` searches a box's children for its last `Text`. *Fixed by:* there is no "text selection". You select a box, and every key acts on `state.selected`. `selected_box_path`, `selected_box`, `selected_text_path`, `selected_text` and `texts_of` are deleted.
3. **`bare` packs four meanings into one flag.** It means no border, no padding, not painted, and not animated. Only the window root uses it. *Fixed by:* `border: bool`, which has one set of rules shared by the window and text leaves.
4. **Empty texts pile up.** `s` followed by `\r` leaves an empty `Text` in the tree. *Fixed by:* pressing `\r` on empty text removes it (see Decisions).
5. **A bordered box can't show text unless you add a child.** `i` creates a `Text` child. *Fixed by:* `i` writes into the selected box's own `text`.

## Technical Design

Decisions:

- **One node type** (`src/flex/state.rs`):
  ```rust
  pub(crate) struct FlexBox {
      pub(crate) text: Option<String>,
      pub(crate) border: bool,      // replaces `bare`, opposite meaning
      pub(crate) justify: Justify,
      pub(crate) direction: Direction,
      pub(crate) filled: bool,
      pub(crate) padding: u16,
  }
  ```
  - `Default` is `text: None, border: true`, with everything else as today. It is written by hand, because `border` defaults to `true`.
  - `FlexBox::window()` is `border: false`, `Column`, `Center`, `text: None`.
  - `new_box()` returns a default box. A new `new_text()` returns a leaf with `border: false, text: Some(String::new())`.
  - `FlexState.boxes` becomes `Tree<FlexBox>`, and `Snapshot.boxes` in `history.rs` follows.
- **What `text` means.**
  - `None`: the box has no text item.
  - `Some(s)`: the text is a flow item, even when `s` is empty, so the text you are typing always has a slot.
  - Layout only looks at `is_some()`, so `measure` and `arrange` stay pure functions of the tree. They don't need to know the mode or the selection.
- **The text is the first item in the flow.** A box's items are its text (when `Some`) followed by its children. The text item measures `interior(text) × 1` and goes along `direction` like any child, with the usual `FLEX_SPACE` gap after it. In a column it stretches across the cross axis like the other items. In a row it takes its share of the width.
- **`border` rules.**
  - **Borderless:** no padding (`padding()` returns 0, so `p` has no visible effect), no `PlacementNode::Box`, and it never grows. `new_boxes` keeps filtering on `border`.
  - **Bordered:** paints its `PlacementNode::Box` as today, with `FLEX_BORDER` and padding.
  - **Both:** the box paints its text, when `Some`, as a `Label` at the text item's rect.
  - **Window root:** a borderless box with `text: None`, so it paints nothing. The `bare` check in `scene()`'s `filter_map` is deleted.
- **Selection colour.**
  - A selected bordered box has the `FLEX_SELECTED_COLOUR` border, and its text keeps `FLEX_TEXT_COLOUR`.
  - A selected borderless box has no border, so its label is painted in `FLEX_SELECTED_COLOUR`, as a selected `Text` is today.
- **`arrange` reports where the text goes.** It now outputs `Vec<Arranged>`:
  ```rust
  struct Arranged { path: Vec<usize>, rect: Rect, text: Option<Rect> }
  ```
  - `text` is the rect of the box's text item.
  - For a borderless leaf, `text` covers the whole box.
  - `paint` returns zero, one or two `Placement`s per entry: the border and/or the label.
- **Keys** (`move_key` / `write_key`). All keys act on `state.selected`.
  - `i`: if `text` is `None`, set it to `Some(String::new())`. Then enter Write.
  - `s`: push `new_text()` into the selected box, select it, and enter Write.
  - `A`: push `new_box()` into the selected box. On a text leaf, this nests the new box inside the text leaf, after its text.
  - `g`, `d`, `f`, `p`: toggle or bump the selected box's own field. The `if let FlexNode::Box` guard on `p` is removed.
  - Write `\x7f` and printable keys: edit the selected box's `text` (it is always `Some` in Write).
  - Write `\r` when `text == Some("")`:
    - On a borderless box with no children: remove the box and select its parent. This uses the existing `Tree::remove(path)`.
    - Otherwise: set `text` back to `None`, so a bordered box drops its empty text item.
    - Then enter Move.
  - Write `\r` with non-empty text: enter Move, as today.
- **`outer_boxes`** loses its `filter_map` and returns every outer box.

### Collaborators

- `types::Tree<FlexBox>`: no change. `remove`, `push`, `parent` and `children` already exist.
- `src/flex/history.rs`: `Snapshot.boxes` changes type. `undoable` keys are unchanged.
- `src/flex/mod.rs`: `new_boxes` filters on `border` instead of `bare: false`.
- `view::{Placement, PlacementNode, Label, interior}`: no change.

### Testing plan (TDD, thin slices)

1. **Model swap, no behaviour change.**
   - Replace `FlexNode` with `FlexBox { text, border }`.
   - In tests, `text("Hi")` becomes a borderless leaf with `Some("Hi")`.
   - All current view tests keep their assertions.
2. **Own text in the flow.**
   - A bordered box with `text: Some("Title")` and two inner boxes, in a column, places the label first with one `FLEX_SPACE` gap before the inner boxes.
   - The same box in a row places the label first along the row.
   - A box with `text: None` lays out exactly as before.
3. **Painting.**
   - A selected borderless leaf paints its label in `FLEX_SELECTED_COLOUR` and paints no box.
   - A selected bordered box with text paints a blue border and a `FLEX_TEXT_COLOUR` label.
   - The window root paints nothing.
4. **Keys on the selected box.**
   - `i` on a bordered box writes into its own text, and no child is created.
   - `s` adds a borderless leaf and selects it.
   - `A` on a text leaf nests a box inside the leaf.
   - `d` on a text leaf toggles the leaf's direction, not its parent's.
5. **Empty text.**
   - `s` then `\r` leaves the tree as it was before `s`, with the parent selected.
   - `i` then `\r` on a bordered box leaves `text: None`.
   - While writing empty text, the item has a 0-width, 1-high slot.
6. **Undo.** `s`, type, `\r`, then `u` brings back the previous tree. History snapshots still work with `Tree<FlexBox>`.
