# Selected box is framed by corner brackets

## Story

Lina opens her diagram in dre in the terminal and moves around her boxes. The box she's on no longer has a blurry glow. Instead it's framed by four crisp white corner brackets, like a camera viewfinder. They stay on the box whether she's moving around, writing a box's name, or naming her file. Everything feels sharp and in tune with the rest of dre, and she keeps drawing, happy.

## Acceptance Criteria

- In the terminal app, the selected box is framed by four corner brackets drawn in the foreground colour.
- The brackets look like "01 reticle" in the selection scratchpad, except for their thickness: each bracket is an L of two solid arms 14 px long and as thick as the box's border, set 6 px out from the box's outer edge on both axes. Rounded boxes get the same square brackets.
- The brackets show on the selected box in every mode: MOVE, WRITE and NAME.
- The glow ring no longer appears around the selected box.

## Technical Design

The glow is replaced end to end by a new decoration, `Brackets`. The layout places it, and both renderers draw it.

### View model — `src/view.rs`

- `PlacementNode::Glow { colour, rounded }` is replaced by `PlacementNode::Brackets { border }`. The brackets are always the foreground colour and always square, so they need nothing from the box except its border thickness.
- `GLOW_MARGIN` is renamed `BRACKET_MARGIN` (still 1 cell).
- `Brackets` is a decoration (`is_decoration`), so `centre()` still ignores it when measuring the diagram.

### Layout — `src/layout/tree.rs`

- `emit` pushes `Brackets { border: BORDER }` right after the selected box. Its geometry is the box grown by `BRACKET_MARGIN` on every side, the same cell geometry the glow had.
- Selection doesn't depend on mode (`State::selected()`, and `body()` always passes it along), so the brackets already show in MOVE, WRITE and NAME. A `body()` test for each mode (`Command`, `Insert`, `NamePrompt`) pins that down.

### Shared dimensions — `src/render/mod.rs`

- `BRACKET_OFFSET = 6` (px out from the box's outer edge on both axes) and `BRACKET_ARM = 14` (px length of each arm) sit next to `ROUNDED_RADIUS`. Both renderers read them, so each number is stated once. The thickness is the `border` carried by the node.

### Terminal renderer — `src/render/{shapes,tiles,terminal}.rs`

- **`BracketsShape`** (shapes.rs) knows `width`, `height`, `margin_x`, `margin_y` (one cell in px), `offset`, `arm`, `thickness`, `colour`. Its `colour_at` paints the four solid Ls in opaque foreground and nothing else. There's no anti-aliasing because every edge is axis-aligned. It replaces `GlowShape`.
- **`edge_extents`** returns `(margin_x + arm - offset, margin_y + arm - offset)`. Each arm reaches 8 px past the box corner, so the corner tiles hold the whole bracket and the middle tiles are transparent. The tile cache stays small and is shared by every box size.
- **Tiles** (tiles.rs): `TileStyle::Glow(GlowStyle)` becomes `TileStyle::Brackets(BracketStyle { border })`. The existing "composing the tiles gives exactly the whole sprite" and band tests cover the new style through `styles()`.
- **`TerminalRenderer`** (terminal.rs): `draw_glow` becomes `draw_brackets`. It tries `place_tiles` first, then falls back to a whole sprite under `SpriteKey::Brackets { width, height, border }`. It draws at `BRACKETS_Z` (the old `GLOW_Z` slot, between box and content).
- **Deleted:** `GlowShape`, `GlowStyle`, `glow_shape`, `glow_key`, `GLOW_OPACITY` and the glow tests.

### SVG renderer — `src/render/svg.rs`

- `Brackets` renders as one `<path>` in the foreground colour, filled, made of the four L shapes, with the same offset, arm and `border` thickness. The web app shows the same selection as the terminal. `--svg` export has no selection, so it doesn't change.
- **Deleted:** the blur filter (`glow_filter_defs`, `GLOW_FILTER_ID`, `GLOW_STROKE_WIDTH`, `GLOW_BLUR_STD_DEVIATION`), `glow_rect`, and the glow tests.

### Collaborators

`State` → `layout::tree::diagram` (emits `Brackets`) → `view::body` / `centre` (treats it as a decoration) → `TerminalRenderer` (`BracketsShape` via `TileShape`) and `SvgRenderer` (`<path>`), both reading `BRACKET_OFFSET` and `BRACKET_ARM`.
