# Flex: j and k select a box to style in MOVE

## User Story

Noor has three boxes stacked in `dre-flex`: "Hello", "World" and "Again". They're in MOVE, and the last box they added, "Again", has a `#8AB4F8` border. Noor presses `k` twice, and the blue border moves up to "Hello". They press `w`, and "Hello" goes full width. Then they press `f`, and "Hello" gets filled. The other boxes stay as they were. Noor presses `j`, and the border moves down to "World". They press `a`, and a new empty box appears at the bottom of the stack with the blue border. When Noor presses `i` to type, the blue border goes away.

## Acceptance Criteria

- In MOVE, the selected box has a `#8AB4F8` border.
- In WRITE, no box has the `#8AB4F8` border.
- In MOVE, `j` selects the box below and `k` selects the box above.
- With the bottom box selected, `j` keeps the selection there. With the top box selected, `k` keeps the selection there.
- `w`, `f`, `g`, `s` and `i` act on the selected box. The other boxes don't change.
- `a` adds the new box at the bottom of the stack, and the new box becomes the selected one.
- In WRITE, `j` and `k` type a "j" and a "k" and don't change the selection.

## Technical Design

### State (`src/flex/state.rs`)

- `FlexState` gains `selected: usize`, an index into `boxes`. `Default` sets it to `0`, the only box.
- `newest_box()` is renamed `selected_box()` and returns `&mut self.boxes[self.selected]`. `newest_text()` is renamed `selected_text()` and returns the last text of the selected box.
- `w`, `f`, `g`, `s` and WRITE typing/backspace already go through these helpers, so after the rename they act on the selected box. Their own code doesn't change.
- `move_key`:
  - `"a"` pushes a `FlexBox::default()` (still at the bottom), then sets `selected = boxes.len() - 1`.
  - `"j"` sets `selected = (selected + 1).min(boxes.len() - 1)`, so it stops at the bottom box.
  - `"k"` sets `selected = selected.saturating_sub(1)`, so it stops at the top box.
- `write_key` doesn't change. `j` and `k` reach the printable branch, get typed, and leave `selected` alone.
- A `selected: bool` flag on `FlexBox` was rejected because it allows zero or several selected boxes.

### View (`src/flex/view.rs`)

- New constant `FLEX_SELECTED_COLOUR: Rgb = (0x8A, 0xB4, 0xF8)`.
- `scene` picks each box's border colour inline in its `(i, flex_box)` loop: `FLEX_SELECTED_COLOUR` when `state.mode == FlexMode::Move && i == state.selected`, otherwise `FLEX_BORDER_COLOUR`.
- Whether the selection is shown is a display rule, so it stays in the view. `FlexState` only knows which box is selected. A `FlexState::highlighted()` method was rejected because only this one place would use it.

### Tests

- State: `j`/`k` move `selected` down/up. `j` on the bottom box and `k` on the top box leave it unchanged. `a` selects the new bottom box. `w`/`f`/`g`/`s` and typing after `i` change only the selected box. In WRITE, `j`/`k` append "j"/"k" to the selected text and leave `selected` unchanged.
- View: in MOVE, the selected box's border is `#8AB4F8` and the other borders are `FLEX_BORDER_COLOUR`. In WRITE, every border is `FLEX_BORDER_COLOUR`.
