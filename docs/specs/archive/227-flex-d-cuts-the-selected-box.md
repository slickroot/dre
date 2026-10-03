# Flex: `d` cuts the selected box

## User Story

Doug is arranging boxes in dre-flex. One of them isn't working out, so in Move
mode he selects it and presses `d`. The box and everything inside it disappear,
and the box that held it is now selected. If it was sitting straight on the
canvas, nothing is highlighted. He changes his mind, presses `u`, and it all
comes back. Happy, he carries on drawing!

## Acceptance Criteria

1. In Move mode with a box selected, `d` removes the selected box together with
   everything nested inside it.
2. After `d`, the selection moves to the box's parent.
3. If the removed box sat directly on the canvas, the selection becomes the
   canvas and no box is highlighted.
4. Cutting away the last box leaves an empty canvas.
5. In Write mode, `d` is typed into the text and removes nothing.
6. If the canvas is selected (no box highlighted), `d` does nothing.
7. One `u` after a cut brings the box and its contents back and restores the
   selection.

## Technical Design

### Decision summary

- `d` is a **real cut**, not a plain delete: the removed subtree is stored on a
  new `clipboard` field of `FlexState` so a future `y`/`p` spec can paste it.
- The clipboard is **not** part of the undo snapshot. One `u` restores the tree
  and selection; the clipboard stays filled (mirrors the legacy editor).
- `"d"` is added to the Move-mode `undoable` list, so the snapshot is taken
  before the cut. Pressing `d` on the canvas still records one no-op snapshot
  (legacy behavior); AC 6 means no *visible* change, not no history entry.
- The cut is a named helper `cut_selected`, next to the other small state
  helpers (`drop_empty_text`, `sibling_or_else_parent`, `parent_path`).
- `FlexState::PartialEq` is **unchanged** (ignores `clipboard` and `history`).
  Tests read `state.clipboard` directly to assert the cut contents.
- `d` cuts **any** selected node — bordered box, box with children, or a
  borderless text leaf. The only guard is `selected.is_empty()`.

### Components and responsibilities

**`FlexState` (`src/flex/state.rs:92`)** — the model, gains one field:

```rust
pub(crate) struct FlexState {
    pub(crate) boxes: Tree<FlexBox>,
    pub(crate) selected: Vec<usize>,          // [] == canvas selected
    pub(crate) mode: FlexMode,
    pub(crate) clipboard: Option<Tree<FlexBox>>, // NEW: last cut subtree
    pub(crate) history: Vec<Snapshot>,
}
```

- `Default` initializes `clipboard: None` (`state.rs:106`).
- `PartialEq` (`state.rs:100`) is **not** changed: it keeps comparing only
  `boxes`, `selected`, `mode`.

**`cut_selected` (`src/flex/state.rs`, near `parent_path` at `:304`)** — owns
the cut:

```rust
fn cut_selected(mut state: FlexState) -> FlexState {
    state.clipboard = Some(state.boxes.remove(&state.selected));
    state.selected = parent_path(&state.selected);
    state
}
```

- Knows: the selected path and the tree.
- Does: detaches the whole subtree (children included), stores it on the
  clipboard, and moves the selection to the parent.
- Collaborators: `Tree::remove` (detach + return subtree) and `parent_path`
  (`state.rs:304`), which yields `[]` for a top-level node, giving the canvas.

**`move_key` (`src/flex/state.rs:240`)** — gains one arm before `_ => {}`:

```rust
"d" => {
    if !state.selected.is_empty() {
        state = cut_selected(state);
    }
}
```

- The empty-path guard prevents `Tree::remove`/`selected_mut` from panicking on
  the root path and implements AC 6.

**`history::undoable` (`src/flex/history.rs:12`)** — `"d"` is added to the
Move arm:

```rust
FlexMode::Move => matches!(key, "a" | "A" | "o" | "O" | "i" | "s" | "r" | "f" | "]" | "d"),
```

`recorded` (`history.rs:18`) then snapshots tree + selection before `move_key`
runs, so AC 7 holds. `Snapshot` (`history.rs:5`) and `undo` (`history.rs:39`)
are **unchanged** — the clipboard is deliberately left filled after undo.

**`view.rs`** — no production change. `selected == []` already paints nothing
(`the_window_root_paints_nothing`, `view.rs:1909`), and after a nested cut the
parent is highlighted by the existing `selected == path` check (`view.rs:257`).

### Dependencies / collaborators

- `types::Tree::remove` (`types/src/tree.rs:74`) — returns the detached
  `Tree<FlexBox>`; panics on `[]`, hence the guard.
- `parent_path` (`state.rs:304`) — not `Tree::parent`, which clamps top-level
  paths to themselves instead of returning `[]`.
- `history::recorded` / `undoable` — snapshot policy only; clipboard not
  involved.
- No new crate dependencies; no changes to `FlexMode`, `FlexEffect`, or the
  reducer dispatch (`state.rs:131`), because Write/Replace are matched before
  the Move catch-all (AC 5 falls out for free).

### Test plan

Update existing tests that pin the old no-op:

- `state.rs:1816` `d_y_and_p_...` — drop `"d"` from the loop (keep `"y"`, `"p"`),
  and rename to `y_and_p_...`.
- `history.rs:51` `box_text_and_toggle_keys_are_undoable_in_move_mode` — add
  `"d"`.
- `history.rs:58` `selection_undo_and_quit_keys_are_not_undoable...` — remove
  `"d"`.

New tests, mapped to the acceptance criteria:

1. AC 1 — `d_cuts_the_selected_box_and_its_nested_contents`: build a box with a
   child (e.g. `hello_box_world`), select it, press `d`; assert the subtree is
   gone from `boxes` and `state.clipboard` equals the removed `Tree`.
2. AC 2/3 — `d_selects_the_parent_after_a_nested_cut` and
   `d_on_a_top_level_box_selects_the_canvas`: assert `selected == parent` and,
   for top level, `selected == vec![]`.
3. AC 4 — `d_on_the_only_box_leaves_an_empty_canvas`: `boxes` has no children
   and `selected` is `[]`.
4. AC 5 — `d_in_write_mode_is_typed_and_removes_nothing`: `reduce` in Write mode
   appends `d` to the label; `boxes`/`clipboard` unchanged.
5. AC 6 — `d_on_the_canvas_changes_nothing`: `selected == []` in, `d`, assert
   `boxes`/`selected`/`clipboard` unchanged.
6. AC 7 — `u_after_d_restores_the_box_and_its_contents_and_selection`: one `u`
   restores `boxes` and `selected`; assert `clipboard` stays `Some(..)`.

### Merge note

Specs 225 (`o`/`O` copy) and 226 (`a`/`A`) live in worktrees and rewrite the
same regions — the `move_key` `"a"`/`"o"` arms, the `undoable` Move list
(`history.rs:12`), and the `d_y_and_p` test. Rebase/merge order must be
coordinated; the `"d"` arm and its history entries should be added last to
avoid textual conflicts.
