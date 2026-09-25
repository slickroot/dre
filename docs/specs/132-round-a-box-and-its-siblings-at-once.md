# Round a box and its siblings at once

## Story

Nadia has a row of five service boxes under "API gateway". She selects one of
them and presses `R`, and all five get rounded corners together, so she doesn't
have to round each box one at a time.

## Acceptance Criteria

- `R` on a box whose row is all square or mixed rounds every box in the row.
- `R` on a box whose row is all rounded makes every box in the row square.
- `R` changes only the selected box and the boxes that share its parent. Their
  children and every other box stay as they are.
- `R` works on a row of top-level boxes too.
- One `u` undoes the whole row's change.
- `R` with no box selected does nothing.
- `R` appears in the keymap as "Toggle rounded corners of every sibling".

## Technical Design

This is the rounded twin of `toggle_siblings_fill`. `Document::set_rounded`
already takes a `Scope`, so `Scope::Siblings` needs no new `Document` API. The
row decision stays inline in the reducer, as it does for fill. No shared
"row is uniform" helper: `next_row_colour` compares colours and this compares
booleans, so nothing else would use one yet.

- `src/state/action.rs`: add `Action::ToggleSiblingsRounded`, listed with the
  other siblings actions.
- `src/state/input.rs`:
  - `command_parse` maps `"R"` to `Action::ToggleSiblingsRounded`.
  - Add a `KeyBinding` with keys `["R"]` and description
    "Toggle rounded corners of every sibling", right after the `r` binding.
- `src/state/command.rs`:
  - New reducer:
    ```rust
    fn toggle_siblings_rounded(mut state: State, path: Vec<usize>) -> State {
        let tree = state.doc.tree();
        let all_rounded = children(tree, parent_of(&path))
            .all(|sibling| tree.value(&sibling).rounded());
        state.doc.set_rounded(&path, !all_rounded, Scope::Siblings);
        state.selected = Some(path);
        state
    }
    ```
    A row that is all rounded becomes square; a row that is all square or
    mixed becomes rounded.
  - `reduce` handles `(Action::ToggleSiblingsRounded, Some(path))`. A `None`
    selection falls through to the existing no-op.
  - `min_depth` keeps the default of 1 for it, so it works on top-level boxes.
    `children(tree, parent_of(&[0]))` is the root's children. It does not join
    the depth-2 guard that `C` and `F` use.
  - Add it to the test-only list of actions that covers the min-depth guard.
- `src/state/history.rs`: add `Action::ToggleSiblingsRounded` to
  `is_undoable`, so one `u` restores the whole row from a single snapshot.
- Tests:
  - `command.rs` reducer tests: a square row all becomes rounded; a mixed row
    all becomes rounded; a rounded row all becomes square; children of the row
    and boxes in other rows are untouched; a top-level row works; nothing
    selected is a no-op; one `u` restores the whole row.
  - `input.rs`: `command_parse("R")` maps to `ToggleSiblingsRounded`, and the
    keymap lists it with the description above.
- No changes to `diagram.rs`, `layout.rs`, `render/`, or `dre_format.rs`:
  rounding is already stored per node, laid out and drawn.
