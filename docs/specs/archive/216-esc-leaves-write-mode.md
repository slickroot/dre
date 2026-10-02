Lina is typing a label into a box in write mode. She's happy with what she's typed but doesn't want a new box to pop up after it, so instead of pressing Enter she presses Esc. The box keeps her text and she's back in move mode, ready to navigate. Another time, she opens a fresh empty box, changes her mind, and presses Esc — the box disappears instead of leaving an empty one behind.

## Acceptance Criteria

- Pressing Esc in write mode with non-empty text returns to move mode, keeping the typed text, and does not create a sibling box.
- Pressing Esc in write mode with empty text removes (or clears) the box, same as Enter does today, and returns to move mode.
- This behavior is the same regardless of how write mode was entered (`i`, `o`, or the Enter-adds-a-sibling flow).

## Technical Design

Esc reuses the same exit-to-move-mode logic as Enter, factored into a shared helper so Enter can later diverge (spec 214's sibling-insertion) without Esc inheriting that behavior.

- In `src/flex/state.rs`, extract a new helper `exit_write_mode(state: FlexState) -> FlexState` next to `drop_empty_text`:
  - If the selected box's `text == Some("")`, call `drop_empty_text(state)`.
  - Otherwise keep `text` as-is.
  - Set `state.mode = FlexMode::Move`.
- `write_key`'s `"\r"` arm is rewritten to call `exit_write_mode(state)` (same observable behavior as today).
- A new `"\x1b"` arm is added to `write_key` that also calls `exit_write_mode(state)`.
- No change to undo/history: write-mode keys remain unrecorded, same as Enter today (see `no_key_is_undoable_in_write_mode`).
- This covers all entry points (`i`, `o`, future Enter-adds-sibling flow) since they all land in `FlexMode::Write`, dispatched through the same `write_key` function.
