# Brackets are not tiled

## Refactoring Goal

Selection brackets go through the tiler (`draw_brackets` → `place_tiles` with `TileStyle::Brackets`) as if they were box-sized sprites. In fact, `BracketsShape` only paints four fixed L's (arms of `BRACKET_ARM` = 14px, starting `BRACKET_OFFSET` = 8px outside each box corner), each about 2×2 cells. The rest of the placement is always empty. The rendered screen should stay the same, but brackets should stop being tiled.

Split out of spec 187.

## Technical Design

Decisions from the design session:

### Unit of drawing: four corner sprites
- `draw_brackets` no longer calls `place_tiles`. It places one sprite per corner (TL, TR, BL, BR).
- Each sprite is a `corner_cells × corner_cells` block of cells (about 2×2: `BRACKET_MARGIN` is 1 cell and the arm reaches `BRACKET_ARM - BRACKET_OFFSET` = 6px past it). It is placed at its corner of the placement.
- The rendered pixels must stay identical to today's.

### Pixels: a single-corner shape
- New `BracketCornerShape { corner, margin_x, margin_y, offset, arm, thickness, colour }` paints one L, mirrored by `corner`. It has no width or height of its own.
- It replaces `BracketsShape` (whole placement). The SVG renderer has its own bracket code and is untouched.

### No fallback
- Placements too small for the corner blocks to stay apart just overlap. Bracket ink is one opaque colour, so overlapping sprites give the same pixels as today's union.
- Deleted: the whole-placement fallback path in `draw_brackets`, `brackets_key`, `brackets_canvas`, the bracket `SpriteKey` variant, `BracketsShape`.

### Transmission and caching: persistent ids
- A corner sprite depends only on `BracketKey { corner, border, cell }`, at most four images per style, independent of placement size.
- The renderer gets `bracket_images: HashMap<BracketKey, ImageId>`. The canvas is built only on first use, so there is no canvas map.
- A shared renderer helper does "id known → `place_cached`, else allocate, build, `place_fresh`". `place_tiles` and `draw_brackets` both use it.

### Layout
- New module `src/render/brackets.rs`: `Corner`, `BracketKey` (with `canvas()`), `BracketCornerShape`, `corner_cells(cell)`, and each corner's cell offset within a placement.
- `TileStyle::Brackets` and `brackets_shape` are removed from `tiles.rs` and `terminal.rs`. `TileStyle` then has one variant, so it collapses into `BoxStyle`.

### Collaborators
- `Renderer::draw_brackets` → `brackets.rs` (keys, canvases, corner offsets) and the shared sprite-placement helper → `Frame::place_fresh` / `place_cached`.
- `frame.shows(cell, area)` still clips each corner block.

### Tests
- Per corner, `BracketCornerShape` pixels equal the current whole-placement `BracketsShape` pixels in the same cell block. Check this for `CELL` and `ODD_CELL` before deleting `BracketsShape`.
- Small placements: the union of the corner sprites equals the old fallback canvas.
- Moving or resizing a bracket placement uploads no new images after the first frame.
