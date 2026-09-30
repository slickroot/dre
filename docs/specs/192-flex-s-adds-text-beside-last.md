# Flex: s adds a text beside the last one in MOVE

## User Story

Noor runs `dre-flex`, types "Hello" and presses Enter, so they're in MOVE. They press `s`, and a new empty text appears in the same box, right after "Hello", with a 1-cell space between them. `dre-flex` switches to WRITE, and Noor types "World". The box grows to hold "Hello World", and it stays centred on the screen. Noor presses Enter, then `s` again, and a third text appears after "World". Back in WRITE, pressing `s` just types an "s".

## Acceptance Criteria

- In MOVE, `s` adds a new empty text to the right of the last text in the same box.
- After `s`, `dre-flex` switches to WRITE.
- Typed characters go into the newly added text.
- Each additional `s` adds another text to the right of the last one.
- There is a 1-cell space between texts.
- The box grows to fit all its texts and the spaces between them.
- The box stays centred on the screen.
- Backspace on an empty new text does nothing.
- In WRITE, `s` types an "s" and doesn't add a text.

## Technical Design

Decisions:

- **A box holds an ordered list of texts.** `FlexBox.text: String` becomes `texts: Vec<String>`. The list is never empty, the same rule `FlexState.boxes` follows. There's no separate `FlexText` struct until a text needs its own fields.
- **Typing goes into the box's last text.** `write_key` edits `newest_box().texts.last_mut()`. Backspace pops a char from that text and never removes the text, so backspace on an empty new text does nothing.
- **`s` is exactly `"s"`, and it's a MOVE key.** It pushes an empty `String` onto the newest box's `texts` and sets the mode to `Write`. In WRITE, `"s"` is a printable char and is typed into the last text. Unlike `a`, `s` changes the mode, because Noor's next move is to type.
- **`s` targets the newest box.** This is the same rule as `a`, `w` and `f`. There's no selection yet.
- **Texts are laid out left to right by a helper in `flex::view`.** `text_offsets(texts) -> Vec<i64>` returns each text's x relative to the box's inner left edge, with `TEXT_GAP = 1` cell between neighbours. An empty text keeps its 1-cell interior (`view::interior`), so the cursor cell shows right after the space. Spec 193 (`g`) will change the helper's body to spread the texts, and nothing else needs to move.
- **A `Fit` box is as wide as its row of texts.** `width = sum(interior) + TEXT_GAP * (n - 1) + 2`. A `Full` box is still `window.cols` wide, and its texts start at `box.x + 1`.
- **Each text is its own `Label` placement** at `box.x + 1 + offset`, on the box's middle row. The border, colours and height are unchanged.
- **Centring needs no change.** The box is still placed at `x = -width.div_euclid(2)` and the whole scene goes through `view::centre`, so the box stays centred as it grows. Nothing changes in `crate::view`.
- **All of the flex box's looks stay in `flex::view`.**

### `flex::state`: what it knows and does

```rust
pub(crate) struct FlexBox {
    pub(crate) texts: Vec<String>,   // never empty
    pub(crate) width: FlexWidth,
    pub(crate) filled: bool,
}
```

- `FlexBox::default()` has `texts: vec![String::new()]`.
- `FlexState` gets a `newest_text(&mut self) -> &mut String` helper next to `newest_box`. `write_key` uses it for typing and backspace.
- `move_key`: `"s"` pushes `String::new()` onto `newest_box().texts` and sets `mode = Write`. Every other arm is unchanged.

### `flex::view`: what changes

```rust
const TEXT_GAP: i64 = 1;

fn text_offsets(texts: &[String]) -> Vec<i64>
```

- `scene` computes the `Fit` width from the texts, as above. It then pushes the box, then one `Label` per text at `x + 1 + offset`.
- `text_offsets` puts the first text at 0 and each next one at the previous offset + the previous text's interior + `TEXT_GAP`.

### Tests

`reduce` unit tests in `flex::state`:

- `FlexBox::default()` has exactly one empty text.
- In `Move` with `["Hello"]`, `"s"` gives `["Hello", ""]` in `Write`, with effect `None`, and the box count is still 1.
- Typing `"W"`, `"o"`, `"r"`, `"l"`, `"d"` after that gives `["Hello", "World"]`.
- After `"\r"`, a second `"s"` gives `["Hello", "World", ""]` in `Write`.
- In `Write` with `["Hello"]`, `"s"` gives `["Hellos"]`. No text is added and the mode stays `Write`.
- Backspace with `["Hello", ""]` in `Write` leaves the state unchanged.
- Backspace with `["Hello", "W"]` gives `["Hello", ""]`, and a second backspace changes nothing.
- `"s"` only touches the newest box: with two boxes, only the second gets a new text.
- The existing `text` tests change to `texts`, through the `holding`, `hello_in`, `stacked` and `newest` helpers.

`scene` and `text_offsets` unit tests in `flex::view` (with the 80×24 `WINDOW`):

- `text_offsets(["Hello", ""])` is `[0, 6]`, and `["Hello", "World", ""]` is `[0, 6, 12]`.
- A `Fit` box with `["Hello", ""]` is 9 wide, and one with `["Hello", "World"]` is 13 wide.
- A box with two texts places one box and two labels, and each label sits inside the box.
- The second label starts `interior(first) + 1` cells after the first, so there's a 1-cell space between them.
- The box stays centred: for `["Hello", "World"]` the left and right margins differ by at most 1.
- The box is centred again after `s` grows it: the margins still differ by at most 1.
- A `Full` box with two texts is still `window.cols` wide, and its first label is at `box.x + 1`.
- The existing single-text tests stay green (a one-text box is laid out exactly as before).
