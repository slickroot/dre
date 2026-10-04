## User Story

Maya selects a box on the flex canvas (in Move mode). Instead of the box's border or label text changing color, she sees a thin outline drawn around the box with a small visible gap between the outline and the box's own edge — the same treatment whether the box has a border of its own or is a plain text leaf. When she selects a different box, the outline moves to it and the previous box returns to its normal appearance.

## Acceptance Criteria

1. Selecting any box in Move mode draws a rectangle outline around it, offset outward by a small visible gap from the box's actual edges — this applies the same way to bordered boxes and borderless text leaves.
2. The box's own border color and label text color are no longer changed when it's selected (removing the current blue-recolor behavior).
3. The outline uses the existing selection blue color.
4. Deselecting / moving selection to another box removes the outline from the old box and draws it on the new one.
5. This applies only to Move mode (Replace mode's text-highlight behavior is untouched).

## Technical Design

### Overview

The current selection behavior recolors the box's own border and/or label to `FLEX_SELECTED_COLOUR` (`src/flex/view.rs:257, 271-275, 323-327`). This is replaced with a separate, additional outline placement, following the same pattern already used for the Replace-mode `highlight` block (`src/flex/view.rs:287-314`): a new `PlacementNode::Box` placement is appended to the `paint()` chain, independent from the box's own `border`/`label` placements.

### Changes in `src/flex/view.rs`

- Remove the `selected` colour-swap on the `border` placement (lines 271-275) and on the `label` placement (lines 323-327). The box's own border always renders `FLEX_BORDER_COLOUR`; the label always renders `FLEX_TEXT_COLOUR`, regardless of selection.
- Add a new `outline` placement to the `paint()` chain:
  - Condition: `state.mode == FlexMode::Move && &state.selected == path` (same condition as the old `selected` check) — applies uniformly whether `flex_box.border` is true or false, since it's driven off `arranged.rect`, which every node has.
  - Rect: `arranged.rect` expanded outward by exactly 1 cell on each side (`x - 1, y - 1, width + 2, height + 2`), i.e. one extra cell of placement room around the box, same pattern as the Replace-mode `highlight` block's derived rect.
  - `PlacementNode::Box` fields: `colour: FLEX_SELECTED_COLOUR`, `sides: ALL_SIDES`, `border: FLEX_BORDER` (same thickness as a normal border), `fill: None`, `solid_fill: None`, `opacity: None`, `rounded: false` (dre-flex has no rounded borders), `grow: false`, and a new `gap: true`.
  - This placement is only ever added for the selected box in Move mode; all other `Box` placements (the box's own border, the Replace-mode highlight) pass `gap: false`.

No clamping/clipping logic is needed for top-level boxes: the canvas always reserves a full cell of margin between itself and the terminal window edge, so the outline's extra cell always has room.

### Changes in `src/render/terminal.rs`

- Add `gap: bool` to `BoxStyle` (and to `PlacementNode::Box` in `src/view.rs`), threaded through to `box_shape`/`box_canvas`.
- Add `gap` to the `SpriteKey::Box` cache key (`box_key()`, ~lines 204-216) so gapped and non-gapped boxes with otherwise-identical styles don't collide in the sprite cache.
- In `box_canvas`, when `gap` is true, shrink the effective outer boundary used for stroke/coverage computation by a fixed `GAP_PX = 2` (pixels) on each side, using the already-in-scope `self.window.cell_width`/`cell_height` to convert. This extends the existing inward inset pattern (`BoxShape`'s stroke→fill insets in `shapes.rs`) with an equivalent outward margin (canvas edge → stroke): anything within that margin band stays fully transparent (coverage 0).
- Net visual result per selected box: unchanged real border → 2px transparent gap → outline stroke in `FLEX_SELECTED_COLOUR`, all contained within the one extra cell of placement space added around the box.
