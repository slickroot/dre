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

Depends on spec 198 (texts are tree nodes). It does not need spec 199.

### Keys (MOVE and WRITE)

- `s` and `A` already append to the end of the selected box's children after spec 198. That append order is what this spec lays out, so no key handling changes here.
- WRITE typing is unchanged: it goes into the selected box's last `Text` child.

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
- Colours: the box at `selected` gets `FLEX_SELECTED_COLOUR` in MOVE, inner boxes included, where today an inner box always has the border colour.

### Collaborators

- `types::Tree<FlexNode>`: `walk` (drives measure and place), `push`, `value`, `value_mut`, `next`, `previous`, `parent`, `contains`. No change to the `types` crate.
- `view::interior`, `view::centre`, `Placement`, `BOX_HEIGHT`. No change.
- Only `src/flex/view.rs` changes. State and `mod.rs` are already on `FlexNode` after spec 198.

### Testing plan (TDD, thin slices)

1. View: the measure pass for text, empty box, and a row of text, box, text.
2. View: the place pass puts siblings in a row with a 1-cell gap, texts centred vertically on the tallest sibling, and the outer box centred on screen.
3. View: `Full` with `g` puts the first sibling at the left edge, the last at the right edge, and equal gaps. The remainder is spread.
4. View: a selected inner box gets the selected colour.
5. Delete the old `place_outer_box` code paths and the tests that only covered them.

### Out of scope

- Selecting texts (spec 199).
- Column direction and `d` (spec 197). The two passes are written for a row, and 197 will add the axis.
- Deleting nodes and moving nodes between boxes.
