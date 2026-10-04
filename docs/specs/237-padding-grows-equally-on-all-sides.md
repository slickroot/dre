# Padding grows equally on all sides

Doug selects a box — whether or not it has a border — and presses `]`. He sees the space around the text grow equally on all four sides: top, bottom, left, and right. He presses `]` again and the space grows further, still equally on every side. Happy, he goes back to sleep!

## Acceptance Criteria

- Pressing `]` on a selected box with no border makes padding appear on all four sides equally (not just left/right).
- Pressing `]` on a selected box with a border grows the padding on all four sides equally (currently only left/right grow).
- Each additional press of `]` keeps growing all four sides together, by the same amount.

## Technical Design

### Overview

Today `padding()` in `src/flex/view.rs:57-69` conflates two unrelated concerns: it conditions the result on `flex_box.border` (returning `0,0` whenever there's no border, regardless of `flex_box.padding`), and it only scales the *width* term by `(1 + flex_box.padding)` while the *height* term is a constant `FLEX_SPACE.height`. That's why a borderless box's padding presses do nothing, and why a bordered box only grows left/right.

The fix: `padding()` is driven purely by the `flex_box.padding` field — border has no bearing on it at all. Pressing `]` with no border grows all four sides starting from flush (0); pressing `]` on a bordered box also starts flush against the border line and grows outward from there. There's no reserved base unit for the border itself — text is allowed to sit flush against the border when `padding == 0`, same as a borderless box sits flush against its own edge.

### Changes in `src/flex/view.rs`

- Replace the body of `padding()`:
  ```rust
  fn padding(flex_box: &FlexBox) -> Size {
      Size {
          width: FLEX_SPACE.width * i64::from(flex_box.padding),
          height: FLEX_SPACE.height * i64::from(flex_box.padding),
      }
  }
  ```
  No `if flex_box.border` branch. Both dimensions scale by the same `flex_box.padding` count, so each `]` press adds one `FLEX_SPACE` unit of space on every side — equal in the aspect-corrected sense `FLEX_SPACE` already provides elsewhere (gaps between flex items use the same unit).
- `border_space()` and all callers (`measure()`, `arrange()`) are unchanged otherwise — they keep calling `padding()`/`border_space()` exactly as before; only `padding()`'s internals change.

### Test updates in `src/flex/view.rs`

- `a_borderless_text_ignores_its_padding` is replaced by a test asserting a borderless box's padding *is* applied (measures larger with a non-zero `padding` field).
- `a_new_box_measures_its_padding_on_both_sides`, `close_bracket_adds_one_space_width_of_padding_on_the_left_and_right_of_the_box`, and `close_bracket_twice_adds_two_space_widths_of_padding_on_each_side` are updated: with the border-conditional base removed, a freshly created box (`padding == 0`) now measures with zero extra padding space (text flush against the border), and each `]` press adds exactly one `FLEX_SPACE.width`/`FLEX_SPACE.height` unit per side instead of the old `2 * FLEX_SPACE.width`-per-press/no-height-growth behavior.
