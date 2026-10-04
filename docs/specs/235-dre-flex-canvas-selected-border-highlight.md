Maya selects the canvas itself (not one of its boxes). Its border switches to the selected highlight colour, the same way a selected box's border would, so she can tell the canvas is the current selection.

## Acceptance Criteria

- When the canvas is the selected item, its border colour becomes `FLEX_SELECTED_COLOUR`.
- When the canvas is not selected, its border stays the normal border colour.

## Technical Design

This spec depends on spec 234 (canvas border always visible), which
changes the canvas node (path `[0]`) in `new_canvas()` to use
`FlexBox::default()` (`border: true`) instead of `FlexBox::window()`.

Once 234 lands, no new logic is needed for 235. `paint()`'s existing
colour decision (`src/flex/view.rs:257`):

```rust
let selected = state.mode == FlexMode::Move && &state.selected == path;
```

is generic over `path` — it is not box-specific — so it already applies
to the canvas's path (`[0]`) the same as any box's path. Combined with
`is_canvas(path) == (path.len() == 1)` (`src/flex/state.rs:167-169`),
selecting the canvas sets `state.selected = [0]`, which this check
picks up directly: the border placement at `view.rs:271-275` will use
`FLEX_SELECTED_COLOUR` when the canvas is selected and
`FLEX_BORDER_COLOUR` otherwise, with zero changes to `paint()`.

Canvas selection only ever occurs within `FlexMode::Move` (see the
`is_canvas(&state.selected)` guard at `src/flex/state.rs:161`, reachable
only from the `FlexMode::Move` branch of the dispatch table), so there
is no mode under which the canvas is selected but this check fails to
fire.

Implementation work for 235, once 234 is merged, is limited to adding a
test confirming `paint()` colours the canvas border with
`FLEX_SELECTED_COLOUR` when `state.selected == [0]` and
`state.mode == FlexMode::Move`, and with `FLEX_BORDER_COLOUR` otherwise.
