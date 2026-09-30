# Flex: A adds a box inside the selected box in MOVE

## User Story

Noor runs `dre-flex`, types "Hello" and presses Enter, so they're in MOVE. They press `A`, and a new empty box appears inside the "Hello" box, below the text. There's a 1-cell gap between "Hello" and the inner box, and the inner box is centred under the text. The outer box grows to fit it, and it stays centred on the screen. Noor is still in MOVE. They press `A` again, and a second empty box appears inside the outer box, below the first one. Noor presses `i` and types "Hi", and the text goes into the outer box's "Hello", not into an inner box. In WRITE, pressing `A` just types an "A".

## Acceptance Criteria

- In MOVE, `A` adds a new empty box inside the selected box, below its text.
- After `A`, `dre-flex` stays in MOVE.
- There is a 1-cell gap between the text and the inner box.
- The inner box is centred under the text.
- The inner box never touches the outer box's left or right padding: there is always at least 1 cell of padding between them.
- The outer box grows to fit the inner box.
- The outer box stays centred on the screen.
- Each additional `A` adds another inner box inside the same outer box, below the last inner box.
- An inner box looks like an empty box from `a`: a thin border in the usual border colour, a 1-cell interior, and no fill.
- Typing in WRITE still goes into the selected box's last text, never into an inner box.
- Inner boxes can't be selected. `j` and `k` still move only between outer boxes.
- `a` is unchanged. It adds a new outer box at the bottom of the stack, as in spec 194.
- In WRITE, `A` types an "A" and doesn't add an inner box.

## Technical Design

### State (`src/flex/state.rs`)

- `FlexState.boxes` changes from `Vec<FlexBox>` to `types::Tree<FlexBox>`, the same tree `Document` uses. The root is hidden (`Tree::root(vec![Tree::leaf(FlexBox::default())])`, which works because `FlexBox: Default`). Outer boxes are the root's children. Inner boxes are children of an outer box.
- `FlexState.selected` changes from `usize` to `Vec<usize>`, a path into the tree. `Default` sets it to `[0]`.
- `selected_box()` returns `self.boxes.value_mut(&self.selected)`. `selected_text()` is unchanged, so `w`, `f`, `g`, `s` and WRITE typing still act on the selected box without changes.
- `move_key`:
  - `"a"`: `self.selected = self.boxes.push(&[], Tree::leaf(FlexBox::default()))`. It still adds an outer box at the bottom and selects it.
  - `"A"` (new): `self.boxes.push(&self.selected, Tree::leaf(FlexBox::default()))`. It leaves `selected` and `mode` alone, so `dre-flex` stays in MOVE and the outer box stays selected.
  - `"j"`: `self.selected = self.boxes.next(&self.selected)`.
  - `"k"`: `self.selected = self.boxes.previous(&self.selected)`.
  - `next` and `previous` stop at the last and first sibling, so the hand-written `min` and `saturating_sub` go away.
- Inner boxes can't be selected because nothing sets `selected` to a depth-2 path. `j` and `k` only step between depth-1 siblings. No special rule is needed.
- `write_key` doesn't change. `"A"` reaches the printable branch and is typed into the selected text.
- Rejected: `inner_boxes: usize` (would need replacing as soon as an inner box gets text or style), a `children: Vec<FlexBox>` field on `FlexBox` (a second tree-shaped structure next to `Tree`), and keeping `selected: usize` (every call site would build `[selected]`, and the spec that selects inner boxes would change the type anyway).

### View (`src/flex/view.rs`)

`scene` walks the outer boxes (the root's children) and stacks them. The per-box code moves into a helper that also places the box's inner boxes and returns the box's height.

- **Height** of an outer box with `n` inner boxes: `BOX_HEIGHT + n * (FLEX_GAP + BOX_HEIGHT)`. One `FLEX_GAP` row comes before every inner box, including the first. There is no padding between the last inner box and the outer bottom border.
- **Stacking:** the next outer box's `y` is the previous `y` plus its height plus `FLEX_GAP`. This replaces the fixed `i * (BOX_HEIGHT + FLEX_GAP)`.
- **Text row:** stays on the first interior row, `y + BOX_HEIGHT / 2`, as now.
- **Inner box width:** `view::interior("") + 2`, the same as an empty box from `a`.
- **Outer Fit width:** `max(texts row width, inner box width) + 2`. Full width is still the window width.
- **Texts row in a widened Fit box:** when the inner box is wider than the texts row, the texts row is centred in the content area (the outer interior, `width - 2`), so the text and the inner box line up.
- **Inner box x:** centred on the span of the texts row, from the start of the first text to the end of the last text, using the offsets from `text_offsets`, then kept inside the content area so it never reaches the outer box's padding cells. `w` and `g` therefore move the inner box together with the text.
- **Inner box look:** `PlacementNode::Box` with `FLEX_BORDER_COLOUR`, `FLEX_BORDER`, no fill, `rounded: false`, `ALL_SIDES`. It is never drawn as selected, and it never wears the outer box's `filled`.
- **Centring on screen:** the existing shift by the leftmost `x` and `view::centre` are unchanged. The outer box is at least as wide as its inner boxes, so they never change the leftmost `x`.

### Tests

- State:
  - `A` in MOVE adds one child to the selected box, keeps `selected` and stays in MOVE.
  - A second `A` adds a second child to the same box.
  - `A` on the second outer box adds the child only to that box.
  - `A` in WRITE types "A" and adds no child.
  - After `A` and `i`, typing goes into the outer box's last text.
  - `j` and `k` still move only between outer boxes after `A`, and never select an inner box.
  - `a` after `A` still adds an outer box at the bottom.
  - The existing `a`, `j`, `k`, `w`, `f`, `g`, `s` tests keep passing with the tree.
- View:
  - One inner box sits 1 row below the text row, is centred under the text, and uses the thin border and the usual border colour with no fill.
  - Two inner boxes are stacked with a 1-row gap.
  - The outer height is `3 + 4n`.
  - The outer Fit width grows to `inner width + 2` when the text is shorter than that.
  - A following outer box starts `FLEX_GAP` below the taller outer box.
  - In Full width and with `g`, the inner box stays centred on the text span.
  - With a short or empty text, in Fit and in Full width with Start justify, the inner box stays inside the content area and never overlaps the outer padding cells.
  - The whole scene stays centred on the screen.
  - In MOVE, the inner box border stays `FLEX_BORDER_COLOUR` even when its outer box is selected.
