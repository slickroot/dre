# Flex: g spreads the texts across a full-width box

## User Story

Noor has a box with "Hello" and "World" in it, and the box is in MOVE. They press `w`, so the box goes full width. The texts sit side by side at the left, with a 1-cell space between them. Noor presses `g`. "Hello" stays at the left edge, "World" moves to the right edge, and the free space is shared between them. Noor presses `g` again, and the texts are back next to each other with the 1-cell space. In WRITE, pressing `g` just types a "g".

## Acceptance Criteria

- In MOVE, `g` turns the gap on. Pressing `g` again turns it off.
- With the gap on and the box full width, the first text sits at the left edge and the last text sits at the right edge.
- With three or more texts, the texts in between are placed so every gap is the same size.
- If the free space doesn't split evenly, the extra cells go to the right gaps.
- With the gap on and only one text, the text stays at the left, where it is today.
- With the gap on and the box fit-to-text, nothing changes on screen. The texts spread out as soon as `w` makes the box full width.
- With the gap off, the texts sit next to each other with a 1-cell space between them.
- In WRITE, `g` types a "g" and doesn't change the gap.

## Technical Design

This builds on spec 192: `FlexBox.texts: Vec<String>` and the `text_offsets` helper in `flex::view`.

Decisions:

- **Justify is a per-box enum, named after CSS `justify-content`.** `FlexBox` gets `justify: Justify`, where `Justify` is `Start` (the default) or `SpaceBetween`. It's shaped like `FlexWidth`, with a `toggle()` that flips between the two.
- **`g` is exactly `"g"`, and it's a MOVE key.** It toggles `newest_box().justify`, the same way `w` toggles `width`. The mode doesn't change. In WRITE, `"g"` is a printable char and is typed into the last text.
- **`g` targets the newest box.** This is the same rule as `a`, `s`, `w` and `f`.
- **`text_offsets` takes the justify and the inner width.** Its signature becomes `text_offsets(texts, justify, inner_width) -> Vec<i64>`. `scene` passes `inner_width = width - 2`.
  - `Start` packs the texts from 0 with `TEXT_GAP` between them, exactly as in spec 192. `inner_width` is ignored.
  - `SpaceBetween` sets `free = inner_width - sum(interior)` and splits it over the `n - 1` gaps. Each gap is `free.div_euclid(n - 1)`, and the last `free.rem_euclid(n - 1)` gaps get one extra cell. With one text there are no gaps, so the text sits at 0.
- **Fit-to-text needs no special case.** A `Fit` box's inner width is the packed width, `sum(interior) + TEXT_GAP * (n - 1)`, so `SpaceBetween` in a `Fit` box gives 1-cell gaps on its own. `scene` doesn't branch on `FlexWidth` for justify, and the texts spread out as soon as `w` makes the box `Full`.
- **Overflow is not handled.** If a narrow window makes `free` smaller than `n - 1`, the gaps shrink or go negative and the texts touch or overlap. Nothing is clamped.
- **All of the flex box's looks stay in `flex::view`.** Nothing changes in `crate::view`.

### `flex::state`: what it knows and does

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Justify {
    #[default]
    Start,
    SpaceBetween,
}

impl Justify {
    pub(crate) fn toggle(self) -> Self;
}

pub(crate) struct FlexBox {
    pub(crate) texts: Vec<String>,   // never empty
    pub(crate) width: FlexWidth,
    pub(crate) justify: Justify,
    pub(crate) filled: bool,
}
```

- `FlexBox::default()` has `justify: Justify::Start`.
- `move_key`: `"g"` toggles `newest_box().justify`. Every other arm is unchanged.

### `flex::view`: what changes

```rust
fn text_offsets(texts: &[String], justify: Justify, inner_width: i64) -> Vec<i64>
```

- `scene` computes `width` as in spec 192. It then calls `text_offsets(&flex_box.texts, flex_box.justify, width - 2)` and places each `Label` at `x + 1 + offset`.

### Tests

`reduce` unit tests in `flex::state`:

- `FlexBox::default()` has `Justify::Start`.
- `Justify::Start.toggle()` is `SpaceBetween`, and `SpaceBetween.toggle()` is `Start`.
- In `Move`, `"g"` sets the newest box to `SpaceBetween` with effect `None`, and the mode stays `Move`.
- A second `"g"` sets it back to `Start`.
- In `Write` with `["Hello"]`, `"g"` gives `["Hellog"]`, and the justify stays `Start`.
- `"g"` only touches the newest box: with two boxes, only the second one changes.

`text_offsets` unit tests in `flex::view`:

- `Start` ignores `inner_width`: `["Hello", "World"]` with inner width 78 is `[0, 6]`.
- `SpaceBetween` with `["Hello", "World"]` and inner width 78 is `[0, 73]`, so the last text ends at the inner right edge.
- `SpaceBetween` with three texts spreads evenly: `["a", "b", "c"]` with inner width 9 is `[0, 4, 8]`.
- An uneven split gives the extra cells to the right gaps: `["a", "b", "c"]` with inner width 10 is `[0, 4, 9]`, and with four texts and inner width 12 it's `[0, 3, 7, 11]`.
- `SpaceBetween` with one text is `[0]`.
- `SpaceBetween` with the packed width as the inner width gives the same offsets as `Start`.

`scene` unit tests in `flex::view` (with the 80×24 `WINDOW`):

- A `Full` `SpaceBetween` box with `["Hello", "World"]`: the first label is at `box.x + 1`, and the last label ends at `box.x + box.width - 1`.
- A `Fit` `SpaceBetween` box places the same things as a `Fit` `Start` box with the same texts.
- A `Full` `Start` box with two texts is laid out as in spec 192, with a 1-cell space between the labels.
