# 080: A readable terminal renderer

Refactoring spec — no user story. One deliberate behaviour loss, named below.

## Goal

`TerminalRenderer::render` is hard to follow because a box is drawn in two places. `grid()` walks the placements and blanks the box's cells; `sprites()` walks them again, clips them and builds their pixels. To see how one box reaches the screen you read four functions, and the clipping arithmetic is repeated in each of them.

`SvgRenderer::draw` reads well: one pass per kind, each calling a small function named after what it draws. The terminal renderer should read the same way — one `match`, four `draw_*` functions, each doing its whole job — without sharing any code with SVG. SVG is untouched by this spec, and no shared "drawing target" trait is introduced.

## Technical Design

### Principle

One placement, one `draw_*` call, one place to read. Everything a kind needs — clipping, caching, pixels, cells — happens inside its own `draw_*`. Above them there is only the `match`. Below them there is `Screen`, which knows the output and nothing about diagrams, and `kitty`, which nothing above `draw_box`/`draw_arrow` mentions.

### `src/terminal.rs` — what the window is

A new top-level module owning window geometry, and the only place that asks the terminal about it.

- `pub(crate) struct Terminal { cols, rows, cell_width, cell_height }` — all `i64`.
- `pub(crate) fn probe() -> io::Result<Terminal>` — one `terminal_window_size` ioctl gives all four: `ws_col`, `ws_row`, and the cell size as `ws_xpixel / ws_col` and `ws_ypixel / ws_row`, rounded.
- `cell_size` and `terminal_size` move here from `writer.rs` and merge into `probe`. They are the same ioctl today, read twice.
- `kitty::supported` stays in `kitty.rs`: it asks whether the terminal speaks the graphics protocol, not how big it is.

The module is `crate::terminal`, distinct from the private `crate::render::terminal`. The overlap is tolerable because the latter is private to `render`; `window` is the alternative name if it reads badly in practice.

### No resize

`Terminal` is probed once, in `writer::run`, and handed to `TerminalRenderer::new`.

- `TerminalRenderer::resize` is removed, along with the `cols`/`rows` fields that `new` initialises to `0` — a state in which the renderer silently draws a zero-sized screen.
- `frame` loses its `cols`/`rows` parameters and reads them from the terminal it is given.

**Deliberate behaviour loss.** `writer.rs` re-probes the size inside the keystroke loop today, so a window resize is picked up on the next key. After this spec it is not: the diagram stays centred on the startup dimensions, and the save prompt writes to what was the bottom row. Restarting dre fixes both. Nothing is corrupted. Handling resize properly is a separate spec, to be written if it ever bites.

### `Screen` — the output for one render

Per-render scratch: built in `render`, drawn onto, consumed by `into_bytes`, dropped. It holds no diagram knowledge and survives nothing.

```rust
struct Screen {
    terminal: Terminal,
    origin: (i64, i64),
    characters: Vec<Vec<char>>,
    images: Vec<Sprite>,
}
```

- `new(terminal)` — a grid of `cols × rows` blanks.
- `centre_on(&[Placement])` — works out the bounding box and sets `origin`, where the diagram's `(0, 0)` lands on screen.
- `crop(&Placement) -> Option<Crop>` — the visible part, `None` when fully off screen.
- `write(x, y, character)` — bounds-checked, in diagram coordinates.
- `place(Sprite)` — an image at a cell.
- `into_bytes() -> Vec<u8>` — the home escape, the rows joined with `\r\n`, the Kitty clear, then each image in order.

`into_bytes` returning bytes costs one extra copy of a screenful and lets tests assert on a frame without a fake writer. `render` ends with a single `write_all`.

The character grid becomes plain `char`. Its colour and fill slots are always `None` today — `stamp` only ever preserves what is already there, and the test `a_coloured_box_puts_no_colour_in_the_grid` pins it. All colour comes from the images. So `cell()`, `RESET`, `BLANK_CELL` and the four `cell_with_*` tests are deleted. `put` and `stamp`, whose bounds checks are identical, collapse into `Screen::write`.

### `origin`

Screen position = placement position + `origin`. It is fixed for the whole render, decided by `centre_on` before anything is drawn — not a write head that advances. Every `draw_*` works in the layout's own coordinates, the same ones SVG uses, and `Screen` adds the origin inside `write` and `crop`. This deletes the `shifted` vector, a full clone of every placement whose only purpose was to carry the offset.

`centre_on` copies today's arithmetic exactly, including its asymmetry:

```rust
let span   = max(x + width) - min_x;   // horizontal: measured from min_x
let height = max(y + height);          // vertical:   measured from 0
origin = ((cols - span) / 2, (rows - height) / 2);
```

Note the shift is `placement.x + origin.0`; `min_x` is never subtracted. With a negative `min_x` the horizontal centring would be off by that amount. Today's layouts start at `x: 0`, so it never shows. Preserved as-is; correcting it is a behaviour change and belongs to its own spec.

### `Crop` — the visible part of a box or arrow

One value replacing the four loose `i64`s threaded through `sprites`, `sprite`, `sprite_key`, `outline_box` and `outline_arrow`, and the four-line cell→pixel conversion repeated in the last two.

```rust
struct Crop {
    col: i64, row: i64,              // where the image goes on screen
    first_x: i64, last_x: i64,       // which pixels of the full shape to keep
    first_y: i64, last_y: i64,
}
```

`Screen::crop` clamps the placement's screen span to `0..cols` and `0..rows`, returns `None` if nothing is left, and converts the dropped cells into pixel offsets using the terminal's cell size.

Cropping is not an optimisation; it is how clipping happens. `kitty::show` positions with `\x1b[{row+1};{col+1}H`, so a negative column is unrepresentable — an overhanging box would appear whole, shifted into view, instead of cut off. And `show` sends `a=T` without `C=1`, so the cursor lands below the image: one overhanging the bottom would scroll the screen and corrupt a home-and-redraw frame. The existing tests `a_box_overhanging_the_{left,top,right,bottom}_is_cropped` and `a_box_off_screen_has_no_sprite` keep passing unchanged.

### The four primitives

```rust
fn draw_box(&mut self, screen: &mut Screen, placement: &Placement)    // &mut self: the cache
fn draw_arrow(&mut self, screen: &mut Screen, placement: &Placement)  // &mut self: the cache
fn draw_label(screen: &mut Screen, placement: &Placement, label: &Label)
fn draw_cursor(screen: &mut Screen, placement: &Placement)
```

- `draw_box` and `draw_arrow` each: take the crop, return early on `None`, look up the cache, build the pixels on a miss, and hand `Screen` an image at the crop's `col`/`row`. `outline_box` and `outline_arrow` become their pixel-building halves, taking a `Crop`.
- `draw_box` no longer touches the character grid. Blanking the cells under a box is a leftover: the grid starts blank and the layout emits a box before its own label and cursor, so it never had a visible effect.
- `draw_label` and `draw_cursor` are free functions over `&mut Screen` — they need nothing from the renderer.
- `sprites()`, `sprite()` and `grid()` disappear. `Sprite` keeps its name: it is `kitty`'s own type and `kitty::show` takes it.

### The cache

Unchanged: same `SpriteKey` variants, same clear-everything-at-`CACHE_LIMIT`. It is the one thing that outlives a render, so it stays on `TerminalRenderer`, which after this spec holds exactly two things: a `Terminal` and a cache. `SpriteKey` carries a `Crop` where it carried four numbers.

### The top level

```rust
fn render(&mut self, doc: &Document, out: &mut impl Write) -> io::Result<()> {
    let placements = with_cursor(layout(&doc.boxes), doc.selected.clone());
    let mut screen = Screen::new(self.terminal);
    screen.centre_on(&placements);
    for placement in &placements {
        match &placement.node {
            PlacementNode::Node(_)      => self.draw_box(&mut screen, placement),
            PlacementNode::Arrow(_)     => self.draw_arrow(&mut screen, placement),
            PlacementNode::Label(label) => draw_label(&mut screen, placement, label),
            PlacementNode::Cursor(_)    => draw_cursor(&mut screen, placement),
        }
    }
    out.write_all(&screen.into_bytes())
}
```

The same shape as `SvgRenderer::draw`, arrived at independently. `draw()` returning `Vec<String>` is gone, and with it the trick of appending the graphics payload to the last line — `into_bytes` emits the rows then the images naturally, byte for byte the same.

### Tests

Bytes out are unchanged, so the frame-level tests stand: `line_count_is_unchanged`, `the_graphics_payload_is_appended_to_the_last_line_only`, `a_frame_with_sprites_ends_with_a_clear_then_each_sprite_shown_in_order`, the centring tests, the clipping tests, the cache tests.

- Deleted: the four `cell_with_*` tests, with `cell()` itself.
- Rewritten to the new surface: tests calling `renderer(1, 1)` now build a `Terminal`; tests calling `draw()` go through `render` or through `Screen`.
- `writer.rs` tests construct a `Terminal` instead of passing `cols`/`rows` to `frame`.
- New: `Terminal::probe` arithmetic, `centre_on`'s origin, `Crop`'s clamping and pixel offsets.

### Dependencies after this spec

```
terminal     ← (nothing)
render       → diagram, layout, kitty, terminal
writer       → state, diagram, render, file_document, terminal
```

`writer.rs` stops knowing about `winsize`. Inside `render`, `terminal.rs` gains `Screen` and `Crop` as private types; `svg.rs` is not touched.
