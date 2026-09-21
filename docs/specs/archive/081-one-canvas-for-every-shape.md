# 081: One canvas for every shape

Refactoring spec — no user story. Behaviour doesn't change: the bytes out are identical.

## Goal

`Canvas` is used by arrows and nothing else. Boxes reach pixels two other ways: `square_pixels`/`body_row` build a row and repeat it, and `RoundedBox` samples every pixel. Three unrelated mechanisms produce the same thing — one sprite's worth of RGBA bytes — and each one carries its own copy of the crop arithmetic, because the four window bounds are threaded into all of them.

Two of those three differences aren't real. A square box is a rounded box with a radius of zero; `coverage()` already yields exact hard edges at radius 0, so `RoundedBox` and `square_pixels` are one shape computed two ways. And an arrow doesn't need to be *stamped* — "is this pixel part of a stroke" is a question it can answer as readily as a box answers it for a border.

What is left once those collapse is a single idea: **a shape answers what colour a pixel is; a canvas asks it for every pixel it has.** Clipping is not a shape's business, and position is not a canvas's business.

## Technical Design

### Principle

A shape is asked one question — what colour is this pixel, if any — and how it decides is private to it. A canvas is a box measured in pixels: it holds the grid, fills itself by asking a shape, and knows no colours and no screen position of its own. `Screen` alone knows the terminal and the origin, so `Screen` alone crops and places.

`render/svg.rs` is not touched, and no drawing concept is shared between the two renderers.

### The pipeline

Today, cropping happens before any pixel exists and is threaded into the drawing:

```
Placement [cells] ──► Screen::crop ──► Crop ──► outline_box / outline_arrow
                                                  (draws only surviving pixels)
                                                  ──► Sprite ──► Screen::place
```

After this spec, a shape is rasterised at full size as if the screen were infinite, and the crop is the last thing that happens to it:

```
Placement [cells] ──► Screen::shows? ──► cells × cell size ──► Canvas::fill(shape)
                            │ no                                    [full size]
                            ▼                                            │
                         skipped                                         ▼  cached by
                                                                         │  shape alone
                                            Screen::place(canvas, placement)
                                                  │  crops and positions
                                                  ▼
                                            kitty::show(canvas, col, row)
```

### `src/canvas.rs` — a box measured in pixels

A new top-level module, a leaf: it imports nothing from the crate.

```rust
pub(crate) type Rgba = [u8; 4];

pub(crate) struct Canvas {
    pub(crate) pixels: Vec<u8>,
    pub(crate) width: i64,
    pub(crate) height: i64,
}
```

- `fill(width, height, shape: &impl Shape) -> Canvas` — allocates a zeroed buffer and walks it, writing what the shape returns. `None` leaves the pixel as it is, which is transparent.
- `crop(&self, first_x, last_x, first_y, last_y) -> Canvas` — a smaller canvas, row slices copied out.

```rust
pub(crate) trait Shape {
    fn colour_at(&self, x: i64, y: i64) -> Option<Rgba>;
}
```

Gone from `Canvas`: `ink`, and with it `point`, `horizontal`, `vertical`, `offset`, `span`, `first_x`/`last_x`/`first_y`/`last_y`, and `pixels()` cloning the buffer. `ink` was never a concept — a canvas has no more colour than paper does — only an argument hoisted into the constructor because an arrow happens to be one colour throughout. Colour now arrives per pixel, from the shape.

A canvas never blends. Mixing a border into a fill along a curve is arithmetic inside the box shape, which hands over one finished `Rgba`. The canvas stores it.

### `src/render/shapes.rs` — the two shapes

Private to `render`. Both implement `Shape`, both are constructed in pixel units, neither knows the screen exists.

```rust
struct BoxShape  { width, height, border, radius, edge: Rgba, fill: Rgba }
struct ArrowShape { width, stop_rows: Vec<i64>, shaft_row, trunk: (i64, i64), stroke, ink: Rgba }
```

**`BoxShape::colour_at`** is today's `RoundedBox::pixel` unchanged: outer coverage minus inner coverage gives the edge's share, the rest is fill, and the two are mixed into one `Rgba` — or `None` when the alpha works out to zero. `RoundedBox`, `square_pixels` and `body_row` are all deleted, along with `outer` and `corner_row`, which exist only to skip sampling on straight rows.

A square box is this shape with `radius: 0`, and it comes out byte for byte the same. At radius 0 every sampled pixel centre sits half a unit inside or outside the rectangle, so `coverage()` returns exactly 1 or exactly 0 and never a fraction: pure edge, or pure fill, with a hard boundary at `border`. The degenerate cases agree too — a box narrower than two borders gives a negative inner width, so the inner coverage is 0 everywhere and the result is solid edge, which is what `square_pixels`' `width <= 2 * border` branch does by hand. The existing pixel tests are the check.

**`ArrowShape::colour_at`** answers the question the stamping loop answered, per pixel, in the same terms — `centered_span` survives and is used to test membership rather than to drive writes:

- shaft: `x` in `0..=midpoint` and `y` in `centered_span(shaft_row, stroke)`
- trunk: `x` in `centered_span(midpoint, stroke)` and `y` in `trunk_top..=trunk_bottom`
- each stop: `x` in `midpoint..=width - 1` and `y` in `centered_span(stop_row, stroke)`
- each arrowhead: for `distance` in `0..depth`, the 4×4 block centred on `(right_edge - distance, stop_row ± python_round(distance * slope))`

A pixel is ink if any of those holds, otherwise `None`. The arrowhead's ~13 iterations run per pixel instead of per stamp; the sprite is cached by shape, so that cost is paid once per distinct arrow.

This is a deliberate transcription, not an improvement. The arrowhead's stair pattern is preserved exactly, and every existing arrow test passes untouched. Smoothing those diagonals is a behaviour change and belongs to its own spec.

### Rasterising at full size

`outline_box` and `outline_arrow` lose their `Crop` parameter and become the two halves of building a full-size canvas: convert the placement's cells to pixels, construct the shape, call `Canvas::fill`. `cells_to_pixels_x`/`cells_to_pixels_y` are called there and nowhere else.

Every clipping conditional inside the drawing code disappears: the four-way bounds test in `point`, the clamped spans in `horizontal` and `vertical`, the `left_edge`/`right_edge` arithmetic in `body_row`, the top and bottom equivalents in `square_pixels`, and the window bounds threaded through `RoundedBox::pixels` and `corner_row`.

"As if the screen were infinite" governs **how** a shape draws itself — always whole, never consulting the terminal — not **which** shapes are drawn. Whether a shape is worth drawing at all is a separate question, and it stays on `Screen`, so the shapes remain ignorant of the terminal either way.

That question has to be asked *before* rasterising, which makes it distinct from cropping rather than a by-product of it:

```rust
fn shows(&self, placement: &Placement) -> bool
```

The same clamping `crop` does, in cells, without the pixel conversion. `draw_box` and `draw_arrow` ask it first and return early, before the cache lookup and before any canvas exists.

Without it the skip would be worthless, since a canvas discarded by `place` has already been built. And it can't simply be dropped in favour of drawing everything: the cache would then hold every shape in the document after the first frame, so a diagram with more than `CACHE_LIMIT` shapes would overflow, clear, and thrash permanently no matter how few were visible.

With it, the cost of full-size rasterising is bounded to the few shapes straddling an edge.

### `Screen` crops and places

`Screen` already clips characters itself, bounds-checking each one inside `write`. It now clips images the same way, so there is one clipping policy in one type instead of two.

- `shows(&self, placement: &Placement) -> bool` — worth drawing at all, asked before anything is rasterised.
- `place(&mut self, canvas: &Canvas, placement: &Placement)` — crops with its own `crop(placement)`, returns without placing on `None`, and records the cut canvas at the crop's `col`/`row`.
- `crop` keeps its six fields and becomes private to this path; `at_origin` is deleted with the cache key that needed it.

`shows` and `crop` answer different questions — whether to bother, and what survives — which today's single `Option<Crop>` conflates. `place` keeps its `None` branch regardless of `shows`, so clipping stays correct on its own terms rather than depending on the optimisation having run.
- `images` holds `Placed { canvas: Canvas, col: i64, row: i64 }`, private to `Screen`.

`draw_box` and `draw_arrow` shrink to: look up the cache, build the full-size canvas on a miss, hand it to `Screen`. Neither learns where the shape landed, and `TerminalRenderer` is out of the screen-geometry business entirely.

### `Sprite` is deleted

`Sprite`'s `col`/`row` exist only so `kitty::show` can emit a cursor-position escape before the payload, and `Screen` is what decides them.

```rust
pub(crate) fn show(canvas: &Canvas, col: i64, row: i64) -> Command
```

`kitty` imports `canvas` rather than owning the type, which keeps the drawing vocabulary out of the escape-code layer. `transmission` is unchanged and still takes pixels, width and height.

### The cache

`SpriteKey` loses its `Crop` field in both variants and becomes shape identity alone — size, colour, fill, radius, stops, shaft. Entries are now full-size canvases.

This is strictly better: a box half off-screen and the same box fully visible are the same entry, so panning and resizing reuse pixels instead of thrashing. Cropping on the way out costs nothing new, because `cached()` already clones the whole buffer on every frame.

The bound stays as it is — same `CACHE_LIMIT`, same clear-everything on overflow.

### Tests

The bytes out don't change, so the frame-level, centring, clipping and box-pixel tests all stand.

- **Deleted**: the `Canvas` stamping tests (`point_*`, `horizontal_*`, `vertical_*`), which test a vocabulary that no longer exists. What they pin — stroke thickness and extent — is already covered by `the_shaft_is_arrow_stroke_pixels_thick`, `the_trunk_is_arrow_stroke_pixels_thick` and the arrowhead tests, which now exercise `ArrowShape::colour_at`.
- **Rewritten**: `a_differently_cropped_box_is_redrawn` asserts the opposite of what it says now — the two placements share one cache entry — and is renamed to match. `clipping_via_outline_box_is_a_pure_crop_of_the_whole_box` moves to `Canvas::crop`, where it stops being a property to verify and becomes the definition.
- **New**: `Canvas::fill` walks every pixel and stores what the shape returns; `Canvas::crop` cuts the right rows and columns; `BoxShape` with `radius: 0` matches the square pixels it replaces; `Screen::shows` is false off screen and true for a shape straddling an edge, and an off-screen placement leaves the cache empty.
- `box_outline` and `arrow_outline` return a `Canvas` instead of a `Sprite`, and `pixel_of` reads from one.

### Dependencies after this spec

```
canvas       ← (nothing)
terminal     ← (nothing)
kitty        → canvas
render       → diagram, layout, kitty, terminal, canvas
```

Inside `render`: `shapes → canvas`, `terminal → shapes`, `svg` untouched and unaware of any of it.
