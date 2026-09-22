# Bob sees Iosevka labels

## User Story

Bob opens dre, draws a box with a label, and sees the label rendered in Iosevka Regular in the terminal, so that his diagram immediately looks like a dre diagram.

## Acceptance Criteria

- The Iosevka Regular font file is bundled with dre (no separate install required by the user).
- Box labels in the terminal render using Iosevka Regular.
- Arrow labels in the terminal render using Iosevka Regular.

## Technical Design

### Why this isn't just a font-config change

Box and arrow strokes are already rasterized procedurally into pixel `Canvas`es (`render/shapes.rs`'s `BoxShape`/`ArrowShape`, via the `Shape` trait) and sent to the terminal as images through the Kitty graphics protocol (`kitty.rs`), bypassing the terminal's own text rendering entirely. Labels are the odd one out: `draw_label` (`render/terminal.rs`) currently writes plain `char`s into `Screen.characters`, which get printed as literal text and rendered in whatever font the user's terminal happens to be configured with — dre has no control over it, and no bundled-font guarantee is possible that way.

The fix is architectural: labels must move onto the same pixel-canvas + Kitty-image pipeline as boxes and arrows, rasterizing text with a bundled font instead of relying on terminal text.

### Font asset and rasterization

- The font file lives at `assets/IosevkaRegular.ttf` and is embedded into the binary at compile time via `include_bytes!`, so there is no runtime file lookup and no separate install step. This is the standard-width Iosevka Regular build (not the narrower "Term" variant): it's already present locally at `~/Library/Fonts/Iosevka-Regular.ttf`, and using it avoids sourcing a separate Iosevka Term release just for this story.
- `fontdue` is used to parse the font and rasterize glyphs. It's a small, pure-Rust, dependency-free crate (no system font or C library dependency), consistent with the project's existing lean dependency list.

### New module: `render/font.rs`

- `GlyphShape { width, height, coverage: Vec<u8>, ink: Rgba }` implements the existing `Shape` trait: `colour_at(x, y)` indexes directly into the precomputed `coverage` buffer (from fontdue's rasterized output) and scales `ink`'s alpha by the coverage value — no per-pixel formula, just a lookup. This lets glyphs reuse the exact same `Canvas::fill` construction path as `BoxShape`/`ArrowShape`.
- `ink` is a fixed grey, reusing `PLAIN_COLOUR` (`render/mod.rs`) — the same colour already used for uncoloured box borders. This mirrors the SVG renderer, where label text always uses a fixed ink colour (`var(--ink)`) regardless of the box's own colour, rather than inheriting the box's palette colour. Unlike the SVG's CSS variable, a Kitty-protocol image has baked-in RGBA pixels with no theme adaptation, so a concrete constant is required.
- `GlyphCache` owns the parsed `fontdue::Font`, a `HashMap<char, Canvas>` cache, and the terminal's cell metrics (`cell_width`, `cell_height`, fixed for the renderer's lifetime). Its `glyph(&mut self, ch: char) -> &Canvas` method rasterizes and caches on first use, keyed per character (not per label string) — labels are free-form text, so whole-string caching could grow unbounded, while a per-glyph cache is naturally bounded to Iosevka's character set and gets reused across every label.
- Glyphs are sized so their advance width exactly equals `terminal.cell_width` (the real per-terminal pixel size, queried from the terminal window — not the fixed `CELL_WIDTH`/`CELL_HEIGHT` constants used by the SVG renderer's virtual grid). This isn't a workaround: Iosevka Regular is monospace by design, so every glyph shares one advance width, and `layout.rs`'s column-counted label/box sizing already assumes "1 character = 1 column" of pixels. No layout changes are needed or in scope.
- Terminal cell aspect ratio is dictated by the user's terminal emulator and font (`Terminal.cell_width`/`cell_height`, queried via `TIOCGWINSZ`), which dre cannot override — box and arrow pixel dimensions already depend on it today regardless of label font. Common monospace terminal fonts cluster near Iosevka's own aspect ratio, so this is accepted as a known limitation, not solved by this story.
- fontdue's `rasterize` returns only a glyph's tight bounding box (e.g. `x` is short, `g` dips below the baseline), not a full-cell rectangle. `GlyphCache` computes one shared baseline per cache from the font's ascent metric, and places each glyph's tight bitmap into a full `cell_width × cell_height` canvas relative to that shared baseline (using the glyph's own `xmin`/`ymin` from fontdue's `Metrics`), so all glyphs align consistently regardless of their individual bounding boxes.

### Integration

- `TerminalRenderer` gains a `glyph_cache: GlyphCache` field, constructed in `TerminalRenderer::new`.
- `draw_label` stops writing to `screen.characters` and instead, for each character, looks up its `Canvas` via `glyph_cache.glyph(ch)` and calls `screen.place(...)` — the same mechanism `draw_box`/`draw_arrow` already use to queue Kitty image placements.
- Both box and arrow labels already flow through the same `PlacementNode::Label` variant and the same `draw_label` function, so this one change satisfies both acceptance criteria without separate code paths.

### Testing approach

- `GlyphShape::colour_at` is unit-tested directly with a small hand-built fake `coverage` buffer — no real font involved, mirroring how `BoxShape`/`ArrowShape` are tested with synthetic geometry.
- `GlyphCache` is tested with the real bundled Iosevka bytes, but only for structural properties: rasterizing the same character twice returns the identical cached `Canvas` without re-rasterizing, and a produced canvas is exactly `cell_width × cell_height`. Pixel-exact glyph shape matching against the real font is deliberately out of scope — asserting the shape of real hinted/antialiased glyph output by hand would be brittle for no real benefit.
