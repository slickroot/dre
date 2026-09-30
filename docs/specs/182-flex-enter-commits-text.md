# Flex: Enter commits the text

## User Story

Lina runs `dre-flex` and types "Hello" into the box. Happy with it, she presses Enter, and `dre-flex` switches from WRITE to MOVE. She taps a few letters and Backspace, and her box still says "Hello", untouched. She presses Ctrl-C to leave.

## Acceptance Criteria

- Pressing Enter in WRITE mode switches to MOVE mode.
- The text typed before Enter stays in the box.
- In MOVE mode, typed characters don't change the box's text.
- In MOVE mode, Backspace doesn't change the box's text.
- In MOVE mode, Ctrl-C quits without saving.

## Technical Design

Decisions:

- **`FlexMode` gains `Move`.** Nothing else changes in `FlexState`. The text stays where it is, so "committing" it is just the mode switch.
- **`reduce` splits by mode.** Ctrl-C is handled first and works the same in every mode. Other keys go to a handler for the current mode. Specs 183 (`i`) and 184 (`q`) only add arms to `move_key`.
- **Enter is exactly `"\r"`**, as in the main editor (`state/input.rs`). `"\n"` (Ctrl-J) is just another control byte and is ignored.
- **No view or loop changes.** 181 decided the mode isn't drawn, so `flex::view` and `flex::run` stay as they are.

### `flex::state`: what it knows and does

```rust
pub(crate) enum FlexMode { Write, Move }

pub(crate) fn reduce(state: FlexState, key: &str) -> FlexState
fn write_key(state: FlexState, key: &str) -> FlexState
fn move_key(state: FlexState, key: &str) -> FlexState
```

- `reduce`: `"\x03"` sets `running = false` in any mode. Other keys go to `write_key` or `move_key`, depending on `state.mode`.
- `write_key`: the WRITE behaviour from 181, moved over unchanged (push a printable char, pop on `"\x7f"`, ignore everything else), plus `"\r"`, which sets `mode = Move` and leaves `text` alone.
- `move_key`: returns the state unchanged for every key.

### Tests

Only `reduce` unit tests, starting from a state with text `"Hello"`:

- In `Write`, `"\r"` switches to `Move` and the text is still `"Hello"`.
- In `Write`, `"\n"` leaves the state unchanged.
- In `Move`, a printable char leaves the text as `"Hello"` and the mode as `Move`.
- In `Move`, `"\x7f"` leaves the text as `"Hello"`.
- In `Move`, `"\x03"` stops running.
