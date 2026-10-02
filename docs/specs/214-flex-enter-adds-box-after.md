# Flex: Enter while writing adds a box after it

Lina is in Write mode typing "DRE" into a box. She presses Enter. A new, empty box appears immediately after the current one — in whichever direction the parent lays out (right for Row, below for Column) — and she's instantly in Write mode on that new box, able to keep typing without pressing any other key.

## Acceptance Criteria

- Pressing Enter while writing non-empty text commits that text to the current box.
- A new empty box is created as the next sibling, inserted immediately after the current box (not appended to the end of the parent's children).
- The new box is selected and Write mode stays active, so typing continues immediately in the new box.
- The new box's position follows the parent's existing direction (beside it for Row, below it for Column) — Enter never changes the parent's direction.
- Pressing Enter on an empty box still drops that box and returns to Move mode on the parent (unchanged from today — spec 182).

## Technical Design

### `Tree::insert` (types/src/tree.rs)

`Tree::push` currently only appends a child to the end of a parent's children list, which isn't enough to insert a box right after the current one. Add a general insertion primitive and reimplement `push` in terms of it:

```rust
pub(crate) fn insert(&mut self, parent: &[usize], index: usize, child: Tree<T>) -> Vec<usize> {
    // insert `child` at `index` in parent's children, returns its path
}

pub(crate) fn push(&mut self, parent: &[usize], child: Tree<T>) -> Vec<usize> {
    let index = self.children(parent).len();
    self.insert(parent, index, child)
}
```

### New box creation (src/flex/state.rs)

In `write_key`, the `"\r"` arm currently only handles the empty-text case (`drop_empty_text`). Extend it:

- Empty text (`Some("")`): unchanged — `drop_empty_text` runs, mode returns to `Move`.
- Non-empty text: clone the current `FlexBox` entirely (border, filled, padding, justify, direction — everything), reset `text` to `Some(String::new())` on the clone, and insert it as the next sibling immediately after the current box. Since `state.selected` is itself a path of child indices, the current box's index within its parent is just `*state.selected.last().unwrap()`; the parent path is `state.selected` minus its last element. Insert at `index + 1` via `Tree::insert`, then set `state.selected` to the new box's path. Mode stays `Write`, so typing continues immediately.

```rust
"\r" => {
    if state.selected_mut().text.as_deref() == Some("") {
        state = drop_empty_text(state);
    } else {
        let mut new_box = state.selected_mut().clone();
        new_box.text = Some(String::new());
        let parent = &state.selected[..state.selected.len() - 1];
        let index = state.selected[state.selected.len() - 1] + 1;
        state.selected = state.boxes.insert(parent, index, Tree::new(new_box, vec![]));
        return (state, None); // stays in Write mode
    }
    state.mode = FlexMode::Move;
}
```

(Exact shape to be refined during implementation; the point is: full-field clone of the box, reset text, insert right after via the new `Tree::insert`, select the new path, keep mode Write.)

No new box "kind" concept is introduced — direction/justify/padding are copied even though they have no visible effect on an empty leaf yet, since they only matter once the box gains children.

### Undo

Every Enter press in Write mode becomes its own undo step — both the drop-empty-box branch and the new-box-creation branch. This is a change from today's behavior, where only `move_key` presses (`a|A|o|i|s|r|f|]`) are snapshotted and all Write-mode keystrokes (including Enter) are absorbed into the single undo step that started the write session.

Concretely: `reduce` must route the `"\r"` key through `history::recorded` even while in `FlexMode::Write` (today only `FlexMode::Move` keys go through `history::recorded`), and `"\r"` needs to be added to the undoable key set used there. Plain character typing and backspace remain un-snapshotted, so undo still rewinds a whole in-progress word at once — it's only Enter that now draws a line.

### Edge cases

A box in Write mode is assumed to always have a parent (writing only ever happens on a non-root box), so `state.selected` always has at least one element and the parent/index computation above is always valid. The root/window case is not handled.
