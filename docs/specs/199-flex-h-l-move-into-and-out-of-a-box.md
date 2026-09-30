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

Depends on spec 198 (texts are tree nodes). All changes are in `src/flex/state.rs` and `src/flex/view.rs`.

### State (`FlexState`)

- **`selected_box_path(&self) -> Vec<usize>`** returns `selected` when that node is a `Box`, and `boxes.parent(&selected)` when it is a `Text`. Every key that acts on a box resolves its target through this.
- **`selected_box(&mut self)`** looks up `selected_box_path()` instead of `selected`, so it can no longer hit its `unreachable!` when a text is selected.
- **`selected_text(&mut self)`** returns the selected node when it is a `Text`. When a box is selected it falls back to the box's last `Text` child, as it does today.

### Keys

- **MOVE `l`**: `state.selected = state.boxes.child(&state.selected)`. `Tree::child` returns the path unchanged for a node with no children.
- **MOVE `h`**: `state.selected = state.boxes.parent(&state.selected)`. `Tree::parent` returns the path unchanged for an outer box.
- **`j` / `k`**: no change. `Tree::next` and `Tree::previous` already move between siblings of any kind and stop at the ends.
- **`w`, `g`, `f`, `d`**: toggle the box at `selected_box_path()` via `selected_box()`. `d` is included even though the acceptance criteria predate it (spec 197).
- **`A`**: pushes a new box onto `selected_box_path()`. The selection does not change.
- **`s`**: appends an empty text to `selected_box_path()`, **selects the new text** (`state.selected = state.boxes.push(..)`) and switches to WRITE. It always appends to the end of the box and never inserts after the selected text. Tests that read `texts_of(&state.selected)` after `s` change to read the box's path.
- **WRITE**: typing and backspace go through `selected_text()`. `h` and `l` go through the existing `printable_char` branch, so they are typed.

### View (`paint`)

- A `FlexNode::Text` label gets `FLEX_SELECTED_COLOUR` when `mode == Move && state.selected == path`, and `FLEX_TEXT_COLOUR` otherwise.
- Box borders need no change. `paint` already colours only the box whose path equals `selected`, so a box whose text is selected keeps `FLEX_BORDER_COLOUR`, and a selected inner box gets `FLEX_SELECTED_COLOUR`.
