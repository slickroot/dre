# Flex: `p` pastes the cut box

## User Story

Noor is arranging boxes in dre-flex. She cuts a box she no longer wants with `d` — it vanishes and her parent is selected. Later she changes her mind, selects the box she actually wants it in, and presses `p`. The cut box reappears inside it, exactly as it was, with everything that was nested inside still nested. She presses `p` again and drops another copy, then realises she overdid it, presses `u`, and the last paste is gone. Happy, she carries on drawing!

## Acceptance Criteria

1. In Move mode, with a box on the clipboard and a box selected, `p` pastes the clipboard box as the last child of the selected box.
2. The pasted box keeps everything that was inside it, plus its label, border, fill, padding, justify, and direction.
3. The clipboard is kept, so `p` can paste again.
4. After pasting, the box that received the paste stays selected.
5. With the canvas selected and a box on the clipboard, `p` adds it as a new top-level box, and the canvas stays selected.
6. With nothing on the clipboard, `p` leaves the diagram and the selection unchanged.
7. One `u` after a paste removes the pasted box.
8. In Write and Replace modes, `p` is typed as the letter "p" and pastes nothing.

## Technical Design

### Overview

`p` is a Move-mode command in the flex reducer. It is purely additive: the
clipboard already exists (`FlexState::clipboard`), `d` already fills it, and
`reduce` already routes Write/Replace keys to `write_key`, so `p` is typed in
those modes with no extra work. We add one paste helper, one `move_key` arm,
and make `p` an undoable Move key.

### Components and responsibilities

- `state::paste_clipboard(state: FlexState) -> FlexState` (new, in
  `src/flex/state.rs`, alongside `cut_selected`)
  - **Knows:** nothing beyond the state it is given.
  - **Does:** if `state.clipboard` is `Some(branch)`, calls
    `state.boxes.push(&state.selected, branch.clone())`. Because an empty
    `state.selected` is the canvas, the same call appends a top-level box when
    the canvas is selected. It never reassigns `state.selected`, so the
    receiving box (or the canvas) stays selected. It clones the branch and
    leaves the clipboard intact, so `p` can be pressed again.
  - **Collaborators:** `types::Tree::push`.

- `move_key` (`src/flex/state.rs`)
  - Add arm `"p" => state = paste_clipboard(state)`.
  - No change to the other arms.

- `history::undoable` (`src/flex/history.rs`)
  - Add `"p"` to the `FlexMode::Move` match arm. The snapshot is pushed by
    `history::recorded` before `move_key` runs, so it is taken **even when the
    clipboard is empty** — matching the existing `J`/`K` boundary convention.

### Behavior mapping to acceptance criteria

1. Move mode, clipboard + box selected: `push` appends as the last child.
2. The clipboard holds the whole removed `Tree`, so nested contents and all
   `FlexBox` fields (text, border, fill, padding, justify, direction) survive
   the clone.
3. `paste_clipboard` clones and does not `take`, so the clipboard is retained.
4. `state.selected` is never reassigned, so the receiver stays selected.
5. Empty `state.selected` means `push(&[], …)`, adding a top-level box; the
   selection stays `[]` (canvas).
6. `clipboard == None` → `paste_clipboard` is a no-op on `boxes`/`selected`;
   the undo snapshot is still recorded (decision above).
7. `u` pops the snapshot taken before `p`, restoring `boxes` and `selected`.
   The clipboard is not part of a snapshot and is untouched.
8. Write/Replace are routed to `write_key` before `move_key`, so `p` is typed
   and never reaches paste.

### Tests

In `src/flex/state.rs`:

- Delete `y_and_p_in_move_mode_leave_the_state_unchanged_and_return_no_effect`
  (the `y` half is covered by spec 230).
- `p` appends the cut branch as the last child of the selected box, preserving
  its nested contents and every `FlexBox` field, and leaves that box selected
  while the clipboard still holds the branch.
- `p` with the canvas selected adds a top-level box and keeps the canvas
  selected.
- `p` with an empty clipboard leaves `boxes` and `selected` unchanged but adds
  one history snapshot (so a following `u` pops it).
- `p` twice appends two independent copies.
- `u` after `p` restores the boxes and selection from before the paste and
  leaves the clipboard filled.
- `p` in Write mode is typed into the box's text and pastes nothing.

In `src/flex/history.rs`:

- Add `"p"` to `box_text_and_toggle_keys_are_undoable_in_move_mode`.
- Remove `"p"` from `selection_undo_and_quit_keys_are_not_undoable_in_move_mode`.
