# Selection outline is painted last

Maya selects a box that sits next to another box's border, or inside a
parent whose own chrome is painted after it in tree order. She expects
the selection outline (spec 236) to always be visible on top, but
instead part of it is hidden behind a neighboring box's border because
that border happens to be painted after the outline.

## Acceptance Criteria

- The selected box's outline is always visible in full, regardless of
  where the selected box sits in the tree relative to other boxes
  (sibling order, parent/child nesting).
- No other visual behavior changes: outline position, gap, and color
  are unchanged from spec 236/239.

## Technical Design

### Root cause

`paint()` (`src/flex/view.rs:255-381`) already chains the `outline`
placement last within its own return value (`border → highlight →
label → caret → outline`), but this only guarantees the outline draws
on top of *that same box's* own border/label. `scene()`
(`src/flex/view.rs:383-401`) builds the full frame by `flat_map`-ing
`paint()` over every box in pre-order tree traversal (parent pushed
before its children, `arrange()` at `src/flex/view.rs:203-251`). A
later sibling's or child's own border placement — pushed later in the
overall list, at the same z (`Placement.depth`, tree depth) — composites
on top of an earlier box's outline where they overlap. Within a single
box, outline and border never overlap (the 2px gap from spec 236 keeps
them apart), which is why this was never visible there; the collision
only happens across boxes.

### Fix

Stop building the outline inside per-box `paint()`. Compute it once in
`scene()`, after all other placements are collected, for
`state.selected` only, and push it as the absolute last entry in the
frame's placement list — guaranteeing it composites on top of every
other box regardless of tree position.

- **`src/flex/view.rs`, `paint()`**: remove the `outline` binding
  (lines ~331-351) and drop it from the `chain(...)` at the end of the
  function. `paint()` no longer knows about Move-mode selection outline
  at all.
- **`src/flex/view.rs`, `scene()`**: after collecting `rects: Vec<Arranged>`
  and building `placements` via the existing `flat_map`, look up the
  `Arranged` entry whose `path == state.selected` (`rects.iter().find(...)`).
  If found and `state.mode == FlexMode::Move`, build the outline
  `Placement` (same fields as before: `colour: FLEX_SELECTED_COLOUR`,
  `sides: ALL_SIDES`, `border: FLEX_BORDER`, `gap: true`, rect expanded
  by `OUTLINE_MARGIN` on each side, `depth` from that `Arranged`'s path
  length) and `.push()` it onto `placements` after the `collect()`,
  before wrapping in `vec![(window, placements)]`.
- No changes needed in `src/render/terminal.rs` or `virtual_terminal.rs`
  — this is purely an ordering fix in how the placement list is
  assembled, not a change to how placements are drawn or composited.
