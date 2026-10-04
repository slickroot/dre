# [ key decreases padding

Doug selects a box that has some padding on it, and presses `[`. He sees the padding shrink by one unit on all sides. He presses `[` again and the padding shrinks further. He keeps pressing `[` until the padding reaches zero, and further presses leave it at zero. Earlier, while typing text into a box, Doug pressed `[` and it was simply typed into the text as a character, leaving the padding untouched. Happy, he goes back to sleep!

## Acceptance Criteria

- Pressing `[` on a selected box with padding greater than 0 decreases the padding by one unit, equally on all sides.
- Pressing `[` repeatedly keeps decreasing the padding until it reaches 0.
- Pressing `[` when padding is already 0 leaves it at 0 (no negative padding).
- In Write mode, pressing `[` types a literal `[` character into the box's text and does not change padding.

## Technical Design

This mirrors the existing `]` (increase padding) command in `src/flex/state.rs`, which already distinguishes Move mode (box selected) from Write/Replace mode (text editing) via `FlexMode`, and already renders a box's size purely from its `padding: u16` field (`src/flex/view.rs`). No new modal-state or rendering logic is needed.

- **`src/flex/state.rs`, `move_key`**: add a `"["` arm:
  ```rust
  "[" => {
      let selected = state.selected_mut();
      selected.padding = selected.padding.saturating_sub(1);
  }
  ```
  `saturating_sub` naturally floors at 0, satisfying "no negative padding" without an explicit check.

- **`src/flex/state.rs`, `canvas_self_edit()`**: remove `"]"` from the blocked-key list (currently `"i" | "o" | "O" | "s" | "r" | "f" | "]" | "d" | "J" | "K" | "y"`), and do not add `"["` to it. This allows both padding brackets to act on the canvas root, while direction (`r`), border (`f`), insert/split (`i`/`o`/`O`/`s`), delete (`d`), move (`J`/`K`), and yank (`y`) remain blocked there as before.

- **`src/flex/history.rs`, `undoable()`**: add `"["` to the `FlexMode::Move` match arm alongside `"]"`, so decreasing padding is undoable with `u`, consistent with increasing it.

- **Write/Replace mode**: no change needed. `write_key` has no special case for `[`, so it already falls through to the default printable-char-insert branch, typing `[` literally into the box's text and leaving padding untouched.

- **Rendering**: no change needed. `view.rs`'s `padding()` function already computes box size purely from `flex_box.padding`, and the main loop (`src/flex/mod.rs` `run_loop`) re-renders after every `reduce` call automatically.
