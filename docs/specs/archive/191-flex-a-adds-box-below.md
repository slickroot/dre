# Flex: a adds a box below in MOVE

## User Story

Noor runs `dre-flex`, types "Hello" and presses Enter, so they're in MOVE. They press `a`, and a new empty box appears below "Hello", with a 1-cell gap between them. Noor is still in MOVE. They press `a` again, and a third empty box appears below the second one. The whole stack stays centred on the screen, with the boxes lined up by their centres. Back in WRITE, pressing `a` just types an "a".

## Acceptance Criteria

- In MOVE, `a` adds a new empty box below the last box.
- After `a`, `dre-flex` stays in MOVE.
- Each additional `a` adds another box below the last one.
- There is a 1-cell gap between boxes.
- The boxes are lined up by their centres.
- The whole stack stays centred on the screen.
- In WRITE, `a` types an "a" and doesn't add a box.

## Technical Design

Decisions:

- **Typing goes into the newest box.** `FlexState` holds a list of boxes that is never empty. `write_key` edits `boxes.last_mut()`, so after `a` then `i`, Noor types into the box they just added. There's no cursor or selection index yet. That comes when a story needs to move between boxes.
- **A box is a `FlexBox` struct, not a `String`.** `FlexBox` has only `text` today. Specs 189 (`fill`) and 190 (`width`) add their fields to `FlexBox`, not to `FlexState`, and their `f`/`w` arms edit `boxes.last_mut()`. This changes 190's design, which puts `width` on `FlexState`. Whichever of 189, 190 and 191 is built first, the others fit in without reshaping the state.
- **`a` is exactly `"a"`, and it's a MOVE key.** It gets one arm in `move_key`, which pushes `FlexBox::default()` and returns `None`. The mode stays `Move`. In WRITE, `"a"` is still a printable char and is typed into the last box.
- **Boxes share a centre line at x = 0.** `scene` places each box at `x = -width.div_euclid(2)` and `y = i * (BOX_HEIGHT + FLEX_GAP)`, where `FLEX_GAP = 1`. `view::centre` already works from the leftmost x and the total height, so it moves the whole stack to the middle of the window. There's no "widest box" pass and no change to `crate::view`.
- **Odd widths lean right.** When a box and the one above it differ in width by an odd number of cells, the spare cell goes on the left, so the narrower box sits half a cell to the right. This is the same rule `label_centre` uses for text inside a box.
- **Each box's label is placed relative to its own box.** It goes at `box.x + label_centre(width, text)` and `box.y + BOX_HEIGHT / 2`. When 190 lands, a `Full` box is placed at `x = -window.cols.div_euclid(2)` with `width = window.cols`, and its label sits at `box.x + 1`.
- **All of the flex box's looks stay in `flex::view`** (as decided in 186).

### `flex::state`: what it knows and does

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FlexBox { pub(crate) text: String }

pub(crate) struct FlexState { pub(crate) boxes: Vec<FlexBox>, pub(crate) mode: FlexMode }
```

- `FlexState::default()` has `boxes: vec![FlexBox::default()]` and `mode: Write`.
- `write_key`: typing and backspace edit `state.boxes.last_mut()` instead of `state.text`. `"\r"` is unchanged.
- `move_key`: `"a"` → push `FlexBox::default()` onto `boxes`, `None`. `"i"` and `"q"` are unchanged.
- Backspace on an empty last box does nothing. It doesn't remove the box.

### `flex::view`: what changes

```rust
const FLEX_GAP: i64 = 1;

pub(crate) fn scene(state: &FlexState, window: Area) -> Scene<'_>
```

- `scene` loops over `state.boxes` with their index. For each box it pushes a `Box` placement at `(-width.div_euclid(2), i * (BOX_HEIGHT + FLEX_GAP))` with `width = interior(text) + 2`, then pushes its `Label` inside it. The border, colours and height are unchanged.
- The whole list goes through `view::centre` once.

### `flex` (`run_loop`) tests

- The existing tests that check `state.text` check `state.boxes.last().unwrap().text` instead. `run_loop` itself doesn't change.

### Tests

`reduce` unit tests in `flex::state`:

- `FlexState::default()` has exactly one empty box and mode `Write`.
- In `Move` with `["Hello"]`, `"a"` gives `["Hello", ""]` in `Move`, with effect `None`.
- A second `"a"` gives `["Hello", "", ""]`.
- In `Write` with `["Hello"]`, `"a"` gives `["Helloa"]`. No box is added.
- With `["Hello", ""]` in `Move`, `"i"` then `"H"`, `"i"` gives `["Hello", "Hi"]`. Typing goes into the last box.
- Backspace with `["Hello", ""]` in `Write` leaves `["Hello", ""]`.
- The existing tests change `text` to `boxes` through the `hello_in` and `typed` helpers.

`scene` unit tests in `flex::view` (with the 80×24 `WINDOW`):

- `["Hello", ""]` places two boxes and two labels.
- The second box's `y` is the first box's `y + BOX_HEIGHT + 1`, so there's a 1-cell gap.
- Boxes line up by their centres: for `["Hello", ""]` (7 and 3 wide), `2 * x + width` is the same for both.
- Odd widths lean right: for `["Hi", ""]` (4 and 3 wide), the empty box's `x` is the `"Hi"` box's `x + 1`.
- The stack is centred: for `["Hello", "", ""]`, the stack's left and right margins differ by at most 1, and so do its top and bottom margins.
- Each label sits inside its own box.
- The existing single-box tests stay green (one box, 7 wide for "Hello", same colours and border).
