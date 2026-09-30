# Flex: i returns to WRITE

## User Story

Lina has typed "Hello" and pressed Enter, so she's in MOVE. She decides it needs more, presses `i`, and `dre-flex` switches back to WRITE. She types " world", and it goes on the end, so her box reads "Hello world". Then she changes her mind about the whole thing and backspaces all the way into "Hello".

## Acceptance Criteria

- Pressing `i` in MOVE mode switches to WRITE mode.
- After `i`, typed characters are added to the end of the existing text.
- After `i`, Backspace can delete characters that were typed before Enter.

## Technical Design

Decisions:

- **`i` only switches the mode.** `move_key` gains one arm: `"i"` sets `mode = Write` and leaves `text` alone. Nothing else in `FlexState`, `flex::view` or `flex::run` changes.
- **Appending and deleting across Enter come for free.** The text is a single string, and it's only ever edited at its end (181). After `i`, `write_key` pushes onto that same string and pops from it. So new characters land after "Hello", and Backspace can reach characters typed before Enter. No cursor or "committed" marker is needed.
- **`i` is exactly `"i"`**, like the main editor's `KeyMatch::Exact("i")` (`state/input.rs`). `"I"` and every other key in MOVE still leave the state unchanged.

### `flex::state`: what changes

```rust
fn move_key(state: FlexState, key: &str) -> FlexState
```

- `"i"` → `FlexState { mode: FlexMode::Write, ..state }`.
- Any other key → the state unchanged (as in 182).

### Tests

One `reduce` unit test:

- In `Move` with text `"Hello"`, `"i"` switches to `Write` and the text is still `"Hello"`.
