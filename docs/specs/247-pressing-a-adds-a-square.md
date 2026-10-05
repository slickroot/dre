# Pressing "a" adds a square

## Problem

dre-flex is currently a blank canvas with no way to place any visible element on it. There is no key handling and no concept of a drawable shape yet.

## Technical Design

- `start()` tracks square count as a plain local, `let mut squares = 0;`, declared before the read loop.
- The read loop becomes a `match byte[0] { 0x03 => return Ok(()), b'a' => { ... squares += 1 }, _ => {} }`, replacing the current single `if` check.
- `kitty::transmit` and `kitty::place` are merged into one function, `kitty::draw(pixels, width, height, col, row, id) -> String`, that moves the cursor to `(col, row)` and sends a single chunked escape using `a=T` (transmit-and-display in one action), instead of a separate `a=t` transmit followed by an `a=p` place. `id` is threaded through explicitly instead of being hardcoded to `i=1`.
- `flex::mod` gets a shared `draw_rect(stdout, color: [u8; 4], width, height, col, row, id) -> io::Result<()>` that builds the solid-color pixel buffer and calls `kitty::draw`. `draw_background` becomes a thin wrapper: `draw_rect(stdout, BACKGROUND_RGBA, width, height, 0, 0, 1)`.
- A pure helper, `layout_rect(idx, window) -> (width, height, col, row)`, computes a square's pixel size (`2 * cell_width`, `1 * cell_height`) and its cell position (`col = 2 * idx`, `row = 0`) from its index, decoupled from any IO.
- On `b'a'`, the loop calls `layout_rect(squares, &window)` then `draw_rect(&mut stdout, SQUARE_RGBA, width, height, col, row, 2 + squares)` (image ids `2, 3, 4, ...`, since the background owns `id=1`), then increments `squares`.
- `SQUARE_RGBA` is a new constant, `[0x3F, 0x3F, 0x46, 0xFF]`, alongside the existing `BACKGROUND_RGBA`.

## Acceptance Criteria

- Pressing "a" on an empty canvas draws a square, 2 cells wide by 1 row tall, anchored at the top-left corner.
- The square is filled with color #3F3F46.
- Pressing "a" again adds another identically-sized square immediately to the right of the previous one.
- Each subsequent press of "a" continues adding squares to the right, in sequence.

## Out of scope

- Overflow/wrapping behavior when squares run out of horizontal space.
