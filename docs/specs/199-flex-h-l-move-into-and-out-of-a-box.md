# Flex: h and l move into and out of a box in MOVE

## User Story

Noor has a box showing `Hello [box] World` and they're in MOVE with the outer box selected. They press `l`. The first child, "Hello", is selected and lit up. They press `j` and the inner box is selected, then `j` again and "World". They press `h` and the outer box is selected again. With "World" selected they press `i` and type "!", and it reads "World!". With "World" selected they press `w`, and the outer box goes full width.

## Acceptance Criteria

- In MOVE, `l` selects the selected node's first child. On a node with no children it changes nothing.
- In MOVE, `h` selects the selected node's parent. On an outer box it changes nothing.
- `j` and `k` move between siblings, texts and boxes alike, and stop at the ends.
- A selected text is drawn in the selected colour. The box around it keeps its normal border colour.
- A selected inner box gets the selected border colour.
- With a text selected, `w`, `g`, `f`, `s` and `A` act on the text's parent box.
- With a text selected, WRITE types into that text and backspace deletes from it.
- With a box selected, WRITE still types into the box's last text.
- In WRITE, `h` and `l` are typed.

## Technical Design

Depends on spec 198.

- **Keys** (`move_key`): `l` → `state.selected = state.boxes.child(&state.selected)`, `h` → `state.boxes.parent(&state.selected)`. `Tree::child` and `Tree::parent` already return the path unchanged when there is nowhere to go.
- **`selected_box_path()`** returns `selected` if that node is a `Box`, and its parent if it's a `Text`. `w`, `g`, `f`, `s` and `A` use it instead of `selected` directly.
- **`selected_text()`** returns the selected node if it's a `Text`, otherwise the last `Text` child of the selected box. WRITE typing and backspace use it.
- **View:** `place_outer_box` picks the label colour from `state.selected`, so a selected text gets `FLEX_SELECTED_COLOUR`. The outer box highlight applies only when the box itself is at `selected`. Inner box colours are handled in spec 196's layout.
- Not decided yet, to revisit later: whether box-level keys should resolve to the parent when a text is selected, or do nothing.
