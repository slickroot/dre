# Footer columns own their padding

This is a technical refactoring spec. It changes how the footer's layout is
built. It does not intentionally change rendered output, except that every
gap between footer elements becomes uniform (see Acceptance Criteria).

## Problem

`layout::footer` (`src/view.rs:153`) only routes the LED and mode-word
columns through `stack_columns`. The filename and suffix columns are still
built by hand:

- `filename_x = word_end + SIDE_PADDING` and `suffix_x = filename_x +
  filename_width` are computed as running offsets, duplicating exactly what
  `stack_columns` already does for the first two columns.
- The mode-word `Column` uses `padding: LED_LABEL_GAP - SIDE_PADDING`, a
  negative value that exists only to cancel out the LED column's own
  trailing padding — because `Column.padding` is symmetric and additive
  between neighbours, one column can't ask for a tighter gap than its
  neighbour's padding without going negative.
- The footer's bordered box's width is `suffix_x + suffix_width +
  SIDE_PADDING` — an extra manual `SIDE_PADDING` bolted on at the end,
  rather than coming from the stack itself.
- The cursor's `x` is computed from the hand-tracked `filename_x` variable.
  Once filename is a `Column` like any other, that variable goes away.

All four symptoms come from the same gap: `stack_columns` isn't used
end-to-end, so every column past the second one falls back to manual
arithmetic.

## Acceptance Criteria

- LED, mode word, filename and suffix are all built as `Column`s and placed
  by a single `stack_columns` call; no footer element's `x` is computed by
  hand.
- Every column uses the same padding for now (`padding: 1` on each `Column`
  literal — a per-column value, not a shared constant, so a later spec can
  differentiate them again without changing the type). `SIDE_PADDING` and
  `LED_LABEL_GAP` are deleted. This doubles the gap between the LED and the
  mode word from 1 to 2 cells (padding is symmetric and additive between
  neighbours) and halves the box's own left/right inset from 2 to 1 cell —
  an accepted, deliberate visual change while the framework is built; exact
  spacing can be tuned per-column later.
- The bordered box's width is exactly the total `stack_columns` returns —
  no separate `+ SIDE_PADDING` added afterwards.
- The cursor placement is emitted by `stack_columns` itself, from a field on
  the filename `Column`, not looked up afterwards by index or path.

## Technical Design

### `Column` gains `padding: u8` and `cursor: Option<i64>`

```rust
struct Column {
    node: PlacementNode<'static>,
    width: i64,
    padding: u8,
    cursor: Option<i64>,
}
```

- `padding: u8` replaces the current `padding: i64`. It is symmetric (added
  on both sides of the column, same as today) and can no longer go
  negative, which is what forces the LED/mode-word cancellation hack out of
  existence. There is no separate container-level padding — the box's own
  left/right inset comes from the first/last column's own `padding`, same
  as today.
- `cursor: Option<i64>` is the character offset from *this column's own
  `x`* at which a `Cursor` placement should be emitted, or `None` if this
  column has no cursor. Only the filename column ever sets it (from
  `model.cursor`); every other column's literal sets `cursor: None`.

### `stack_columns` emits the cursor inline

```rust
fn stack_columns(columns: Vec<Column>, y: i64) -> (Vec<Placement<'static>>, i64)
```

For each column, in order:

1. `x += column.padding as i64`.
2. Push the column's own `Placement` at `(x, y)`.
3. If `column.cursor` is `Some(offset)`, push a `Cursor` placement at
   `(x + offset, y)`.
4. `x += column.width + column.padding as i64`.

Return `(placements, x)` — `x` is the total width, unchanged in meaning
from today.

This removes the need for any lookup by index or path: the cursor is
emitted at the moment its column's position is known, which is exactly
what today's hand-written code after `stack_columns` already does for the
filename column — this just moves that logic inside `stack_columns` so it
applies uniformly, and works even though filename is no longer special-cased
after the call.

### `layout::footer`

Builds one `Vec<Column>` of all four columns — LED, mode word, filename,
suffix — each with `padding: 1`, and `cursor: model.cursor.map(|c| c as
i64)` on the filename column only. One `stack_columns` call places all
four. The bordered box is inserted at the front with width equal to the
`x` `stack_columns` returns, and height `BOX_HEIGHT` as today. No
`filename_x`/`suffix_x`/`word_end` variables remain.

### Considered and rejected

- **A `gap_before` field (gap since the previous sibling only, first
  column's gap ignored) with the container owning its own left/right
  padding separately.** Closer to a flexbox `gap` + container `padding`
  model, but explicitly not what's wanted here: the box's own edges should
  keep coming from the columns themselves, not a separate container-level
  value.
- **A shared `const COLUMN_PADDING: u8` used uniformly instead of a
  per-column field.** Rejected — `padding` stays a per-column field so a
  later spec can give individual columns different spacing again; every
  literal just happens to pass `1` for now.
- **Looking up the filename column's placed `x` by indexing into the
  `Vec<Placement>` `stack_columns` returns (e.g. `placements[2].x`).**
  Works, but re-introduces a "build, then search/index" step of exactly the
  shape spec 160 and 163 removed elsewhere. Emitting the cursor from a
  field on the `Column` itself, inline in `stack_columns`, avoids it.

### Tests

- `stack_columns`: a column with `cursor: Some(offset)` emits a `Cursor`
  placement at that column's `x + offset`, immediately after the column's
  own placement; a column with `cursor: None` emits no `Cursor` placement.
- `stack_columns`: `padding` is applied symmetrically and additively
  between neighbouring columns (regression coverage carried over from
  spec 160).
- `layout::footer`: with all four columns built through one `stack_columns`
  call, the bordered box's width equals the stack's returned total exactly
  (no added constant); the cursor placement is present only when
  `model.cursor` is `Some`, and lands at the filename column's position
  plus the offset.
