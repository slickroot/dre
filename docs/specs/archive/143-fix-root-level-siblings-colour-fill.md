## User Story

Doug draws a row of boxes at the root level of his diagram, with no parent box selected. He selects one of the root-level boxes and presses `C`; the colour cycles across all the root-level boxes in that row. He then presses `F`; the fill toggles across all of those same root-level boxes. Both behave exactly like `R` already does for rounded corners at the root level.

## Acceptance Criteria

- Selecting any root-level box and pressing `C` cycles the colour of all root-level boxes in that row (not just the selected one).
- Selecting any root-level box and pressing `F` toggles the fill of all root-level boxes in that row (not just the selected one).
- Both match the existing behavior of `C` and `F` on nested sibling boxes, and the existing behavior of `R` at the root level.
- The existing tests asserting `C` and `F` do nothing on a top-level box are updated to reflect the new expected behavior.

## Technical Design

The sibling-read logic for `C` and `F` (`children(tree, parent_of(path))` in `next_row_colour` and `toggle_siblings_fill`, `src/state/command.rs`) is already depth-agnostic: at the root, `parent_of(path)` returns the empty path, and `children(tree, &[])` correctly enumerates root-level boxes. `R`'s handler (`toggle_siblings_rounded`) uses the identical pattern and already works at the root.

The bug is isolated to `min_depth` (`src/state/command.rs`, lines 7-13), which gates `Action::CycleSiblingsColour` and `Action::ToggleSiblingsFill` behind depth ≥ 2, while `Action::ToggleSiblingsRounded` falls through to the default depth ≥ 1 arm:

```rust
pub(crate) fn min_depth(command: Action) -> usize {
    match command {
        Action::Undo | Action::NewBox | Action::Paste | Action::Quit => 0,
        Action::SelectParent | Action::CycleSiblingsColour | Action::ToggleSiblingsFill => 2,
        _ => 1,
    }
}
```

Fix: remove `Action::CycleSiblingsColour` and `Action::ToggleSiblingsFill` from the depth-2 arm, so both fall into the default `_ => 1` arm alongside `Action::ToggleSiblingsRounded`. No other code changes are needed.

```rust
pub(crate) fn min_depth(command: Action) -> usize {
    match command {
        Action::Undo | Action::NewBox | Action::Paste | Action::Quit => 0,
        Action::SelectParent => 2,
        _ => 1,
    }
}
```

Existing tests `capital_c_on_a_top_level_box_does_nothing` and `capital_f_on_a_top_level_box_does_nothing` (`src/state/command.rs`) are rewritten to assert the new expected behavior, mirroring the existing `capital_r_works_on_a_row_of_top_level_boxes` test: pressing `C`/`F` on a selected root-level box with siblings cycles colour / toggles fill across all root-level boxes in that row, and preserves the selection.
