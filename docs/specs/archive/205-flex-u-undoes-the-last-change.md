# Doug undoes his last change with u

## User Story

Doug is building a diagram in dre-flex. He adds a box he didn't want, so he presses `u` in move mode and it's gone. He presses `u` a few more times and watches his earlier changes come off one at a time, until he's back where he wanted to be. Happy, he carries on drawing!

## Acceptance Criteria

- In move mode, pressing `u` undoes the last change to the diagram: adding a box (`a`, `A`), adding a text (`s`, or `i` on an empty box), editing a text (`i`), or toggling width, spread, direction or fill (`w`, `g`, `d`, `f`).
- Pressing `u` again keeps going back one change at a time, all the way to how the diagram was when Doug opened dre-flex.
- Adding a text and typing into it count as one change, so after `s`, "Hello" and Enter, one `u` removes the whole text.
- Editing a text counts as one change, so after `i`, " World" and Enter on "Hello", one `u` brings back "Hello".
- Moving the selection (`h`, `j`, `k`, `l`) isn't a change, so `u` skips past it and undoes the last real change.
- Pressing `u` when there's nothing to undo does nothing.
- In write mode, `u` types the letter "u" like any other letter.

## Technical Design
This follows dre's undo (`src/state/history.rs`). Before each undoable key, a snapshot goes onto a history stack, and `u` pops it. All changes are in `src/flex/`.

### New module `src/flex/history.rs`

- **`Snapshot { boxes: Tree<FlexNode>, selected: Vec<usize> }`** is one history entry. Undo puts back the tree and the selection together, so the selection always points at a node that exists and no `nearest_existing` repair is needed. Any `h/j/k/l` moves made after the last change are dropped along with it.
- **`undoable(mode: FlexMode, key: &str) -> bool`** returns true only in `Move`, for `a`, `A`, `s`, `i`, `w`, `g`, `d` and `f`. Everything else returns false: `h/j/k/l`, `u`, `q`, Ctrl-C, Enter, Backspace, and every key in `Write`. Flex keeps raw string keys. There is no action enum.
- **`recorded(state, key, reduce)`** works like dre's: if `undoable(state.mode, key)`, it pushes a `Snapshot` of the current `boxes` and `selected`, then runs `reduce`.
- **`undo(state)`**: if `history.pop()` returns a snapshot, it restores `boxes` and `selected` from it. With an empty history it returns the state unchanged.

### State (`state.rs`)

- **`FlexState`** gains `history: Vec<Snapshot>`, empty in `Default`.
- **`PartialEq` for `FlexState` is written by hand.** It compares `boxes`, `selected` and `mode` and ignores `history`, so the existing whole-state assertions (`w_twice…`, `f_twice…`, `g_twice…`, `d_twice…`) keep passing unchanged.
- **`reduce`** sends the move-mode branch through `history::recorded(state, key, |s| move_key(s, key))`. Ctrl-C and write mode stay as they are and never record anything.
- **`move_key`** handles `"u" => state = history::undo(state)`. In write mode `u` reaches `write_key` and is typed like any other letter.

### Why the grouping criteria hold

- The snapshot is taken when `s` or `i` is pressed in move mode. Typing, Backspace and Enter in write mode never push, so `s` + "Hello" + Enter is one entry, and `i` + " World" + Enter on "Hello" is one entry.
- `i` on an empty box adds a text and switches to write. It's still one key, so it's one entry.
- Like dre's `EditLabel`, `i` always snapshots. Pressing `i` and then Enter without editing leaves one undo step that changes nothing visibly. This is accepted.

### Tests

`history.rs`:
- `undoable` is true in `Move` for `a A s i w g d f` and false for `h j k l u q`.
- `undoable` is false for every key in `Write`, including `u` and `s`.
- `undo` with an empty history leaves the state unchanged.

`state.rs`, one per acceptance criterion:
- `a` then `u`, `A` then `u`, `w/g/d/f` then `u`: the state equals the one before the key, selection included.
- `s`, "Hello", Enter, then `u`: the text is gone and the selection is back where it was before `s`.
- On "Hello": `i`, " World", Enter, then `u`: the text is "Hello" again.
- `i` on an empty box, "Hi", Enter, then `u`: the box has no text again.
- `a`, `w`, `f` then `u` three times gives `FlexState::default()`. A fourth `u` changes nothing.
- `w`, `j`, `k`, `l`, then `u`: the width is back to `Fit` and the selection is where it was before `w`.
- `u` on `FlexState::default()` leaves the state unchanged and has no effect.
- `u` in write mode types "u" and leaves `history` alone.
