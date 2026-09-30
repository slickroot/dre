# Flex: inner boxes stay visible on a filled box

## User Story

Sami runs `dre-flex`, types "Hello" and presses Enter to switch to MOVE. They press `A` to add a box inside it, and the inner box appears with its border. They press `f` to fill the outer box. It fills with #141416, and the inner box's border stays visible on top of the fill. They press `f` again, the fill goes away, and the inner box is still there.

## Acceptance Criteria

- An inner box added before the outer box is filled keeps its border visible on top of the fill.
- An inner box added after the outer box is filled also shows its border on top of the fill.
- Removing the fill with `f` leaves the inner box's border visible.
- The outer box's fill stays #141416.

## Technical Design

**Root cause.** The terminal renderer sends every box tile to kitty at the same `BOX_Z` (`src/render/terminal.rs`). When images share a z, kitty stacks them by image ID, so paint order is ignored. Pressing `f` creates new fill tiles with higher IDs than the inner box's cached border tiles, and the outer fill covers the inner box. The flex view already pushes inner boxes after the outer box, but that order never reaches kitty.

**Decision: z comes from depth in the tree.** A node is drawn above its parent because it is deeper in the tree. Since spec 198, texts are `FlexNode` children of their box, so the same rule puts a text above the fill it sits on.

- `Placement` (`src/view.rs`) gains `depth: u8`, next to `x`/`y`/`width`/`height`.
- `flex::view` sets `depth` from the node's path: `path.len() - 1`. An outer box is depth 0, and its texts and inner boxes are depth 1. `FlexBox`/`FlexState` get no new state.
- The terminal renderer draws boxes and labels at `z = depth`. `BOX_Z` goes away for them.
- Other scene producers (main editor, etc.) set `depth: 0` on boxes and `depth: 1` on the labels inside them, so their text stays above their fills. Other node kinds keep their own fixed z, moved above the depth range so they still draw on top.
- The SVG renderer ignores `depth`; it already paints in scene order.

**Tests**

- Flex view: the outer box has depth 0, and its texts and inner box have depth 1, both before and after the box is filled.
- Flex view: an inner box added after `f` also has depth 1.
- Renderer: a placement with depth 1 is placed at a higher z than one with depth 0.
- Existing fill-colour test (`a_filled_box_is_solid_with_the_flex_fill_colour`) still guards #141416.
