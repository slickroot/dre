# Flex: texts and inner boxes sit in one row, in the order they were added

## User Story

Noor runs `dre-flex`, types "Hello" and presses Enter, so they're in MOVE. They press `A`, and an empty inner box appears to the right of "Hello", not below it. They press `s` and type "World", and "World" appears to the right of the inner box. The outer box now shows `Hello [box] World` in one row. "Hello" and "World" are centred vertically on the inner box's middle row. Noor presses Enter, then `w` and `g`. "Hello" moves to the left edge, "World" moves to the right edge, and the inner box sits between them with the same space on each side.

## Acceptance Criteria

- Texts and inner boxes are siblings in a single row, in the order Noor added them.
- In MOVE, `A` adds an inner box to the right of the last sibling.
- In MOVE, `s` adds a text to the right of the last sibling.
- There's a 1-cell gap between neighbouring siblings.
- Texts are centred vertically against the tallest sibling.
- The outer box grows to fit the row and stays centred on the screen.
- With `g` on and the box full width, all siblings are spread out: the first sits at the left edge, the last at the right edge, and every gap is the same size.
- Typing in WRITE still goes into the selected box's last text.

## Technical Design
### Data model (`src/flex/state.rs`)

- Texts become tree nodes, so the order of a box's children is the order Noor added them.
  ```rust
  pub(crate) enum FlexNode { Text(String), Box(FlexBox) }
  pub(crate) struct FlexBox { width: FlexWidth, justify: Justify, filled: bool } // no more `texts`
  ```
- `FlexState.boxes` becomes `Tree<FlexNode>`. The root's children are always `Box` nodes, so `outer_boxes()` keeps working and only matches `Box` nodes.
- A new box is built by one helper, `new_box()`. It is a `Box(FlexBox::default())` node with one child, `Text("")`. The first box and every box added with `a` or `A` use it.
- `selected` is a path to any node, a box or a text. `j`/`k` keep using `Tree::next`/`previous` unchanged, so they walk every node, texts included.
- `selected_box_path()` returns `selected` if that node is a `Box`, and its parent if it's a `Text`.

### Key handling (MOVE and WRITE)

- `w`, `g`, `f` act on the box at `selected_box_path()`.
- `A` appends `new_box()` as the last child of that box. `selected` does not change.
- `s` appends a `Text("")` node as the last child of that box, selects it and switches to WRITE.
- `a` still appends a new outer box and selects it.
- WRITE types into `selected` if it's a `Text`. If a box is selected, it types into that box's last `Text` child.
- Backspace follows the same target rule as typing.
- Not decided yet, to revisit later: what should happen when a text is selected and a box-level key is pressed. For now they resolve to the parent box.

### Layout (`src/flex/view.rs`)

`place_outer_box`, `text_offsets`, `centring_shift` and `inner_x` are replaced by two passes over `Tree::walk()` (pre-order, parents before children). No recursion and no change to the `types` crate.

- Measure pass: iterate `walk()` in reverse, so children come before their parent, and fill a `HashMap<Vec<usize>, Size { width, height }>`.
  - A `Text` is `interior(text) × 1`.
  - A `Box` reads its children's sizes, found with `contains(&[path, i])`. Width is the sum of child widths, plus `FLEX_GAP` (1) between neighbours, plus 2 for the border. Height is the tallest child plus 2.
  - An empty box therefore still measures 3 × 3.
- Place pass: iterate `walk()` forward. Each box records its origin and inner width in a second map, `HashMap<Vec<usize>, Frame>`, and keeps an x-cursor for its next child. A node reads its parent's frame, pushes its placement, and advances the parent's cursor.
  - A `Box` pushes its border placement. Each child is centred vertically on the row's middle: `child_y = y + 1 + (inner_height - child_height) / 2`.
  - `Fit` centres the row inside the box. `Full` takes `width = window.cols`, and with `Justify::SpaceBetween` every gap is `free / gaps`, with the remainder handed to the last gaps. This is today's `text_offsets` remainder rule, generalised from texts to all children. With `Justify::Start` the gap is `FLEX_GAP`.
  - Inner boxes are always `Fit` and lay out their own children the same way, because the walk reaches them too.
- `scene` runs both passes, stacks the outer boxes with `FLEX_GAP`, and `view::centre` centres the whole scene, as now.
- Colours: the box at `selected` gets `FLEX_SELECTED_COLOUR` in MOVE. A selected `Text` label also uses `FLEX_SELECTED_COLOUR`. An inner box gets the selected colour when it is itself selected, where today it is always the border colour.

### Collaborators

- `types::Tree<FlexNode>`: `walk` (drives measure and place), `push`, `value`, `value_mut`, `next`, `previous`, `parent`, `contains`. No change to the `types` crate.
- `view::interior`, `view::centre`, `Placement`, `BOX_HEIGHT`. No change.
- `src/flex/mod.rs` and `src/bin/dre-flex.rs` only need updating for the new `FlexNode` type, if they touch `texts`.

### Testing plan (TDD, thin slices)

1. State: `new_box()`, `FlexNode`, `selected_box_path()`. Existing state tests move from `texts` to child nodes and stay green.
2. State: `s` and `A` append to the end of the selected box's children, in order. WRITE targets the selected text or the box's last text.
3. View: the measure pass for text, empty box, and a row of text, box, text.
4. View: the place pass puts siblings in a row with a 1-cell gap, texts centred vertically on the tallest sibling, and the outer box centred on screen.
5. View: `Full` with `g` puts the first sibling at the left edge, the last at the right edge, and equal gaps. The remainder is spread.
6. View: a selected text and a selected inner box get the selected colour.
7. Delete the old `place_outer_box` code paths and the tests that only covered them.

### Out of scope

- Column direction and `d` (spec 197). the two passes are written for a row, and 197 will add the axis.
- Deleting nodes and moving nodes between boxes.
