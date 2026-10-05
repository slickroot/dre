# Capital A prepends a square

## Problem

Dana is editing her flex layout. She presses capital `A`, and a new square appears at the front of the row in a darker color (`#2A2A2E`), sliding every square she's already placed one step to the right to make room.

## Technical Design

The row of squares stays represented as counters, not a `Vec<Square>`. Replace the single `let mut squares = 0;` in `start()` (`src/flex/mod.rs:33`) with two counters:

```rust
let mut front_count: i64 = 0;
let mut back_count: i64 = 0;
```

Rename the existing color constant for symmetry and add the new one (`src/flex/mod.rs:7-8`):

```rust
const SQUARE_BACK_RGBA: [u8; 4] = [0x3F, 0x3F, 0x46, 0xFF];
const SQUARE_FRONT_RGBA: [u8; 4] = [0x2A, 0x2A, 0x2E, 0xFF];
```

No new helper functions and no new tests — reuse `layout_rect`/`draw_rect` exactly as they are, and handle both keys inline in the `match` in `start()`:

- `b'a'`: unchanged shape, just uses the new constant name and the combined count as the index:
  ```rust
  b'a' => {
      let idx = front_count + back_count;
      let (width, height, col, row) = layout_rect(idx, &window);
      draw_rect(&mut stdout, SQUARE_BACK_RGBA, width, height, col, row, 2 + idx)?;
      back_count += 1;
  }
  ```
- `b'A'`: increments `front_count`, then redraws every square (front and back) from scratch at its recomputed index, since prepending shifts everyone's column and id:
  ```rust
  b'A' => {
      front_count += 1;
      for idx in 0..(front_count + back_count) {
          let color = if idx < front_count { SQUARE_FRONT_RGBA } else { SQUARE_BACK_RGBA };
          let (width, height, col, row) = layout_rect(idx, &window);
          draw_rect(&mut stdout, color, width, height, col, row, 2 + idx)?;
      }
  }
  ```

This keeps `layout_rect`'s `(width, height, col, row)` purely index-derived (no change needed there), and reuses the existing `id = 2 + idx` scheme — since the full row is redrawn on every `A`, ids and columns are always recomputed consistently and never collide with the background's `id = 1`. Appending via `a` stays a single incremental `draw_rect` call since it never moves existing squares.

## Acceptance Criteria

- Pressing `A` (uppercase) inserts a new square at the leftmost position.
- All previously placed squares shift one slot to the right.
- The new square's color is `#2A2A2E`.
- Pressing lowercase `a` still appends to the end as before, unaffected.
- Pressing `A` repeatedly keeps inserting at the front each time, pushing the whole row right.

## Out of scope
