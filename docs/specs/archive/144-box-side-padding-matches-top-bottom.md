# Box side padding matches top/bottom padding

Doug draws a box with a label in dre. Looking at it, he notices the space
between the label and the box's left/right edges now visually matches the
space between the label and the top/bottom edges, instead of looking
cramped on the sides.

## Acceptance Criteria

- Every box (regardless of label length) gets one extra character cell of
  horizontal spacing added on the left and right of the label, on top of
  what's there today.
- Eyeballing any box in the terminal, the left/right padding now looks
  roughly even with the top/bottom padding, instead of visibly tighter.

## Technical Design

Today, horizontal box sizing/positioning is driven by `src/layout.rs`'s
`BORDERS: i64 = 2` constant, used in `width()` and `centre()`. Despite its
name, `BORDERS` isn't consumed by border-drawing code (that reads `width`
and `sides` in `render/terminal.rs`/`render/svg.rs`); it's purely a
horizontal sizing/positioning budget, and today's value leaves the label
sitting flush against the border character with no padding at all — hence
the cramped look versus the vertical padding baked into `BOX_HEIGHT`.

- Rename `BORDERS` to `SIDE_PADDING: i64 = 2` (per-side value, up from an
  implicit 1 today), in `src/layout.rs`.
- Update `width()` to build the box width as
  `interior(label) + SIDE_PADDING * 2`.
- Update `centre()`'s leftover calculation the same way:
  `width - SIDE_PADDING * 2 - interior(label)`.
- Update `footer()`'s `box_width` calculation identically:
  `interior(text) + SIDE_PADDING * 2`.
- Update the `svg.rs` import of `BORDERS` to `SIDE_PADDING` and any other
  reference sites (tests in `src/layout.rs` asserting against `BORDERS`).
- No change to `BOX_HEIGHT` or vertical placement — vertical padding is
  already correct and untouched.
