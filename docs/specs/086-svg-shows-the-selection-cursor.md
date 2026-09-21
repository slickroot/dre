# 086: SVG shows the selection cursor

## User Story

When I'm using dre in the browser (the web editor on the website) and a box is selected,
I see the same block cursor over that box's label as in the terminal, so I always know
which box is selected.

## Acceptance Criteria

1. When a box is selected in the web editor, the block cursor shows over its label, the
   same as in the terminal.
2. When I move the selection to another box, the cursor moves with it.
3. The cursor is visible on the website's dark background.

## Technical Design

Mirror the terminal renderer: the cursor is a `PlacementNode::Cursor` placement produced by
`layout::with_cursor`, and each renderer decides how to paint it.

### Changes (all in `src/render/svg.rs`)

- `SvgRenderer::render` builds the placements as the terminal renderer does:
  `with_cursor(layout(&doc.boxes), doc.selected.clone())`, then passes them to `draw`.
- `draw` stays selection-agnostic (it only sees placements). It gets one new loop, after the
  label loop, so the cursor paints over the label like in the terminal:
  `PlacementNode::Cursor(_)` → `<rect>` at `x * CELL_WIDTH`, `y * CELL_HEIGHT`, size
  `CELL_WIDTH` × `CELL_HEIGHT`, `fill="var(--ink)"`.
- `view_box` is unchanged: the cursor sits inside its box, so it can't extend the bounds.

### Decisions

- **Always drawn when `doc.selected` is set.** No opt-in flag. CLI exports (`docs/*.svg`) load
  documents with `selected: None`, so they stay cursor-free.
- **Solid ink block, no blink or accent colour.** `--ink` already flips with
  `prefers-color-scheme`, which satisfies the dark-background criterion (AC 3).
- **Moving the selection (AC 2)** needs no extra code: the web editor re-renders the SVG on every
  key press and `with_cursor` follows `doc.selected`.
- **`web/src/lib.rs` is unchanged.** It already passes `session.document()` to `SvgRenderer`.

### Collaborators

- `layout::with_cursor` / `PlacementNode::Cursor` (existing, reused as-is)
- `CELL_WIDTH`, `CELL_HEIGHT` (existing render constants)

### Tests (`src/render/svg.rs`)

- Replace `a_selection_does_not_change_the_svg` with: a selected box's SVG contains the cursor
  rect at the label end, filled with `var(--ink)`.
- No selection → no cursor rect.
- Moving the selection to another box moves the cursor rect.
- `draw` given a cursor placement paints the rect after the labels (paint-order test).
