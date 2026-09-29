# Boxes and glows are drawn from cached cell tiles

This is a performance spec. It changes how box and glow sprites are produced and transmitted, not what the user sees.

It builds on spec 169 (Kitty caches font sprites): `kitty::ImageId`, the `Image::Fresh` / `Image::Cached` frame representation, and per-frame placement IDs on `kitty::place`.

## Problem

Typing into a box label makes each keystroke slower as the label grows (about 20 ms to 80 ms+ in a debug build, for a single borderless box).

The label widens the box, and the box's `SpriteKey` includes its width, so every keystroke misses the sprite cache for both the box and its selection glow. Both canvases are rasterized again pixel by pixel. `BoxShape` evaluates two rounded-rectangle distance fields per pixel and `GlowShape` calls `powf` per pixel. Both canvases are then zlib-compressed, base64-encoded and transmitted in full on every frame, including frames that hit the local cache. Their size, and so the cost of each keystroke, grows linearly with the label's width.

Most of that work is redundant. Away from its corners, a rounded box or glow looks the same in every column, and the same in every row. Only the pixels near an edge depend on how far they are from that edge.

## Acceptance Criteria

- The rendered screen is visually unchanged: every box and glow is pixel-identical to its current whole-sprite rendering.
- Box and glow sprites are drawn as one-cell tiles. Tiles that are pixel-identical by construction share a single cached image.
- Changing a box's width or height by typing reuses the tiles already cached for its style, and transmits no box or glow pixels for them.
- Once a style's tiles are cached, the rendering and transmission cost of a box or glow no longer depends on its width or height.
- A box or glow too small to have a middle band in some axis is still drawn from a single whole sprite, exactly as today.
- Arrows, LEDs, carets and the cursor are unchanged.

## Technical Design

### Bands

Along each axis a tiled sprite is split into three bands of whole cells:

- **Start:** the first `band` cells from the left (or top) edge.
- **Middle:** every cell between the two edge bands.
- **End:** the last `band` cells before the right (or bottom) edge.

`band` is the smallest whole number of cells that covers every pixel whose colour depends on its distance from that edge, plus one antialiasing pixel. For an axis with cell size `cell` pixels:

- Box: `extent = BoxShape::outer()` when it has a corner radius, otherwise the border inset on that side. `band = ceil((extent + 1) / cell)`.
- Glow: `extent = margin + radius`, where `radius` is the clamped radius `GlowShape` already computes. `band = ceil((extent + 1) / cell)`.

A sprite is tiled only when both axes have room for a middle band, meaning `cells >= 2 * band + 1` along each axis. Otherwise it goes through the existing whole-sprite path.

### Tile identity

```rust
enum Band { Start(i64), Middle, End(i64) }

struct TileKey {
    shape: TileShape,
    column: Band,
    row: Band,
}
```

- `Start(n)` / `End(n)` hold the cell's offset from that edge (`0..band`). Every middle cell is `Middle`.
- `TileShape` is the existing box or glow style with the width and height removed: sides, border, radius, edge and fill colours for a box; colour and roundedness for a glow. The cell size in pixels is part of the key, so a resize never reuses a stale tile.
- Pixel-identical tiles share a key by construction. Tiles that differ in any style field, band or offset never share one.

### Rasterizing a tile

A tile is rasterized by evaluating the existing `Shape::colour_at` over the tile's one cell of pixels in a **reference sprite**. The reference sprite has the same style and is `2 * band + 1` cells long along each axis; its middle cell stands for every `Middle` cell. `BoxShape` and `GlowShape` are unchanged. Only the region being evaluated changes, from the whole canvas to one cell.

`TerminalRenderer` keeps rasterized tiles in a `TileKey -> Canvas` map. The map has no eviction: its size is bounded by the number of distinct styles times a few band offsets, and does not depend on width.

### Terminal-side caching and placement

Tiles use the glyph mechanism from spec 169:

- A `TileKey -> kitty::ImageId` map lives as long as `TerminalRenderer`.
- The first visible occurrence of a tile is queued as `Image::Fresh`. Every later occurrence, in the same frame or a later one, is queued as `Image::Cached` and emitted as `kitty::place` with its own per-frame placement ID.
- Tiles keep the z-index of the sprite they come from (`BOX_Z`, `GLOW_Z`), and are queued in row-major order at the sprite's position in scene order.

Tiles are one cell and clipping areas are cell-aligned, so each tile is either fully visible or skipped with the existing visibility check. A tile is never cropped.

Whole sprites that fall back because they are too small stay transient, as specified in 169: freshly transmitted each frame and hard-deleted by the next.

### Out of scope

- The second, untimed render per keystroke in the controller loop.
- Tiling arrows, LEDs or other sprites.
- Kitty placement scaling (`c=` / `r=`) to stretch a middle tile. One-cell tiles placed repeatedly avoid depending on the terminal's scaling quality.
- Eviction of the tile maps, and terminal-side eviction detection.

### Tests

`render/terminal.rs` (or a tile module beside it):

- **Pixel equivalence** is the main guard. For a range of widths and heights at and above the tiling threshold, across box styles (all sides with radius, partial sides without radius, borderless with fill, each palette colour) and glow styles (rounded and square, coloured and uncoloured), and at least two cell sizes, composing the tiles gives exactly the whole-sprite canvas.
- Bands: a sprite exactly `2 * band + 1` cells long along an axis is tiled; one cell shorter falls back to the whole sprite.
- Rendering a box at width `W` and then `W + 1` with the same style: the second frame contains no `a=T` for any box or glow tile, and one more `a=p` per tiled row than the first.
- Two boxes of different widths and the same style share every tile image ID. Boxes whose styles differ share none.
- A partly clipped box shows only the tiles inside the area, none of them cropped.
- Resizing the cell size produces new tile keys and fresh transmissions.
- Existing placement order, clipping, z-index, resize and visual pixel tests continue to pass.
