# Flex: the box wears its own colours

## User Story

Sami runs `dre-flex` and sees an empty box with a dark #2A2A2E border. They type "Hello", and the text shows in a soft #C9C9CF grey while the box grows around it, the border still dark. They press Backspace, the box shrinks, and the colours don't change. They press Enter to switch to MOVE, and everything keeps the same colours. They press Ctrl-C to leave.

## Acceptance Criteria

- The box border is drawn in #2A2A2E.
- The text inside the box is drawn in #C9C9CF.
- The colours stay the same in both WRITE and MOVE mode.
- The border stays #2A2A2E while the box grows and shrinks as you type and backspace.

## Technical Design

Decisions:

- **The colours are local to `flex::view`.** Two hex constants live next to `FLEX_BORDER` from 186. `style::PALETTE` stays as it is, so the main editor's `next_on_palette` cycle can't pick up the flex greys.
- **Placements carry final RGB, not palette indices.** `PlacementNode::Box.colour` and `Label.colour` change from `Option<u8>` to a required `view::Rgb` (`(u8, u8, u8)`), just as `border` already carries final pixels. Whoever builds a placement decides exactly how it looks, and the renderer stops reading the palette for boxes and labels.
- **No `None`, no hidden default.** "Plain means foreground" moves out of the renderer and into the main editor's placement builders (`layout::tree`, and `footer` and the other label builders in `view`). The main editor's palette colours are turned into RGB there with a new `style::rgb(Option<u8>) -> Rgb`, which does what `render::colour` does today (`None` → `FOREGROUND`). `render::colour` delegates to it and is still used for brackets, arrows, cursors and LEDs.
- **Out of scope:** `fill` (`Option<u8>` + opacity) and `Led.colour` (`u8`) stay palette indices. Neither is needed here.
- **The colours don't depend on the mode.** `scene` never reads `state.mode`, and the colours are constants. 185 doesn't depend on 182: when 182 adds `FlexMode::Move`, it also adds the test that a MOVE state gets the same colours.

### `view`: what changes

```rust
pub type Rgb = (u8, u8, u8);

pub struct Label<'a> {
    pub text: Cow<'a, str>,
    pub colour: Rgb,
    pub bold: bool,
}

pub enum PlacementNode<'a> {
    Box {
        colour: Rgb,
        fill: Option<u8>,
        opacity: Option<f64>,
        rounded: bool,
        sides: Sides,
        border: i64,
    },
    // ...unchanged
}
```

- Each `colour: None` becomes `colour: style::rgb(None)`. The footer's dim filename label becomes `style::rgb(Some(style::DIM))`.

### `style`: what changes

```rust
pub(crate) fn rgb(colour: Option<u8>) -> Rgb
```

- `rgb(None)` is the `FOREGROUND` entry, and `rgb(Some(i))` is `palette(i)`.

### `layout::tree`: what changes

- The diagram box uses `colour: style::rgb(node.colour())`, and the diagram label uses `style::rgb(None)`. `fill` still comes from `node.colour()` as an index.

### `render`: what changes

- `terminal`: `BoxStyle.colour`, `LabelStyle.colour` and `GlyphKey.colour` become `Rgb`. `box_shape` uses `style.colour` directly as the edge, and glyphs are tinted with the RGB they carry.
- `svg`: `rect` takes the edge as `Rgb`, and `label_text` uses `label.colour` directly.
- `render::colour(Option<u8>)` becomes a thin wrapper over `style::rgb` for the shapes that still use palette indices.

### `flex::view`: what changes

```rust
const FLEX_BORDER_COLOUR: Rgb = (0x2A, 0x2A, 0x2E);
const FLEX_TEXT_COLOUR: Rgb = (0xC9, 0xC9, 0xCF);
```

- The box placement uses `colour: FLEX_BORDER_COLOUR`, and the label uses `colour: FLEX_TEXT_COLOUR`. Everything else in `scene`, and all of `FlexState`, `reduce` and `run_loop`, stay the same.

### Tests

`scene` unit tests in `flex::view`:

- For an empty box (`""`), the box `colour` is `(0x2A, 0x2A, 0x2E)` and the label `colour` is `(0xC9, 0xC9, 0xCF)`.
- The box for `"Hello"` and the box after Backspace from `"Hellp"` both still have `colour == (0x2A, 0x2A, 0x2E)`, and their labels are `(0xC9, 0xC9, 0xCF)`.

`style` unit tests:

- `rgb(None)` equals `palette(FOREGROUND)`, and `rgb(Some(DIM))` equals `palette(DIM)`.

Main editor guard (existing tests, updated to RGB, not new ones):

- The `layout::tree` tests that pin a box's colour (`layout_carries_a_boxs_colour_and_rounding` and the others) assert `style::rgb(Some(i))` instead of `Some(i)`, and plain boxes and labels assert `style::rgb(None)`.
- The SVG tests `colourless_boxes_are_foreground_while_coloured_boxes_keep_palette_colours` and `a_coloured_filled_rounded_box_renders_stroke_fill_and_rx` keep checking the same stroke colours. They build placements with RGB, but the output stays the same, which shows the main editor looks exactly as before.
- The `render::colour` tests stay as they are.
