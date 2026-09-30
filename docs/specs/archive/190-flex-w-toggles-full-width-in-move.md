# Flex: w toggles full width in MOVE

## User Story

Sami runs `dre-flex` and types "Hello". The box is just wide enough for the text. They press Enter to go to MOVE, then press `w`. The box stretches from the left edge of the window to the right edge, with "Hello" at the left. They press `w` again and the box shrinks back to fit the text. They press `w` once more to make it full width, then press `i` to go back to WRITE. The box stays full width while they type. In WRITE, pressing `w` just types a "w".

## Acceptance Criteria

- The box starts at text width.
- In MOVE, `w` stretches the box across the whole window, left edge to right edge.
- When the box is full width, the text sits at the left.
- In MOVE, `w` on a full-width box shrinks it back to text width.
- In WRITE, `w` types a "w" and doesn't change the width.
- The box stays full width when Sami switches from MOVE back to WRITE with `i`.

## Technical Design

Decisions:

- **Width is an enum, not a bool.** `FlexState` gains `width: FlexWidth`, with the variants `Fit` and `Full`. It starts as `Fit`. The names are the spec's own words, "text width" and "full width". `scene` does one `match` on it to get both the box width and the label's x. A new width later becomes a new variant, not a second bool.
- **The state stores `Full`, not a number of columns.** The box width comes from `window.cols` at render time, so a full-width box follows the window when it's resized. `reduce` never needs to know the window size.
- **`w` is exactly `"w"`, and it's a MOVE key.** It gets one arm in `move_key`, which flips the width with `FlexWidth::toggle()` and returns `None`. In WRITE, `"w"` is still a printable char and is typed into the box.
- **Switching modes keeps the width.** `write_key` and the `i` arm in `move_key` already build the new state with `..state`, so `width` carries over and nothing extra is needed.
- **The full-width box spans the window, and its label sits at `x = 1`.** The box is placed at `x: 0` with `width: window.cols`. `view::centre` then works out a zero horizontal shift and still centres the box vertically, so `crate::view` doesn't change. In a `Fit` box, `label_centre` already gives `x = 1`, just inside the 1-pixel border. `Full` uses the same `x = 1`, so the text doesn't move relative to the box's left edge when the width toggles. There's no extra padding.
- **All of the flex box's looks stay in `flex::view`** (as decided in 186). There's no new helper in `crate::view`.

### `flex::state`: what it knows and does

```rust
pub(crate) enum FlexWidth { Fit, Full }

impl FlexWidth {
    pub(crate) fn toggle(self) -> Self   // Fit <-> Full
}

pub(crate) struct FlexState { text: String, mode: FlexMode, width: FlexWidth }
```

- `FlexState::default()` has `width: FlexWidth::Fit`.
- `move_key`: `"w"` → `FlexState { width: state.width.toggle(), ..state }`, `None`. `"i"` and `"q"` are unchanged.
- `write_key`: unchanged. `"w"` is typed like any other printable char.

### `flex::view`: what changes

```rust
pub(crate) fn scene(state: &FlexState, window: Area) -> Scene<'_>
```

- `let (width, label_x) = match state.width { Fit => (interior(text) + 2, label_centre(width, text)), Full => (window.cols, 1) };`
- The box is placed at `x: 0` with that `width`. The label is placed at `label_x`. The height, border, colours and `view::centre` are unchanged.

### Tests

`reduce` unit tests in `flex::state`:

- `FlexState::default()` has `width == Fit`.
- In `Move` with `"Hello"`, `"w"` gives `Full` and leaves the text and mode unchanged, with effect `None`. A second `"w"` gives `Fit` again.
- In `Write` with `"Hello"`, `"w"` makes the text `"Hellow"` and leaves the width `Fit`, with effect `None`.
- `Full` in `Move`, then `"i"`: the mode is `Write` and the width is still `Full`. Typing a char after that keeps `Full`.
- `FlexWidth::toggle`: `Fit` → `Full` → `Fit`.

`scene` unit tests in `flex::view` (with the 80×24 `WINDOW`):

- A `Full` box for `"Hello"` is at `x == 0` with `width == 80`, the window's columns.
- In a `Full` box, the label is at the box's `x + 1`.
- A `Fit` box for `"Hello"` is still 7 wide, which is covered by the existing tests.
- A `Full` box follows the window: with a 40-column window, `width == 40`.
