# Doug adds side padding to a box with p

## User Story

Doug has a box in dre-flex with the text "Hello". The text feels squeezed against the sides. In move mode he selects the box and presses `p`. The box gets one more unit of space on its left and right, and "Hello" now sits 4 columns in from each side. Happy, he carries on drawing!

## Acceptance Criteria

- In move mode, `p` adds one unit (2 columns) of padding on the left and right of the selected box.
- The top and bottom padding stays at 1 row.
- Pressing `p` again keeps adding one unit each time, with no limit.
- `u` undoes one `p`.
- `p` works on inner boxes too, and the parent box grows to make room.
- If a text is selected, `p` does nothing.
- In write mode, `p` just types a "p".
- A full-width box keeps its width, and its content gets the extra room on the inside.

## Technical Design
All changes are in `src/flex/state.rs`, `src/flex/history.rs` and `src/flex/view.rs`. Each box stores its own extra side padding. The layout already sends all padding through one function, `padding()`, so growing the parent and keeping a full-width box's width both follow from the existing rules.

### State (`state.rs`)

- **`FlexBox`** gains `padding: u16`, the number of side units added *on top of* the standard one. The derived `Default` makes it 0, so `FlexBox::default()`, `new_box()` and `FlexBox::window()` stay the same.
- **`p` in move mode** adds one to `padding` (`saturating_add(1)`, so there's no limit in practice) only when the selected node is a `FlexNode::Box`. It doesn't go through `selected_box()`, which would reach the parent of a selected text. When a text is selected, `p` does nothing.
- **`p` in write mode** needs no change: `write_key` already types it.

### History (`history.rs`)

- `"p"` joins the key list in `undoable`, so `u` undoes one `p`.
- History stays key-based. A `p` on a selected text still records a snapshot, so the next `u` changes nothing on screen. This is accepted, and matches `i` on a box that already holds text.

### View (`view.rs`)

- **`padding(flex_box)`** returns `width: FLEX_SPACE.width * (1 + padding)` and `height: FLEX_SPACE.height`. A `bare` box still gets no padding.
- No other layout changes:
  - `measure` adds the padding to the box's size, so a padded inner box makes its parent grow.
  - `arrange` uses `rect.inner(padding(..))`, so a `Full` box keeps the width the stretch rule gives it, and the extra padding takes room from its content.

### Tests

State:
- `p` on a selected box raises its `padding` from 0 to 1, and a second `p` raises it to 2.
- `p` on a selected text leaves `boxes` unchanged.
- `p` in write mode types a "p" and leaves `padding` at 0.
- `p` then `u` gives back the starting boxes.
- `"p"` is in the undoable keys for move mode and not for write mode (extend the existing lists in `history.rs`).

View (one test per acceptance criterion):
- After `p`, "Hello" sits `2 * FLEX_SPACE.width` from the box's left and right edges, and the box is `2 * FLEX_SPACE.width` wider. The top and bottom gap stays `FLEX_SPACE.height`.
- After `p` twice, the box is `4 * FLEX_SPACE.width` wider than before.
- `p` on an inner box makes the inner box and its parent both `2 * FLEX_SPACE.width` wider.
- `p` on a `Full` outer box keeps its width, and its text starts `2 * FLEX_SPACE.width` in from its left edge.
