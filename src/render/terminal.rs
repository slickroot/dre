use std::io::{self, Write};

use super::brackets::{corner_cells, corner_offset, BracketKey, CORNERS};
use super::font::GlyphSource;
use super::shapes::{ArrowShape, BoxShape, LedShape};
use super::tiles::{CellSize, TileKey, TileShape};
use super::virtual_terminal::{
    CaretKey, Content, Desired, GrowKey, ImageKey, SourceRect, Sprites, TypingCaretKey,
};
use super::{colour, Renderer, ARROW_OPACITY, OPAQUE, ROUNDED_RADIUS};
use crate::canvas::Canvas;
use crate::composer::Area;
#[cfg(test)]
use crate::style::palette;
use crate::tty::Window;
use crate::view::Scene;
use crate::view::{Geometry, Placement, PlacementNode, Rgb, Sides};

const BEGIN_SYNCHRONIZED_UPDATE: &str = "\x1b[?2026h";
const END_SYNCHRONIZED_UPDATE: &str = "\x1b[?2026l";

pub(super) const ARROW_STROKE: i64 = 3;
const GAP_PX: i64 = 2;
const ARROWHEAD_EDGE_LENGTH: f64 = 15.0;

pub(crate) const CACHE_LIMIT: usize = 512;

const TRANSPARENT: (u8, u8, u8, u8) = (0, 0, 0, 0);

const BRACKETS_Z: i32 = u8::MAX as i32 + 1;
const CONTENT_Z: i32 = BRACKETS_Z + 1;
const INK_Z: i32 = CONTENT_Z;

fn depth_z(depth: u8) -> i32 {
    i32::from(depth)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct BoxStyle {
    pub(super) colour: Rgb,
    pub(super) fill: Option<u8>,
    pub(super) fill_alpha: Option<u8>,
    pub(super) solid_fill: Option<Rgb>,
    pub(super) rounded: bool,
    pub(super) sides: Sides,
    pub(super) border: i64,
    pub(super) gap: bool,
}

#[derive(Clone, Copy)]
struct LedStyle {
    colour: u8,
    lit: bool,
}

struct LabelStyle {
    text: String,
    colour: Rgb,
    bold: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct GlyphKey {
    pub(super) character: char,
    pub(super) colour: Rgb,
    pub(super) bold: bool,
}

#[derive(Clone)]
struct ArrowStyle {
    stops: Vec<i64>,
    shaft: i64,
}

pub(super) fn centered_span(c: i64, width: i64) -> std::ops::Range<i64> {
    let start = c - (width - 1).div_euclid(2);
    start..(start + width)
}

pub(super) fn quantized_alpha(opacity: Option<f64>) -> Option<u8> {
    opacity.map(|value| (value * OPAQUE as f64).round() as u8)
}

fn fill_colour(fill: Option<u8>, alpha: Option<u8>) -> (u8, u8, u8, u8) {
    match (fill, alpha) {
        (Some(colour), Some(alpha)) => {
            let (r, g, b) = crate::style::palette(colour).unwrap();
            let composite =
                |channel: u8| (channel as f64 * alpha as f64 / OPAQUE as f64).round() as u8;
            (composite(r), composite(g), composite(b), OPAQUE)
        }
        _ => TRANSPARENT,
    }
}

pub(super) fn box_shape(width: i64, height: i64, style: BoxStyle) -> BoxShape {
    let (r, g, b) = style.colour;
    let (fill_r, fill_g, fill_b, fill_a) = match style.solid_fill {
        Some((r, g, b)) => (r, g, b, OPAQUE),
        None => fill_colour(style.fill, style.fill_alpha),
    };
    BoxShape {
        width,
        height,
        border: style.border,
        radius: if style.rounded { ROUNDED_RADIUS } else { 0 },
        sides: style.sides,
        edge: [r, g, b, OPAQUE],
        fill: [fill_r, fill_g, fill_b, fill_a],
    }
}

const GROW_FRAMES: usize = 9;

fn eased(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(3)
}

fn grow_frames(width: i64, height: i64, style: BoxStyle) -> Vec<Canvas> {
    (1..=GROW_FRAMES)
        .map(|k| {
            let e = eased(k as f64 / GROW_FRAMES as f64);
            let box_width = ((width as f64 * e).round() as i64).clamp(1, width);
            let box_height = ((height as f64 * e).round() as i64).clamp(1, height);
            Canvas::fill(width, height, &box_shape(box_width, box_height, style))
        })
        .collect()
}

// WezTerm keys an animation's playback state by the hash of its root frame,
// computed once at `a=T` (ImageData::with_data, wezterm-cell/src/image.rs) and
// cached by the GUI (cached_image, wezterm-gui/src/glyphcache.rs). Two grows
// with the same root would share one state and the second would show at full
// size, so the stamp makes every root unique: its 48 low bits go into the
// lowest bit of the R, G and B bytes of the first 16 pixels. The root is the
// full box because some terminals (Ghostty before August 2026) don't play the
// frames and show only the root. WezTerm and Ghostty create the root gapless
// and skip it during the grow.
fn grow_root(full: &Canvas, stamp: u64) -> Canvas {
    let mut root = Canvas {
        pixels: full.pixels.clone(),
        width: full.width,
        height: full.height,
    };
    let colour_bytes = root
        .pixels
        .iter_mut()
        .enumerate()
        .filter(|(index, _)| index % 4 != 3);
    for (bit, (_, byte)) in colour_bytes.take(48).enumerate() {
        *byte = (*byte & !1) | ((stamp >> bit) & 1) as u8;
    }
    root
}

fn whole(window: Window) -> Area {
    Area {
        col: 0,
        row: 0,
        cols: window.cols,
        rows: window.rows,
    }
}

pub(super) fn python_round(value: f64) -> f64 {
    let floor = value.floor();
    let diff = value - floor;
    if diff < 0.5 {
        floor
    } else if diff > 0.5 {
        floor + 1.0
    } else if (floor as i64) % 2 == 0 {
        floor
    } else {
        floor + 1.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum SpriteKey {
    Box {
        width: i64,
        height: i64,
        colour: Rgb,
        fill: Option<u8>,
        fill_alpha: Option<u8>,
        solid_fill: Option<Rgb>,
        rounded: bool,
        sides: Sides,
        border: i64,
        gap: bool,
    },
    Arrow {
        width: i64,
        height: i64,
        stops: Vec<i64>,
        shaft: i64,
    },
    Led {
        width: i64,
        height: i64,
        colour: u8,
        lit: bool,
    },
}

fn box_key(width: i64, height: i64, style: BoxStyle) -> SpriteKey {
    SpriteKey::Box {
        width,
        height,
        colour: style.colour,
        fill: style.fill,
        fill_alpha: style.fill_alpha,
        solid_fill: style.solid_fill,
        rounded: style.rounded,
        sides: style.sides,
        border: style.border,
        gap: style.gap,
    }
}

fn arrow_key(width: i64, height: i64, style: &ArrowStyle) -> SpriteKey {
    SpriteKey::Arrow {
        width,
        height,
        stops: style.stops.clone(),
        shaft: style.shaft,
    }
}

fn led_key(width: i64, height: i64, style: LedStyle) -> SpriteKey {
    SpriteKey::Led {
        width,
        height,
        colour: style.colour,
        lit: style.lit,
    }
}

struct SolidShape {
    colour: crate::canvas::Rgba,
}

impl crate::canvas::Shape for SolidShape {
    fn colour_at(&self, _x: i64, _y: i64) -> Option<crate::canvas::Rgba> {
        Some(self.colour)
    }
}

struct InsetShape<'a> {
    shape: &'a BoxShape,
    x: i64,
    y: i64,
}

impl crate::canvas::Shape for InsetShape<'_> {
    fn colour_at(&self, x: i64, y: i64) -> Option<crate::canvas::Rgba> {
        self.shape.colour_at(x - self.x, y - self.y)
    }
}

struct CaretShape {
    bar: i64,
    colour: crate::canvas::Rgba,
}

impl crate::canvas::Shape for CaretShape {
    fn colour_at(&self, x: i64, _y: i64) -> Option<crate::canvas::Rgba> {
        (x < self.bar).then_some(self.colour)
    }
}

pub(crate) struct TerminalRenderer {
    window: Window,
    cache: std::collections::HashMap<SpriteKey, Canvas>,
    cache_limit: usize,
    glyph_source: Box<dyn GlyphSource>,
    tile_canvases: std::collections::HashMap<TileKey, Canvas>,
    grow_stamp: u64,
    pub(crate) wezterm: bool,
    vt: super::virtual_terminal::VirtualTerminal,
    clear_pending: bool,
    pending_ops: Vec<super::virtual_terminal::Op>,
}

impl Renderer for TerminalRenderer {
    fn render(&mut self, scene: &Scene<'_>, out: &mut impl Write) -> io::Result<()> {
        let desired = self.paint_scene(scene);
        let mut vt = std::mem::take(&mut self.vt);
        let ops = vt.commit(&desired, self);
        self.vt = vt;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(BEGIN_SYNCHRONIZED_UPDATE.as_bytes());
        if self.clear_pending {
            bytes.extend_from_slice(b"\x1b[2J");
            self.clear_pending = false;
        }
        let mut all_ops = std::mem::take(&mut self.pending_ops);
        all_ops.extend(ops);
        bytes.extend_from_slice(&crate::kitty::encode_ops(&all_ops, self.wezterm));
        bytes.extend_from_slice(END_SYNCHRONIZED_UPDATE.as_bytes());
        out.write_all(&bytes)
    }
}

impl TerminalRenderer {
    pub(crate) fn new(
        window: Window,
        glyph_source: Box<dyn GlyphSource>,
        cache_limit: usize,
    ) -> Self {
        TerminalRenderer {
            window,
            cache: std::collections::HashMap::new(),
            cache_limit,
            glyph_source,
            tile_canvases: std::collections::HashMap::new(),
            grow_stamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos() as u64),
            wezterm: false,
            vt: super::virtual_terminal::VirtualTerminal::new(),
            clear_pending: true,
            pending_ops: Vec::new(),
        }
    }

    pub(crate) fn on_resize(&mut self, window: Window) {
        self.window.cols = window.cols;
        self.window.rows = window.rows;
        self.pending_ops.extend(self.vt.reset());
        self.clear_pending = true;
    }

    pub(crate) fn area(&self) -> Area {
        whole(self.window)
    }

    fn paint_scene(&mut self, scene: &Scene<'_>) -> Vec<super::virtual_terminal::Desired> {
        let mut desired = Vec::new();
        for (area, placements) in scene {
            self.paint(&mut desired, placements, *area);
        }
        desired
    }

    fn paint(&mut self, desired: &mut Vec<Desired>, placements: &[Placement], area: Area) {
        for placement in placements {
            let geometry = Geometry::from(placement);
            match &placement.node {
                PlacementNode::Box {
                    colour,
                    fill,
                    opacity,
                    solid_fill,
                    rounded,
                    sides,
                    border,
                    grow,
                    gap,
                } => self.draw_box(
                    desired,
                    geometry,
                    area,
                    placement.depth,
                    *grow,
                    BoxStyle {
                        colour: *colour,
                        fill: *fill,
                        fill_alpha: quantized_alpha(*opacity),
                        solid_fill: *solid_fill,
                        rounded: *rounded,
                        sides: *sides,
                        border: *border,
                        gap: *gap,
                    },
                ),
                PlacementNode::Brackets { border } => {
                    self.draw_brackets(desired, geometry, area, *border)
                }
                PlacementNode::Arrow(arrow) => self.draw_arrow(
                    desired,
                    geometry,
                    area,
                    ArrowStyle {
                        stops: arrow.stops.clone(),
                        shaft: arrow.shaft,
                    },
                ),
                PlacementNode::Label(label) => self.draw_label(
                    desired,
                    geometry,
                    area,
                    placement.depth,
                    LabelStyle {
                        text: label.text.to_string(),
                        colour: label.colour,
                        bold: label.bold,
                    },
                ),
                PlacementNode::Caret(_) => self.draw_caret(desired, geometry, area),
                PlacementNode::TypingCaret { colour, bold } => {
                    self.draw_typing_caret(desired, geometry, area, placement.depth, *colour, *bold)
                }
                PlacementNode::Cursor(_) => self.draw_cursor(desired, geometry, area),
                PlacementNode::Led { colour, lit } => self.draw_led(
                    desired,
                    geometry,
                    area,
                    LedStyle {
                        colour: *colour,
                        lit: *lit,
                    },
                ),
            }
        }
    }

    fn place_tiles(
        &mut self,
        desired: &mut Vec<Desired>,
        style: BoxStyle,
        geometry: Geometry,
        area: Area,
        z: i32,
    ) -> bool {
        let shape = TileShape {
            style,
            cell: self.cell_size(),
        };
        let Some(tiles) = shape.tiles(geometry.width, geometry.height) else {
            return false;
        };
        for (col, row, key, col_span, row_span) in tiles {
            let tile_col = geometry.x + col;
            let tile_row = geometry.y + row;
            if col_span == 1 && row_span == 1 {
                if let Some((vc, vr, source)) =
                    clip_natural(tile_col, tile_row, 1, 1, area, self.window)
                {
                    desired.push(Desired {
                        image: ImageKey::Tile(key),
                        col: vc,
                        row: vr,
                        z,
                        source,
                        cells: None,
                    });
                }
            } else if let Some((vc, vr, cs, rs)) =
                clip_stretched(tile_col, tile_row, col_span, row_span, area, self.window)
            {
                desired.push(Desired {
                    image: ImageKey::Tile(key),
                    col: vc,
                    row: vr,
                    z,
                    source: None,
                    cells: Some((cs, rs)),
                });
            }
        }
        true
    }

    fn draw_box(
        &mut self,
        desired: &mut Vec<Desired>,
        geometry: Geometry,
        area: Area,
        depth: u8,
        grow: bool,
        style: BoxStyle,
    ) {
        let z = depth_z(depth);
        if grow && self.place_growing(desired, geometry, area, z, style) {
            return;
        }
        if self.place_tiles(desired, style, geometry, area, z) {
            return;
        }
        let key = box_key(geometry.width, geometry.height, style);
        if let Some((col, row, source)) = clip_natural(
            geometry.x,
            geometry.y,
            geometry.width,
            geometry.height,
            area,
            self.window,
        ) {
            if !self.cache.contains_key(&key) {
                let canvas = self.build_sprite(&key);
                self.remember(key.clone(), canvas);
            }
            desired.push(Desired {
                image: ImageKey::Sprite(key),
                col,
                row,
                z,
                source,
                cells: None,
            });
        }
    }

    fn place_growing(
        &mut self,
        desired: &mut Vec<Desired>,
        geometry: Geometry,
        area: Area,
        z: i32,
        style: BoxStyle,
    ) -> bool {
        let left = geometry.x;
        let top = geometry.y;
        let right = geometry.x + geometry.width;
        let bottom = geometry.y + geometry.height;
        if left < area.col
            || top < area.row
            || right > area.col + area.cols
            || bottom > area.row + area.rows
            || left < 0
            || top < 0
            || right > self.window.cols
            || bottom > self.window.rows
        {
            return false;
        }
        self.grow_stamp = self.grow_stamp.wrapping_add(1);
        desired.push(Desired {
            image: ImageKey::Grow(GrowKey {
                style_key: box_key(geometry.width, geometry.height, style),
                stamp: self.grow_stamp,
            }),
            col: geometry.x,
            row: geometry.y,
            z,
            source: None,
            cells: None,
        });
        true
    }

    fn draw_brackets(
        &mut self,
        desired: &mut Vec<Desired>,
        geometry: Geometry,
        area: Area,
        border: i64,
    ) {
        let cell = self.cell_size();
        let blocks = corner_cells(cell);
        for corner in CORNERS {
            let (col, row) = corner_offset(corner, geometry.width, geometry.height, cell);
            let bx = geometry.x + col;
            let by = geometry.y + row;
            if let Some((vc, vr, source)) = clip_natural(bx, by, blocks, blocks, area, self.window)
            {
                desired.push(Desired {
                    image: ImageKey::Bracket(BracketKey {
                        corner,
                        border,
                        cell,
                    }),
                    col: vc,
                    row: vr,
                    z: BRACKETS_Z,
                    source,
                    cells: None,
                });
            }
        }
    }

    fn draw_led(
        &mut self,
        desired: &mut Vec<Desired>,
        geometry: Geometry,
        area: Area,
        style: LedStyle,
    ) {
        let key = led_key(geometry.width, geometry.height, style);
        if let Some((col, row, source)) = clip_natural(
            geometry.x,
            geometry.y,
            geometry.width,
            geometry.height,
            area,
            self.window,
        ) {
            if !self.cache.contains_key(&key) {
                let canvas = self.build_sprite(&key);
                self.remember(key.clone(), canvas);
            }
            desired.push(Desired {
                image: ImageKey::Sprite(key),
                col,
                row,
                z: INK_Z,
                source,
                cells: None,
            });
        }
    }

    fn draw_arrow(
        &mut self,
        desired: &mut Vec<Desired>,
        geometry: Geometry,
        area: Area,
        style: ArrowStyle,
    ) {
        let key = arrow_key(geometry.width, geometry.height, &style);
        if let Some((col, row, source)) = clip_natural(
            geometry.x,
            geometry.y,
            geometry.width,
            geometry.height,
            area,
            self.window,
        ) {
            if !self.cache.contains_key(&key) {
                let canvas = self.build_sprite(&key);
                self.remember(key.clone(), canvas);
            }
            desired.push(Desired {
                image: ImageKey::Sprite(key),
                col,
                row,
                z: CONTENT_Z,
                source,
                cells: None,
            });
        }
    }

    fn draw_label(
        &mut self,
        desired: &mut Vec<Desired>,
        geometry: Geometry,
        area: Area,
        depth: u8,
        style: LabelStyle,
    ) {
        let z = depth_z(depth);
        for (offset, character) in style.text.chars().enumerate() {
            let char_x = geometry.x + offset as i64;
            let char_y = geometry.y;
            if let Some((col, row, source)) = clip_natural(char_x, char_y, 1, 1, area, self.window)
            {
                let key = GlyphKey {
                    character,
                    colour: style.colour,
                    bold: style.bold,
                };
                desired.push(Desired {
                    image: ImageKey::Glyph(key),
                    col,
                    row,
                    z,
                    source,
                    cells: None,
                });
            }
        }
    }

    fn draw_caret(&mut self, desired: &mut Vec<Desired>, geometry: Geometry, area: Area) {
        if let Some((col, row, source)) = clip_natural(
            geometry.x,
            geometry.y,
            geometry.width,
            geometry.height,
            area,
            self.window,
        ) {
            desired.push(Desired {
                image: ImageKey::Caret(CaretKey {
                    cols: geometry.width,
                    rows: geometry.height,
                }),
                col,
                row,
                z: CONTENT_Z,
                source,
                cells: None,
            });
        }
    }

    fn draw_typing_caret(
        &mut self,
        desired: &mut Vec<Desired>,
        geometry: Geometry,
        area: Area,
        depth: u8,
        colour: Rgb,
        bold: bool,
    ) {
        if let Some((col, row, source)) = clip_natural(
            geometry.x,
            geometry.y,
            geometry.width,
            geometry.height,
            area,
            self.window,
        ) {
            desired.push(Desired {
                image: ImageKey::TypingCaret(TypingCaretKey { colour, bold }),
                col,
                row,
                z: depth_z(depth),
                source,
                cells: None,
            });
        }
    }

    fn draw_cursor(&mut self, desired: &mut Vec<Desired>, geometry: Geometry, area: Area) {
        self.draw_caret(desired, geometry, area);
    }

    fn remember(&mut self, key: SpriteKey, drawn: Canvas) {
        if self.cache.len() >= self.cache_limit {
            self.cache.clear();
        }
        self.cache.insert(key, drawn);
    }

    fn cell_size(&self) -> CellSize {
        CellSize {
            width: self.window.cell_width,
            height: self.window.cell_height,
        }
    }

    fn cells_to_pixels_x(&self, cells: i64) -> i64 {
        cells * self.window.cell_width
    }

    fn cells_to_pixels_y(&self, cells: i64) -> i64 {
        cells * self.window.cell_height
    }

    fn box_canvas(&self, cell_width: i64, cell_height: i64, style: BoxStyle) -> Canvas {
        let width = self.cells_to_pixels_x(cell_width);
        let height = self.cells_to_pixels_y(cell_height);
        if !style.gap {
            return Canvas::fill(width, height, &box_shape(width, height, style));
        }
        let inset_x = self.window.cell_width - style.border - GAP_PX;
        let inset_y = self.window.cell_height - style.border - GAP_PX;
        let shape = box_shape(
            (width - 2 * inset_x).max(1),
            (height - 2 * inset_y).max(1),
            style,
        );
        Canvas::fill(
            width,
            height,
            &InsetShape {
                shape: &shape,
                x: inset_x,
                y: inset_y,
            },
        )
    }

    fn led_canvas(&self, cell_width: i64, cell_height: i64, style: LedStyle) -> Canvas {
        let width = self.cells_to_pixels_x(cell_width);
        let height = self.cells_to_pixels_y(cell_height);
        let shape = LedShape {
            width,
            height,
            colour: colour(Some(style.colour)),
            lit: style.lit,
        };
        Canvas::fill(width, height, &shape)
    }

    fn arrow_canvas(&self, cell_width: i64, cell_height: i64, style: &ArrowStyle) -> Canvas {
        let width = self.cells_to_pixels_x(cell_width);
        let height = self.cells_to_pixels_y(cell_height);
        let stop_rows: Vec<i64> = style
            .stops
            .iter()
            .map(|stop| self.cells_to_pixels_y(*stop) + self.window.cell_height / 2)
            .collect();
        let trunk = (
            *stop_rows
                .iter()
                .min()
                .expect("an arrow always has at least one stop"),
            *stop_rows
                .iter()
                .max()
                .expect("an arrow always has at least one stop"),
        );
        let (r, g, b) = colour(None);
        let shape = ArrowShape {
            width,
            stop_rows,
            shaft_row: self.cells_to_pixels_y(style.shaft) + self.window.cell_height / 2,
            trunk,
            stroke: ARROW_STROKE,
            arrowhead_edge_length: ARROWHEAD_EDGE_LENGTH,
            ink: [r, g, b, (ARROW_OPACITY * OPAQUE as f64).round() as u8],
        };
        Canvas::fill(width, height, &shape)
    }

    fn build_sprite(&self, key: &SpriteKey) -> Canvas {
        match key {
            SpriteKey::Box {
                width,
                height,
                colour,
                fill,
                fill_alpha,
                solid_fill,
                rounded,
                sides,
                border,
                gap,
            } => self.box_canvas(
                *width,
                *height,
                BoxStyle {
                    colour: *colour,
                    fill: *fill,
                    fill_alpha: *fill_alpha,
                    solid_fill: *solid_fill,
                    rounded: *rounded,
                    sides: *sides,
                    border: *border,
                    gap: *gap,
                },
            ),
            SpriteKey::Arrow {
                width,
                height,
                stops,
                shaft,
            } => self.arrow_canvas(
                *width,
                *height,
                &ArrowStyle {
                    stops: stops.clone(),
                    shaft: *shaft,
                },
            ),
            SpriteKey::Led {
                width,
                height,
                colour,
                lit,
            } => self.led_canvas(
                *width,
                *height,
                LedStyle {
                    colour: *colour,
                    lit: *lit,
                },
            ),
        }
    }
}

impl Sprites for TerminalRenderer {
    fn content(&mut self, key: &ImageKey) -> Content {
        match key {
            ImageKey::Glyph(gk) => Content::Still(
                self.glyph_source
                    .glyph(gk.character, gk.colour, gk.bold)
                    .clone(),
            ),
            ImageKey::Tile(tk) => Content::Still(
                self.tile_canvases
                    .entry(*tk)
                    .or_insert_with(|| tk.canvas())
                    .clone(),
            ),
            ImageKey::Bracket(bk) => Content::Still(bk.canvas()),
            ImageKey::Sprite(sk) => {
                if !self.cache.contains_key(sk) {
                    let canvas = self.build_sprite(sk);
                    self.remember(sk.clone(), canvas);
                }
                Content::Still(self.cache[sk].clone())
            }
            ImageKey::Caret(ck) => {
                let width = self.cells_to_pixels_x(ck.cols);
                let height = self.cells_to_pixels_y(ck.rows);
                let (r, g, b) = colour(None);
                Content::Still(Canvas::fill(
                    width,
                    height,
                    &SolidShape {
                        colour: [r, g, b, OPAQUE],
                    },
                ))
            }
            ImageKey::TypingCaret(ck) => {
                let width = self.cells_to_pixels_x(1);
                let height = self.cells_to_pixels_y(1);
                let bar = (self.window.cell_width / 8).max(1);
                let (r, g, b) = ck.colour;
                Content::Still(Canvas::fill(
                    width,
                    height,
                    &CaretShape {
                        bar,
                        colour: [r, g, b, OPAQUE],
                    },
                ))
            }
            ImageKey::Grow(gk) => {
                let (width, height, style) = match &gk.style_key {
                    SpriteKey::Box {
                        width,
                        height,
                        colour,
                        fill,
                        fill_alpha,
                        solid_fill,
                        rounded,
                        sides,
                        border,
                        gap,
                    } => (
                        *width,
                        *height,
                        BoxStyle {
                            colour: *colour,
                            fill: *fill,
                            fill_alpha: *fill_alpha,
                            solid_fill: *solid_fill,
                            rounded: *rounded,
                            sides: *sides,
                            border: *border,
                            gap: *gap,
                        },
                    ),
                    _ => panic!("grow key must be a box sprite"),
                };
                let pw = self.cells_to_pixels_x(width);
                let ph = self.cells_to_pixels_y(height);
                let frames = grow_frames(pw, ph, style);
                let root = grow_root(&frames[GROW_FRAMES - 1], gk.stamp);
                Content::Animation { root, frames }
            }
        }
    }
}

fn clip_natural(
    x: i64,
    y: i64,
    width: i64,
    height: i64,
    area: Area,
    window: Window,
) -> Option<(i64, i64, Option<SourceRect>)> {
    let col = x.max(area.col).max(0);
    let row = y.max(area.row).max(0);
    let right = (x + width).min(area.col + area.cols).min(window.cols);
    let bottom = (y + height).min(area.row + area.rows).min(window.rows);
    if col >= right || row >= bottom {
        return None;
    }
    let first_x = (col - x) * window.cell_width;
    let last_x = (right - x) * window.cell_width;
    let first_y = (row - y) * window.cell_height;
    let last_y = (bottom - y) * window.cell_height;
    let fully_visible = first_x == 0
        && first_y == 0
        && last_x == width * window.cell_width
        && last_y == height * window.cell_height;
    let source = if fully_visible {
        None
    } else {
        Some(SourceRect {
            x: first_x,
            y: first_y,
            width: last_x - first_x,
            height: last_y - first_y,
        })
    };
    Some((col, row, source))
}

fn clip_stretched(
    col_start: i64,
    row_start: i64,
    col_span: i64,
    row_span: i64,
    area: Area,
    window: Window,
) -> Option<(i64, i64, i64, i64)> {
    let col = col_start.max(area.col).max(0);
    let row = row_start.max(area.row).max(0);
    let right = (col_start + col_span)
        .min(area.col + area.cols)
        .min(window.cols);
    let bottom = (row_start + row_span)
        .min(area.row + area.rows)
        .min(window.rows);
    if col >= right || row >= bottom {
        return None;
    }
    Some((col, row, right - col, bottom - row))
}

#[cfg(test)]
mod tests {
    use super::super::font::FakeGlyphSource;
    use super::super::tiles::{cells_with_middle, TileShape};
    use super::super::virtual_terminal::{Content, Desired, SourceRect, Sprites, TypingCaretKey};
    use super::*;
    use crate::state::Mode;
    use crate::style::{BOX_FILL_OPACITY, CELL_HEIGHT, CELL_WIDTH, FOOTER_FILL_OPACITY};
    use crate::view::editor;
    use crate::view::{ALL_SIDES, BORDER, BRACKET_MARGIN, FOOTER_ROWS};
    use crate::State;

    #[test]
    fn fill_colour_of_plain_is_transparent() {
        assert_eq!(fill_colour(None, None), TRANSPARENT);
    }

    #[test]
    fn fill_colour_of_a_palette_index_is_alpha_composited_and_opaque() {
        let (r, g, b) = palette(2).unwrap();
        let alpha = (BOX_FILL_OPACITY * OPAQUE as f64).round() as u8;
        let round = |channel: u8| (channel as f64 * alpha as f64 / OPAQUE as f64).round() as u8;
        let expected = (round(r), round(g), round(b), OPAQUE);
        assert_eq!(
            fill_colour(Some(2), quantized_alpha(Some(BOX_FILL_OPACITY))),
            expected
        );
    }

    #[test]
    fn fill_colour_of_the_footer_opacity_matches_a_regular_box_fill() {
        let box_fill = fill_colour(
            Some(crate::style::FOREGROUND),
            quantized_alpha(Some(BOX_FILL_OPACITY)),
        );
        let footer_fill = fill_colour(
            Some(crate::style::FOREGROUND),
            quantized_alpha(Some(FOOTER_FILL_OPACITY)),
        );
        assert!(box_fill.0 > 0);
        assert_eq!(footer_fill, box_fill);
    }

    #[test]
    fn box_shape_with_a_solid_fill_is_filled_with_that_colour_opaque() {
        let (r, g, b) = (12, 34, 56);
        let shape = box_shape(
            30,
            30,
            BoxStyle {
                colour: colour(None),
                fill: None,
                fill_alpha: None,
                solid_fill: Some((r, g, b)),
                rounded: false,
                sides: ALL_SIDES,
                border: BORDER,
                gap: false,
            },
        );
        assert_eq!(shape.fill, [r, g, b, OPAQUE]);
    }

    fn edge_rgba(index: Option<u8>) -> (u8, u8, u8, u8) {
        let (r, g, b) = colour(index);
        (r, g, b, OPAQUE)
    }

    fn box_pixels(
        width: i64,
        height: i64,
        radius: i64,
        edge: (u8, u8, u8, u8),
        fill: (u8, u8, u8, u8),
    ) -> Vec<u8> {
        let shape = BoxShape {
            width,
            height,
            border: BORDER,
            radius,
            sides: ALL_SIDES,
            edge: [edge.0, edge.1, edge.2, edge.3],
            fill: [fill.0, fill.1, fill.2, fill.3],
        };
        Canvas::fill(width, height, &shape).pixels
    }

    fn pixel_at(pixels: &[u8], width: i64, x: i64, y: i64) -> (u8, u8, u8, u8) {
        let offset = ((y * width + x) * 4) as usize;
        (
            pixels[offset],
            pixels[offset + 1],
            pixels[offset + 2],
            pixels[offset + 3],
        )
    }

    #[test]
    fn plain_fill_renders_transparent_interior() {
        let size = 2 * BORDER + 3;
        let edge = edge_rgba(None);
        let fill = fill_colour(None, None);
        let pixels = box_pixels(size, size, 0, edge, fill);
        assert_eq!(pixel_at(&pixels, size, BORDER + 1, BORDER + 1), TRANSPARENT);
    }

    #[test]
    fn a_fill_colour_is_composited_over_black_and_made_opaque() {
        let size = 2 * BORDER + 3;
        let edge = edge_rgba(None);
        let fill = fill_colour(Some(2), quantized_alpha(Some(BOX_FILL_OPACITY)));
        let pixels = box_pixels(size, size, 0, edge, fill);
        assert_eq!(
            pixel_at(&pixels, size, BORDER + 1, BORDER + 1),
            fill_colour(Some(2), quantized_alpha(Some(BOX_FILL_OPACITY)))
        );
    }

    #[test]
    fn border_pixels_are_unaffected_by_fill() {
        let size = 2 * BORDER + 3;
        let edge = edge_rgba(Some(3));
        let fill = fill_colour(Some(2), quantized_alpha(Some(BOX_FILL_OPACITY)));
        let pixels = box_pixels(size, size, 0, edge, fill);
        assert_eq!(pixel_at(&pixels, size, 0, 0), edge_rgba(Some(3)));
        assert_eq!(
            pixel_at(&pixels, size, BORDER + 1, BORDER + 1),
            fill_colour(Some(2), quantized_alpha(Some(BOX_FILL_OPACITY)))
        );
    }

    #[test]
    fn a_border_is_bold_at_every_edge() {
        let size = 3 * 4;
        let edge = edge_rgba(Some(1));
        let fill = fill_colour(Some(2), quantized_alpha(Some(BOX_FILL_OPACITY)));
        let pixels = box_pixels(size, size, 0, edge, fill);
        for offset in 0..BORDER {
            assert_eq!(pixel_at(&pixels, size, 5, offset), edge);
            assert_eq!(pixel_at(&pixels, size, 5, size - 1 - offset), edge);
        }
        assert_eq!(pixel_at(&pixels, size, 5, BORDER), fill);
        assert_eq!(pixel_at(&pixels, size, 5, size - 1 - BORDER), fill);
        for offset in 0..BORDER {
            assert_eq!(pixel_at(&pixels, size, offset, 5), edge);
            assert_eq!(pixel_at(&pixels, size, size - 1 - offset, 5), edge);
        }
        assert_eq!(pixel_at(&pixels, size, BORDER, 5), fill);
        assert_eq!(pixel_at(&pixels, size, size - 1 - BORDER, 5), fill);
    }

    const CORNER_SIZE: i64 = 80;

    #[test]
    fn a_square_box_is_built_from_flat_edge_and_body_rows() {
        let edge = edge_rgba(Some(1));
        let fill = fill_colour(Some(2), quantized_alpha(Some(BOX_FILL_OPACITY)));
        let pixels = box_pixels(CORNER_SIZE, CORNER_SIZE, 0, edge, fill);

        let edge_px = [edge.0, edge.1, edge.2, edge.3];
        let fill_px = [fill.0, fill.1, fill.2, fill.3];
        let edge_row = edge_px.repeat(CORNER_SIZE as usize);
        let mut expected_body_row = Vec::new();
        expected_body_row.extend(edge_px.repeat(BORDER as usize));
        expected_body_row.extend(fill_px.repeat((CORNER_SIZE - 2 * BORDER) as usize));
        expected_body_row.extend(edge_px.repeat(BORDER as usize));

        let mut expected = Vec::new();
        expected.extend(edge_row.repeat(BORDER as usize));
        expected.extend(expected_body_row.repeat((CORNER_SIZE - 2 * BORDER) as usize));
        expected.extend(edge_row.repeat(BORDER as usize));
        assert_eq!(pixels, expected);
    }

    #[test]
    fn a_rounded_box_cuts_away_its_extreme_corners() {
        let edge = edge_rgba(Some(1));
        let fill = fill_colour(Some(2), quantized_alpha(Some(BOX_FILL_OPACITY)));
        let pixels = box_pixels(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, edge, fill);
        let (last_x, last_y) = (CORNER_SIZE - 1, CORNER_SIZE - 1);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, 0, 0), TRANSPARENT);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, last_x, 0), TRANSPARENT);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, 0, last_y), TRANSPARENT);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, last_x, last_y), TRANSPARENT);
    }

    #[test]
    fn straight_edges_stay_as_crisp_as_a_square_box() {
        let edge = edge_rgba(Some(1));
        let fill = fill_colour(Some(2), quantized_alpha(Some(BOX_FILL_OPACITY)));
        let square = box_pixels(CORNER_SIZE, CORNER_SIZE, 0, edge, fill);
        let rounded = box_pixels(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, edge, fill);
        let middle_y = CORNER_SIZE / 2;
        let middle_x = CORNER_SIZE / 2;
        for x in 0..CORNER_SIZE {
            assert_eq!(
                pixel_at(&rounded, CORNER_SIZE, x, middle_y),
                pixel_at(&square, CORNER_SIZE, x, middle_y)
            );
        }
        for y in 0..CORNER_SIZE {
            assert_eq!(
                pixel_at(&rounded, CORNER_SIZE, middle_x, y),
                pixel_at(&square, CORNER_SIZE, middle_x, y)
            );
        }
    }

    #[test]
    fn the_arc_is_anti_aliased() {
        let edge = edge_rgba(Some(1));
        let fill = TRANSPARENT;
        let square = box_pixels(CORNER_SIZE, CORNER_SIZE, 0, edge, fill);
        let rounded = box_pixels(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, edge, fill);
        assert!(!square
            .iter()
            .skip(3)
            .step_by(4)
            .any(|&alpha| alpha > 0 && alpha < OPAQUE));
        assert!(rounded
            .iter()
            .skip(3)
            .step_by(4)
            .any(|&alpha| alpha > 0 && alpha < OPAQUE));
    }

    #[test]
    fn arc_coverage_is_continuous_at_the_pixel_centre() {
        let edge = edge_rgba(Some(1));
        let fill = TRANSPARENT;
        let pixels = box_pixels(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, edge, fill);
        let (r, g, b) = colour(Some(1));
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, 14, 2), (r, g, b, 254));
    }

    #[test]
    fn border_coverage_is_composed_over_the_opaque_fill() {
        let edge = edge_rgba(Some(1));
        let fill = fill_colour(Some(2), quantized_alpha(Some(BOX_FILL_OPACITY)));
        let pixels = box_pixels(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, edge, fill);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, 20, 4), (33, 91, 76, OPAQUE));
    }

    #[test]
    fn a_rounded_box_cuts_away_more_than_a_square_one() {
        let edge = edge_rgba(Some(1));
        let fill = fill_colour(Some(2), quantized_alpha(Some(BOX_FILL_OPACITY)));
        let square = box_pixels(CORNER_SIZE, CORNER_SIZE, 0, edge, fill);
        let rounded = box_pixels(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, edge, fill);
        let alpha_total = |pixels: &[u8]| {
            pixels
                .iter()
                .skip(3)
                .step_by(4)
                .map(|&a| a as u64)
                .sum::<u64>()
        };
        assert!(alpha_total(&rounded) < alpha_total(&square));
    }

    #[test]
    fn the_fringe_keeps_the_edge_colour_instead_of_fading_to_black() {
        let edge = edge_rgba(Some(1));
        let fill = TRANSPARENT;
        let pixels = box_pixels(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, edge, fill);
        let (r, g, b) = colour(Some(1));
        let mut found_partial = false;
        for y in 0..CORNER_SIZE {
            for x in 0..CORNER_SIZE {
                let (pr, pg, pb, pa) = pixel_at(&pixels, CORNER_SIZE, x, y);
                if pa > 0 && pa < OPAQUE {
                    found_partial = true;
                    assert_eq!((pr, pg, pb), (r, g, b));
                }
            }
        }
        assert!(found_partial);
    }

    const SMALL_SIZE: i64 = 20;

    #[test]
    fn the_sprite_holds_exactly_one_pixel_per_cell_of_its_area() {
        let edge = edge_rgba(Some(1));
        let fill = fill_colour(Some(2), quantized_alpha(Some(BOX_FILL_OPACITY)));
        let pixels = box_pixels(SMALL_SIZE, SMALL_SIZE, ROUNDED_RADIUS, edge, fill);
        assert_eq!(pixels.len() as i64, SMALL_SIZE * SMALL_SIZE * 4);
    }

    fn box_node(colour: Option<u8>, fill: Option<u8>, rounded: bool) -> PlacementNode<'static> {
        PlacementNode::Box {
            colour: crate::style::rgb(colour),
            fill,
            opacity: fill.map(|_| BOX_FILL_OPACITY),
            solid_fill: None,
            rounded,
            sides: ALL_SIDES,
            border: BORDER,
            grow: false,
            gap: false,
        }
    }

    fn with_sides(node: &PlacementNode<'static>, new_sides: Sides) -> PlacementNode<'static> {
        match node.clone() {
            PlacementNode::Box {
                colour,
                fill,
                opacity,
                solid_fill,
                rounded,
                border,
                grow,
                gap,
                ..
            } => PlacementNode::Box {
                colour,
                fill,
                opacity,
                solid_fill,
                rounded,
                sides: new_sides,
                border,
                grow,
                gap,
            },
            _ => panic!("expected a Box"),
        }
    }

    fn with_border(node: &PlacementNode<'static>, new_border: i64) -> PlacementNode<'static> {
        match node.clone() {
            PlacementNode::Box {
                colour,
                fill,
                opacity,
                solid_fill,
                rounded,
                sides,
                grow,
                gap,
                ..
            } => PlacementNode::Box {
                colour,
                fill,
                opacity,
                solid_fill,
                rounded,
                sides,
                border: new_border,
                grow,
                gap,
            },
            _ => panic!("expected a Box"),
        }
    }

    fn with_gap(node: &PlacementNode<'static>, new_gap: bool) -> PlacementNode<'static> {
        match node.clone() {
            PlacementNode::Box {
                colour,
                fill,
                opacity,
                solid_fill,
                rounded,
                sides,
                border,
                grow,
                ..
            } => PlacementNode::Box {
                colour,
                fill,
                opacity,
                solid_fill,
                rounded,
                sides,
                border,
                grow,
                gap: new_gap,
            },
            _ => panic!("expected a Box"),
        }
    }

    fn selected_box_node() -> PlacementNode<'static> {
        PlacementNode::Brackets { border: BORDER }
    }

    fn box_placement(
        node: &PlacementNode<'static>,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
    ) -> crate::view::Placement<'static> {
        crate::view::Placement {
            node: node.clone(),
            x,
            y,
            width,
            height,
            depth: 0,
        }
    }

    fn arrow_placement(
        stops: Vec<i64>,
        shaft: i64,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
    ) -> crate::view::Placement<'static> {
        crate::view::Placement {
            node: crate::view::PlacementNode::Arrow(crate::view::Arrow { stops, shaft }),
            x,
            y,
            width,
            height,
            depth: 0,
        }
    }

    fn key_of(placement: &Placement) -> SpriteKey {
        match &placement.node {
            PlacementNode::Box {
                colour,
                fill,
                opacity,
                solid_fill,
                rounded,
                sides,
                border,
                gap,
                ..
            } => box_key(
                placement.width,
                placement.height,
                BoxStyle {
                    colour: *colour,
                    fill: *fill,
                    fill_alpha: quantized_alpha(*opacity),
                    solid_fill: *solid_fill,
                    rounded: *rounded,
                    sides: *sides,
                    border: *border,
                    gap: *gap,
                },
            ),
            PlacementNode::Arrow(arrow) => arrow_key(
                placement.width,
                placement.height,
                &ArrowStyle {
                    stops: arrow.stops.clone(),
                    shaft: arrow.shaft,
                },
            ),
            PlacementNode::Led { colour, lit } => led_key(
                placement.width,
                placement.height,
                LedStyle {
                    colour: *colour,
                    lit: *lit,
                },
            ),
            _ => panic!("expected a cached placement"),
        }
    }

    #[test]
    fn sprite_key_of_two_identically_shaped_boxes_is_equal() {
        let node_a = box_node(Some(1), Some(1), true);
        let node_b = box_node(Some(1), Some(1), true);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_eq!(key_of(&a), key_of(&b));
    }

    #[test]
    fn sprite_key_differs_by_colour() {
        let node_a = box_node(Some(1), None, true);
        let node_b = box_node(Some(2), None, true);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_ne!(key_of(&a), key_of(&b));
    }

    #[test]
    fn sprite_key_differs_by_fill() {
        let node_a = box_node(Some(1), Some(1), true);
        let node_b = box_node(Some(1), None, true);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_ne!(key_of(&a), key_of(&b));
    }

    #[test]
    fn sprite_key_differs_by_rounded() {
        let node_a = box_node(Some(1), Some(1), true);
        let node_b = box_node(Some(1), Some(1), false);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_ne!(key_of(&a), key_of(&b));
    }

    #[test]
    fn sprite_key_differs_by_sides() {
        let node_a = box_node(None, None, false);
        let node_b = with_sides(&node_a, (true, false, false, true));
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_ne!(key_of(&a), key_of(&b));
    }

    #[test]
    fn sprite_key_differs_by_border() {
        let node_a = box_node(None, None, false);
        let node_b = with_border(&node_a, BORDER + 1);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_ne!(key_of(&a), key_of(&b));
    }

    #[test]
    fn sprite_key_differs_by_gap() {
        let node_a = box_node(None, None, false);
        let node_b = with_gap(&node_a, true);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        let c = box_placement(&with_gap(&node_a, false), 0, 0, 10, 10);
        assert_ne!(key_of(&a), key_of(&b));
        assert_eq!(key_of(&a), key_of(&c));
    }

    #[test]
    fn box_edge_colour_is_unchanged_by_selection() {
        let r = renderer(1, 1);
        let plain = box_outline(&r, &box_node(Some(1), None, false), 10, 10);
        let plain_pixel = pixel_of(&plain, 5, 0);
        let selected_pixel = pixel_of(
            &box_outline(&r, &box_node(Some(1), None, false), 10, 10),
            5,
            0,
        );
        assert_eq!(selected_pixel, plain_pixel);
    }

    #[test]
    fn boxes_differing_only_in_sides_or_border_are_cached_distinctly() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let plain = box_node(None, None, false);
        let variants = [
            plain.clone(),
            with_sides(&plain, (true, false, false, true)),
            with_border(&plain, BORDER + 1),
        ];
        for variant in &variants {
            paint_desired(&mut r, &[box_placement(variant, 0, 0, 4, 3)]);
        }
        assert_eq!(r.cache.len(), variants.len());
    }

    #[test]
    fn sprite_key_of_arrows_with_different_stops_differs() {
        let a = arrow_placement(vec![0, 2], 1, 0, 0, 4, 3);
        let b = arrow_placement(vec![0, 3], 1, 0, 0, 4, 3);
        assert_ne!(key_of(&a), key_of(&b));
    }

    #[test]
    fn sprite_key_of_identical_arrows_is_equal() {
        let a = arrow_placement(vec![0, 2], 1, 0, 0, 4, 3);
        let b = arrow_placement(vec![0, 2], 1, 0, 0, 4, 3);
        assert_eq!(key_of(&a), key_of(&b));
    }

    #[test]
    fn an_unselected_box_places_no_brackets() {
        let mut r = renderer_on(window(20, 20, 2, 2));
        let node = box_node(Some(1), None, false);
        let images = paint_desired(&mut r, &[box_placement(&node, 4, 4, 4, 4)]);
        assert_eq!(images.len(), 1);
    }

    #[test]
    fn a_selected_box_places_its_brackets_above_the_fill_and_below_content() {
        let mut r = renderer_on(window(20, 20, 2, 2));
        let node = selected_box_node();
        let images = paint_desired(
            &mut r,
            &[
                box_placement(&box_node(Some(1), None, false), 4, 4, 4, 4),
                box_placement(&node, 3, 3, 6, 6),
                caret_placement(5, 5, 1, 1),
            ],
        );
        let layers: std::collections::BTreeSet<i32> = images.iter().map(|image| image.z).collect();
        let brackets = composed_desired(&mut r, &images, BRACKETS_Z);
        let boxed = composed_desired(&mut r, &images, depth_z(0));
        let content = images
            .iter()
            .find(|image| {
                (image.col, image.row) == (5, 5) && ![depth_z(0), BRACKETS_Z].contains(&image.z)
            })
            .unwrap();
        assert_eq!((boxed.col, boxed.row), (4, 4));
        assert_eq!((brackets.col, brackets.row), (3, 3));
        assert_eq!(
            layers.into_iter().collect::<Vec<_>>(),
            vec![depth_z(0), BRACKETS_Z, content.z]
        );
    }

    fn z_at(images: &[Desired], cell: (i64, i64)) -> i32 {
        images
            .iter()
            .find(|image| (image.col, image.row) == cell)
            .unwrap()
            .z
    }

    #[test]
    fn a_deeper_box_is_placed_above_a_shallower_one() {
        let mut r = renderer_on(window(20, 20, 2, 2));
        let deeper = crate::view::Placement {
            depth: 1,
            ..box_placement(&box_node(None, None, false), 12, 12, 3, 3)
        };
        let images = paint_desired(
            &mut r,
            &[
                box_placement(&box_node(Some(1), None, false), 2, 2, 3, 3),
                deeper,
            ],
        );
        assert!(z_at(&images, (12, 12)) > z_at(&images, (2, 2)));
    }

    #[test]
    fn a_deeper_box_is_placed_above_a_shallower_label() {
        let mut r = renderer_on(window(20, 20, 2, 2));
        let label = crate::view::Placement {
            depth: 0,
            ..label_placement("x", 2, 2, 1, 1)
        };
        let deeper = crate::view::Placement {
            depth: 1,
            ..box_placement(&box_node(None, None, false), 12, 12, 3, 3)
        };
        let images = paint_desired(&mut r, &[label, deeper]);
        assert!(z_at(&images, (12, 12)) > z_at(&images, (2, 2)));
    }

    #[test]
    fn the_brackets_is_centred_on_the_box_and_extends_beyond_it() {
        let mut r = renderer_on(window(20, 20, 2, 2));
        let node = selected_box_node();
        let images = paint_desired(&mut r, &[box_placement(&node, 7, 7, 6, 6)]);
        let brackets = composed_desired(&mut r, &images, BRACKETS_Z);
        assert_eq!(brackets.col, 8 - BRACKET_MARGIN);
        assert_eq!(brackets.row, 8 - BRACKET_MARGIN);
        assert_eq!(
            brackets.canvas.width,
            (4 + 2 * BRACKET_MARGIN) * r.window.cell_width
        );
        assert_eq!(
            brackets.canvas.height,
            (4 + 2 * BRACKET_MARGIN) * r.window.cell_height
        );
    }

    #[test]
    fn transparent_sprite_padding_is_cell_aligned_independently_of_bracket_thickness() {
        let mut r = renderer_on(window(20, 20, 5, 9));
        let node = selected_box_node();
        let images = paint_desired(&mut r, &[box_placement(&node, 3, 3, 6, 6)]);
        let brackets = composed_desired(&mut r, &images, BRACKETS_Z);
        assert_eq!(brackets.col, 4 - BRACKET_MARGIN);
        assert_eq!(brackets.row, 4 - BRACKET_MARGIN);
        let padding_px_x = BRACKET_MARGIN * r.window.cell_width;
        let padding_px_y = BRACKET_MARGIN * r.window.cell_height;
        assert_ne!(padding_px_x, padding_px_y);
        assert_eq!(
            brackets.canvas.width,
            (4 + 2 * BRACKET_MARGIN) * r.window.cell_width
        );
        assert_eq!(
            brackets.canvas.height,
            (4 + 2 * BRACKET_MARGIN) * r.window.cell_height
        );
    }

    #[test]
    fn the_brackets_are_drawn_in_the_foreground_colour() {
        let mut r = renderer_on(window(20, 20, CELL_WIDTH, CELL_HEIGHT));
        let node = selected_box_node();
        let images = paint_desired(&mut r, &[box_placement(&node, 3, 3, 6, 6)]);
        let brackets = composed_desired(&mut r, &images, BRACKETS_Z);
        let (foreground_r, foreground_g, foreground_b) = colour(None);
        let painted: Vec<&[u8]> = brackets
            .canvas
            .pixels
            .chunks(4)
            .filter(|pixel| pixel[3] > 0)
            .collect();
        assert!(!painted.is_empty());
        assert!(painted.iter().all(|pixel| {
            (pixel[0], pixel[1], pixel[2], pixel[3])
                == (foreground_r, foreground_g, foreground_b, OPAQUE)
        }));
    }

    #[test]
    fn moving_or_resizing_brackets_uploads_no_new_image_after_the_first_frame() {
        let mut r = renderer_on(window(80, 80, CELL_WIDTH, CELL_HEIGHT));
        let node = selected_box_node();
        let blocks = corner_cells(r.cell_size());
        let first = rendered_placements(
            &mut r,
            &[box_placement(&node, 2, 2, blocks + 3, blocks + 3)],
        );
        assert_eq!(shown_ids(&first).len(), CORNERS.len());
        for (x, y, width, height) in [
            (5, 6, blocks + 3, blocks + 3),
            (5, 6, blocks + 9, blocks + 4),
            (1, 1, 2 * blocks - 1, 2 * blocks - 1),
        ] {
            let next = rendered_placements(&mut r, &[box_placement(&node, x, y, width, height)]);
            assert!(shown_ids(&next).is_empty());
            assert!(deleted_ids(&next).is_empty());
            assert!(placed_ids(&next).len() <= CORNERS.len());
        }
    }

    fn paint_desired(r: &mut TerminalRenderer, placements: &[Placement]) -> Vec<Desired> {
        let mut desired = Vec::new();
        r.paint(&mut desired, placements, whole(r.window));
        desired
    }

    fn paint_desired_in(
        r: &mut TerminalRenderer,
        placements: &[Placement],
        area: Area,
    ) -> Vec<Desired> {
        let mut desired = Vec::new();
        r.paint(&mut desired, placements, area);
        desired
    }

    fn framed(r: &mut TerminalRenderer, state: &State) -> Vec<Desired> {
        r.paint_scene(&editor(state, whole(r.window)))
    }

    fn canvas_of(r: &mut TerminalRenderer, d: &Desired) -> Canvas {
        let content = Sprites::content(r, &d.image);
        let canvas = match content {
            Content::Still(c) => c,
            Content::Animation { root, .. } => root,
        };
        match &d.source {
            Some(src) => cropped(&canvas, src),
            None => canvas,
        }
    }

    fn cropped(canvas: &Canvas, src: &SourceRect) -> Canvas {
        let mut pixels = Vec::new();
        for y in src.y..src.y + src.height {
            let start = ((y * canvas.width + src.x) * 4) as usize;
            let end = ((y * canvas.width + src.x + src.width) * 4) as usize;
            pixels.extend_from_slice(&canvas.pixels[start..end]);
        }
        Canvas {
            pixels,
            width: src.width,
            height: src.height,
        }
    }

    fn covered_cells(r: &mut TerminalRenderer, d: &Desired) -> (i64, i64) {
        match d.cells {
            Some(cells) => cells,
            None => {
                let canvas = canvas_of(r, d);
                (
                    canvas.width / r.window.cell_width,
                    canvas.height / r.window.cell_height,
                )
            }
        }
    }

    fn stretched_canvas_of(r: &mut TerminalRenderer, d: &Desired) -> Canvas {
        let tile = canvas_of(r, d);
        let (cols, rows) = d.cells.unwrap_or((1, 1));
        let (width, height) = (tile.width * cols, tile.height * rows);
        if d.cells.is_none() {
            return tile;
        }
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let from = (((y % tile.height) * tile.width + x % tile.width) * 4) as usize;
                pixels.extend_from_slice(&tile.pixels[from..from + 4]);
            }
        }
        Canvas {
            pixels,
            width,
            height,
        }
    }

    fn glyph_cols(desired: &[Desired], row: i64) -> Vec<i64> {
        desired
            .iter()
            .filter(|d| d.row == row && matches!(d.image, ImageKey::Glyph(_)))
            .map(|d| d.col)
            .collect()
    }

    fn is_caret(d: &Desired) -> bool {
        matches!(d.image, ImageKey::Caret(_))
    }

    fn image_keys(desired: &[Desired]) -> std::collections::HashSet<ImageKey> {
        desired.iter().map(|d| d.image.clone()).collect()
    }

    struct Sprite {
        col: i64,
        row: i64,
        canvas: Canvas,
    }

    fn composed_desired(r: &mut TerminalRenderer, desired: &[Desired], z: i32) -> Sprite {
        let layer: Vec<usize> = desired
            .iter()
            .enumerate()
            .filter(|(_, d)| d.z == z)
            .map(|(i, _)| i)
            .collect();
        assert!(!layer.is_empty(), "no items at z={z}");
        let canvases: Vec<Canvas> = layer
            .iter()
            .map(|&i| stretched_canvas_of(r, &desired[i]))
            .collect();
        let col = layer.iter().map(|&i| desired[i].col).min().unwrap();
        let row = layer.iter().map(|&i| desired[i].row).min().unwrap();
        let cell_w = r.window.cell_width;
        let cell_h = r.window.cell_height;
        let width = layer
            .iter()
            .zip(&canvases)
            .map(|(&i, c)| (desired[i].col - col) * cell_w + c.width)
            .max()
            .unwrap();
        let height = layer
            .iter()
            .zip(&canvases)
            .map(|(&i, c)| (desired[i].row - row) * cell_h + c.height)
            .max()
            .unwrap();
        let channels = 4i64;
        let mut pixels = vec![0u8; (width * height * channels) as usize];
        for (&idx, canvas) in layer.iter().zip(&canvases) {
            let x = (desired[idx].col - col) * cell_w;
            let y = (desired[idx].row - row) * cell_h;
            for line in 0..canvas.height {
                for column in 0..canvas.width {
                    let from = ((line * canvas.width + column) * channels) as usize;
                    let to = (((y + line) * width + x + column) * channels) as usize;
                    if canvas.pixels[from + 3] != 0 {
                        pixels[to..to + 4].copy_from_slice(&canvas.pixels[from..from + 4]);
                    }
                }
            }
        }
        Sprite {
            col,
            row,
            canvas: Canvas {
                pixels,
                width,
                height,
            },
        }
    }

    fn window(cols: i64, rows: i64, cell_width: i64, cell_height: i64) -> Window {
        Window {
            cols,
            rows,
            cell_width,
            cell_height,
        }
    }

    fn renderer(cell_width: i64, cell_height: i64) -> TerminalRenderer {
        renderer_on(window(0, 0, cell_width, cell_height))
    }

    fn renderer_on(window: Window) -> TerminalRenderer {
        let source = Box::new(FakeGlyphSource::new(window.cell_width, window.cell_height));
        TerminalRenderer::new(window, source, CACHE_LIMIT)
    }

    fn label_placement(
        text: &str,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
    ) -> crate::view::Placement<'_> {
        bold_label_placement(text, x, y, width, height, false)
    }

    fn bold_label_placement(
        text: &str,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        bold: bool,
    ) -> crate::view::Placement<'_> {
        crate::view::Placement {
            node: crate::view::PlacementNode::Label(crate::view::Label {
                text: text.into(),
                colour: crate::style::rgb(None),
                bold,
            }),
            x,
            y,
            width,
            height,
            depth: 1,
        }
    }

    fn caret_placement(x: i64, y: i64, width: i64, height: i64) -> crate::view::Placement<'static> {
        crate::view::Placement {
            node: crate::view::PlacementNode::Caret(crate::view::Caret),
            x,
            y,
            width,
            height,
            depth: 0,
        }
    }

    fn typing_caret_placement(x: i64, y: i64) -> crate::view::Placement<'static> {
        crate::view::Placement {
            node: crate::view::PlacementNode::TypingCaret {
                colour: crate::style::rgb(None),
                bold: false,
            },
            x,
            y,
            width: 1,
            height: 1,
            depth: 1,
        }
    }

    #[test]
    fn on_resize_with_a_different_rounded_cell_height_does_not_panic_on_the_next_render() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let node = box_node(None, None, false);
        let placement = box_placement(&node, 0, 0, 4, 3);
        paint_desired(&mut r, std::slice::from_ref(&placement));
        r.on_resize(window(40, 20, 2, 5));
        paint_desired(&mut r, &[placement]);
    }

    #[test]
    fn on_resize_leaves_the_sprite_cache_untouched() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 0, 4, 3)],
        );
        assert_eq!(r.cache.len(), 1);
        r.on_resize(window(80, 40, 2, 4));
        assert_eq!(r.cache.len(), 1);
    }

    fn rendered_placements(r: &mut TerminalRenderer, placements: &[Placement<'static>]) -> String {
        let scene = vec![(whole(r.window), placements.to_vec())];
        let mut out = Vec::new();
        r.render(&scene, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn empty_state() -> State {
        crate::state::new_state(vec![], Mode::Command, None)
    }

    #[test]
    fn repeated_glyphs_transmit_once_and_place_the_image_under_distinct_placement_ids() {
        let placements = [label_placement("aa", 0, 0, 2, 1)];
        let mut r = renderer_on(window(3, 1, 1, 1));

        let output = rendered_placements(&mut r, &placements);

        let shown = shown_ids(&output);
        assert_eq!(shown.len(), 1);
        assert_eq!(placed_ids(&output), vec![shown[0].clone(); 2]);
        assert!(all_distinct(&placement_ids(&output)));
        assert!(output.find("a=t").unwrap() < output.find("a=p").unwrap());
    }

    fn command_fields(output: &str, action: &str, field: &str) -> Vec<String> {
        output
            .split("\x1b_G")
            .filter(|command| command.starts_with(action))
            .map(|command| {
                command
                    .split([',', ';'])
                    .find_map(|pair| pair.strip_prefix(field))
                    .expect("every command of this action names the field")
                    .to_string()
            })
            .collect()
    }

    fn shown_ids(output: &str) -> Vec<String> {
        command_fields(output, "a=t,", "i=")
    }

    fn placed_ids(output: &str) -> Vec<String> {
        command_fields(output, "a=p,", "i=")
    }

    fn deleted_ids(output: &str) -> Vec<String> {
        command_fields(output, "a=d,d=I,", "i=")
    }

    fn placement_ids(output: &str) -> Vec<String> {
        command_fields(output, "a=p,", "p=")
    }

    fn all_distinct(ids: &[String]) -> bool {
        ids.iter().collect::<std::collections::HashSet<_>>().len() == ids.len()
    }

    #[test]
    fn an_unchanged_frame_sends_no_commands() {
        let placements = [label_placement("a", 0, 0, 1, 1)];
        let mut r = renderer_on(window(3, 1, 1, 1));

        let first = rendered_placements(&mut r, &placements);
        let second = rendered_placements(&mut r, &placements);

        assert_eq!(first.matches("a=t").count(), 1);
        assert_eq!(first.matches("a=p").count(), 1);
        assert!(!second.contains("\x1b_G"));
    }

    #[test]
    fn glyph_character_colour_and_weight_each_have_distinct_images() {
        let placements = [
            label_placement("a", 0, 0, 1, 1),
            label_placement("b", 1, 0, 1, 1),
            crate::view::Placement {
                node: crate::view::PlacementNode::Label(crate::view::Label {
                    text: "a".into(),
                    colour: crate::style::rgb(Some(1)),
                    bold: false,
                }),
                x: 2,
                y: 0,
                width: 1,
                height: 1,
                depth: 1,
            },
            bold_label_placement("a", 3, 0, 1, 1, true),
        ];
        let mut r = renderer_on(window(4, 1, 1, 1));

        let output = rendered_placements(&mut r, &placements);

        let shown = shown_ids(&output);
        assert_eq!(shown.len(), placements.len());
        assert!(all_distinct(&shown));
        assert_eq!(placed_ids(&output).len(), placements.len());
        assert!(all_distinct(&placed_ids(&output)));
    }

    #[test]
    fn the_first_frame_hard_deletes_nothing() {
        let placements = [
            label_placement("a", 0, 0, 1, 1),
            caret_placement(1, 0, 1, 1),
        ];
        let mut r = renderer_on(window(3, 1, 1, 1));

        let first = rendered_placements(&mut r, &placements);

        assert!(!shown_ids(&first).is_empty());
        assert!(deleted_ids(&first).is_empty());
    }

    #[test]
    fn the_caret_is_drawn_last_as_a_solid_sprite() {
        let mut r = renderer_on(window(20, 10, 1, 1));
        let box_at = box_placement(&box_node(None, None, false), 0, 0, 4, 3);
        let label = label_placement("hi", 1, 1, 2, 1);
        let caret = caret_placement(label.x + label.width - 1, label.y, 1, 1);
        let desired = paint_desired(&mut r, &[box_at, label, caret]);
        let last = desired.last().expect("a sprite is drawn for the caret");
        let canvas = canvas_of(&mut r, last);
        let (cr, cg, cb) = colour(None);
        let solid = [cr, cg, cb, OPAQUE];
        assert!(canvas.pixels.chunks(4).all(|pixel| pixel == solid));
    }

    #[test]
    fn an_empty_drawing_draws_only_in_the_footer_row() {
        let window = window(40, 10, 1, 1);
        let mut r = renderer_on(window);
        let frame = framed(&mut r, &empty_state());
        assert!(frame
            .iter()
            .all(|image| image.row >= window.rows - FOOTER_ROWS));
    }

    #[test]
    fn the_footer_box_is_filled_with_the_foreground_colour_and_has_no_border() {
        let cell = 4;
        let mut r = renderer_on(window(40, 10, cell, cell));
        let frame = framed(&mut r, &empty_state());
        let footer = composed_desired(&mut r, &frame, depth_z(0));
        let (er, eg, eb, ea) = fill_colour(
            Some(crate::style::FOREGROUND),
            quantized_alpha(Some(FOOTER_FILL_OPACITY)),
        );
        for pixel in footer.canvas.pixels.chunks(4) {
            assert_eq!(pixel, [er, eg, eb, ea]);
        }
    }

    #[test]
    fn label_is_drawn_inside_the_box() {
        let node = box_node(None, None, false);
        let placements = vec![
            box_placement(&node, 0, 0, 5, 3),
            label_placement("hi", 1, 1, 2, 1),
        ];
        let mut r = renderer_on(window(5, 3, 1, 1));
        let desired = paint_desired(&mut r, &placements);
        assert_eq!(glyph_cols(&desired, 1), vec![1, 2]);
    }

    fn coloured_label_placement(
        text: &str,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        colour: Option<u8>,
    ) -> crate::view::Placement<'_> {
        crate::view::Placement {
            node: crate::view::PlacementNode::Label(crate::view::Label {
                text: text.into(),
                colour: crate::style::rgb(colour),
                bold: false,
            }),
            x,
            y,
            width,
            height,
            depth: 1,
        }
    }

    #[test]
    fn a_bold_label_renders_a_different_glyph_than_a_regular_label() {
        let mut r = renderer_on(window(2, 1, 1, 1));
        let regular = paint_desired(&mut r, &[bold_label_placement("h", 0, 0, 1, 1, false)]);
        let regular_pixels = canvas_of(&mut r, &regular[0]).pixels;

        let mut r = renderer_on(window(2, 1, 1, 1));
        let bold = paint_desired(&mut r, &[bold_label_placement("h", 0, 0, 1, 1, true)]);
        let bold_pixels = canvas_of(&mut r, &bold[0]).pixels;

        assert_ne!(regular_pixels, bold_pixels);
    }

    #[test]
    fn a_label_with_a_colour_renders_a_different_glyph_colour_than_the_default() {
        let mut r = renderer_on(window(2, 1, 1, 1));
        let default = paint_desired(&mut r, &[coloured_label_placement("h", 0, 0, 1, 1, None)]);
        let default_pixels = canvas_of(&mut r, &default[0]).pixels;

        let mut r = renderer_on(window(2, 1, 1, 1));
        let coloured = paint_desired(
            &mut r,
            &[coloured_label_placement(
                "h",
                0,
                0,
                1,
                1,
                Some(crate::style::LIME),
            )],
        );
        let coloured_pixels = canvas_of(&mut r, &coloured[0]).pixels;

        assert_ne!(default_pixels, coloured_pixels);
    }

    #[test]
    fn a_label_without_a_colour_matches_todays_default_foreground_rendering() {
        let mut r = renderer_on(window(2, 1, 1, 1));
        let desired = paint_desired(&mut r, &[label_placement("h", 0, 0, 1, 1)]);
        let pixels = canvas_of(&mut r, &desired[0]).pixels;

        assert!(pixels.iter().all(|&byte| byte == 0));
    }

    #[test]
    fn caret_is_drawn_after_the_label() {
        let node = box_node(None, None, false);
        let placements = vec![
            box_placement(&node, 0, 0, 5, 3),
            label_placement("hi", 1, 1, 2, 1),
            caret_placement(3, 1, 1, 1),
        ];
        let mut r = renderer_on(window(5, 3, 1, 1));
        let desired = paint_desired(&mut r, &placements);
        let caret_cols: Vec<i64> = desired
            .iter()
            .filter(|d| d.row == 1 && is_caret(d))
            .map(|d| d.col)
            .collect();
        assert_eq!(caret_cols, vec![3]);
        assert_eq!(glyph_cols(&desired, 1), vec![1, 2]);
        let caret_z = desired.iter().find(|d| is_caret(d)).unwrap().z;
        assert!(desired
            .iter()
            .filter(|d| matches!(d.image, ImageKey::Glyph(_)))
            .all(|d| d.z < caret_z));
    }

    #[test]
    fn label_and_caret_past_the_edge_are_clipped() {
        let node = box_node(None, None, false);
        let placements = vec![
            box_placement(&node, 0, 0, 5, 3),
            label_placement("hi", 1, 1, 2, 1),
            caret_placement(3, 1, 1, 1),
        ];
        let mut r = renderer_on(window(3, 3, 1, 1));
        let desired = paint_desired(&mut r, &placements);
        assert_eq!(glyph_cols(&desired, 1), vec![1, 2]);
        assert!(!desired.iter().any(is_caret));
    }

    #[test]
    fn a_box_does_not_draw_a_caret() {
        let mut r = renderer_on(window(5, 3, 1, 1));
        let desired = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 0, 5, 3)],
        );
        assert!(!desired.iter().any(is_caret));
    }

    #[test]
    fn caret_placement_is_drawn_at_its_own_position() {
        let mut r = renderer_on(window(4, 3, 1, 1));
        let images = paint_desired(&mut r, &[caret_placement(2, 1, 1, 1)]);
        assert_eq!(images.len(), 1);
        assert_eq!((images[0].col, images[0].row), (2, 1));
        let (cr, cg, cb) = colour(None);
        assert_eq!(
            &canvas_of(&mut r, &images[0]).pixels[0..4],
            &[cr, cg, cb, OPAQUE]
        );
    }

    #[test]
    fn a_caret_outside_the_grid_is_clipped() {
        let mut r = renderer_on(window(4, 3, 1, 1));
        let images = paint_desired(&mut r, &[caret_placement(9, 9, 1, 1)]);
        assert!(images.is_empty());
    }

    #[test]
    fn a_caret_has_a_sprite() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        assert_eq!(
            paint_desired(&mut r, &[caret_placement(1, 1, 1, 1)]).len(),
            1
        );
    }

    #[test]
    fn a_box_off_screen_has_no_sprite() {
        let mut r = renderer_on(window(5, 20, 4, 4));
        let images = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 10, 0, 4, 3)],
        );
        assert!(images.is_empty());
    }

    fn visible_source(images: &[Desired]) -> &SourceRect {
        images[0]
            .source
            .as_ref()
            .expect("a partly visible sprite names its visible pixels")
    }

    #[test]
    fn a_box_overhanging_the_left_is_cropped() {
        let mut r = renderer_on(window(40, 20, 4, 4));
        let images = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), -2, 1, 5, 3)],
        );
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].col, 0);
        let source = visible_source(&images);
        assert_eq!(source.x, 2 * r.window.cell_width);
        assert_eq!(source.width, 3 * r.window.cell_width);
        assert_eq!(source.height, 3 * r.window.cell_height);
    }

    #[test]
    fn a_box_overhanging_the_top_is_cropped() {
        let mut r = renderer_on(window(40, 20, 4, 4));
        let images = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 1, -2, 4, 5)],
        );
        assert_eq!(images[0].row, 0);
        let source = visible_source(&images);
        assert_eq!(source.y, 2 * r.window.cell_height);
        assert_eq!(source.height, 3 * r.window.cell_height);
    }

    #[test]
    fn a_box_overhanging_the_right_is_cropped() {
        let mut r = renderer_on(window(4, 20, 4, 4));
        let images = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 1, 0, 6, 3)],
        );
        assert_eq!(images[0].col, 1);
        let source = visible_source(&images);
        assert_eq!(source.x, 0);
        assert_eq!(source.width, 3 * r.window.cell_width);
    }

    #[test]
    fn a_box_overhanging_the_bottom_is_cropped() {
        let mut r = renderer_on(window(40, 4, 4, 4));
        let images = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 1, 3, 6)],
        );
        assert_eq!(images[0].row, 1);
        let source = visible_source(&images);
        assert_eq!(source.y, 0);
        assert_eq!(source.height, 3 * r.window.cell_height);
    }

    #[test]
    fn a_partly_visible_sprite_carries_a_source_rect() {
        let mut r = renderer_on(window(10, 10, 4, 6));
        let whole_box = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 2, 2, 4, 3)],
        );
        let clipped = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), -1, 2, 4, 3)],
        );
        assert!(whole_box[0].source.is_none());
        assert_eq!(whole_box[0].image, clipped[0].image);
        assert_eq!(clipped[0].cells, None);
        let source = visible_source(&clipped);
        assert_eq!(
            (source.x, source.y, source.width, source.height),
            (
                r.window.cell_width,
                0,
                3 * r.window.cell_width,
                3 * r.window.cell_height
            )
        );
        let content = canvas_of(&mut r, &whole_box[0]);
        assert_eq!(
            (content.width, content.height),
            (4 * r.window.cell_width, 3 * r.window.cell_height)
        );
    }

    #[test]
    fn a_box_sprite_sits_at_the_placement_cell() {
        let mut r = renderer_on(window(40, 20, 6, 12));
        let images = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 1, 2, 4, 3)],
        );
        let boxed = composed_desired(&mut r, &images, depth_z(0));
        assert_eq!((boxed.col, boxed.row), (1, 2));
        assert_eq!((boxed.canvas.width, boxed.canvas.height), (24, 36));
    }

    #[test]
    fn a_border_takes_the_colour_of_its_palette_index() {
        for index in (0..).take_while(|&i| palette(i).is_some()) {
            let mut r = renderer_on(window(40, 20, 2, 4));
            let images = paint_desired(
                &mut r,
                &[box_placement(
                    &box_node(Some(index), None, false),
                    0,
                    0,
                    2,
                    2,
                )],
            );
            let (px, py, pz) = colour(Some(index));
            assert_eq!(
                &canvas_of(&mut r, &images[0]).pixels[0..4],
                &[px, py, pz, OPAQUE]
            );
        }
    }

    #[test]
    fn an_unchanged_box_is_not_redrawn() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let node = box_node(Some(1), Some(1), false);
        let placement = box_placement(&node, 0, 0, 4, 3);
        let first = paint_desired(&mut r, std::slice::from_ref(&placement));
        assert_eq!(r.cache.len(), 1);
        let second = paint_desired(&mut r, &[placement]);
        assert!(first == second);
        assert_eq!(r.cache.len(), 1);
    }

    #[test]
    fn a_recoloured_box_is_redrawn() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let plain = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 0, 4, 3)],
        );
        let blue = paint_desired(
            &mut r,
            &[box_placement(&box_node(Some(4), None, false), 0, 0, 4, 3)],
        );
        assert_ne!(plain[0].image, blue[0].image);
        assert_ne!(
            canvas_of(&mut r, &plain[0]).pixels,
            canvas_of(&mut r, &blue[0]).pixels
        );
        assert_eq!(r.cache.len(), 2);
    }

    #[test]
    fn rounded_and_square_are_cached_distinctly() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        for rounded in [false, true] {
            paint_desired(
                &mut r,
                &[box_placement(&box_node(None, None, rounded), 0, 0, 4, 3)],
            );
        }
        assert_eq!(r.cache.len(), 2);
    }

    #[test]
    fn a_relabelled_box_of_the_same_size_reuses_its_pixels() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let first = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 0, 4, 3)],
        );
        let second = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 0, 4, 3)],
        );
        assert_eq!(first[0].image, second[0].image);
        assert_eq!(r.cache.len(), 1);
    }

    #[test]
    fn a_cached_sprite_moves_to_its_own_position() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 0, 4, 3)],
        );
        let moved = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 5, 2, 4, 3)],
        );
        assert_eq!((moved[0].col, moved[0].row), (5, 2));
    }

    #[test]
    fn a_moved_box_reuses_its_cached_pixels() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let first = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 0, 4, 3)],
        );
        let moved = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 5, 2, 4, 3)],
        );
        assert_eq!(first[0].image, moved[0].image);
        assert_eq!(
            canvas_of(&mut r, &first[0]).pixels,
            canvas_of(&mut r, &moved[0]).pixels
        );
        assert_eq!(r.cache.len(), 1);
    }

    #[test]
    fn a_differently_cropped_box_shares_one_cache_entry() {
        let mut r = renderer_on(window(4, 20, 2, 4));
        let whole = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 0, 4, 3)],
        );
        let cropped = paint_desired(
            &mut r,
            &[box_placement(&box_node(None, None, false), 2, 0, 4, 3)],
        );
        assert_eq!(whole[0].image, cropped[0].image);
        assert_ne!(whole[0].source, cropped[0].source);
        assert_eq!(r.cache.len(), 1);
    }

    #[test]
    fn a_shape_beyond_the_screen_leaves_the_cache_empty() {
        let mut r = renderer_on(window(4, 4, 2, 4));
        paint_desired(
            &mut r,
            &[
                box_placement(&box_node(None, None, false), 10, 0, 4, 3),
                arrow_placement(vec![0], 0, 0, 10, 4, 2),
            ],
        );
        assert!(r.cache.is_empty());
    }

    #[test]
    fn a_box_beyond_the_right_edge_is_not_shown() {
        let node = box_node(None, None, false);
        let mut r = renderer_on(window(20, 10, 4, 8));
        let desired = paint_desired(&mut r, &[box_placement(&node, 20, 0, 4, 3)]);
        assert!(desired.is_empty());
    }

    #[test]
    fn a_box_beyond_the_top_edge_is_not_shown() {
        let node = box_node(None, None, false);
        let mut r = renderer_on(window(20, 10, 4, 8));
        let desired = paint_desired(&mut r, &[box_placement(&node, 0, -3, 4, 3)]);
        assert!(desired.is_empty());
    }

    #[test]
    fn a_box_straddling_an_edge_is_shown() {
        let node = box_node(None, None, false);
        let mut r = renderer_on(window(20, 10, 4, 8));
        assert_eq!(
            paint_desired(&mut r, &[box_placement(&node, 18, 0, 4, 3)]).len(),
            1
        );
        assert_eq!(
            paint_desired(&mut r, &[box_placement(&node, -2, 8, 4, 3)]).len(),
            1
        );
    }

    #[test]
    fn a_block_poking_out_of_the_placement_is_cropped_to_the_screen() {
        let w = window(20, 10, 4, 8);
        let area = whole(w);
        let (col, row, source) = clip_natural(-1, -1, 2, 2, area, w).unwrap();
        assert_eq!((col, row), (0, 0));
        let source = source.unwrap();
        assert_eq!(
            (source.x, source.y, source.width, source.height),
            (w.cell_width, w.cell_height, w.cell_width, w.cell_height)
        );
        assert!(clip_natural(-2, 0, 2, 2, area, w).is_none());
    }

    #[test]
    fn a_block_beyond_the_area_is_clipped_to_the_area() {
        let w = window(20, 10, 4, 8);
        let area = Area {
            col: 2,
            row: 2,
            cols: 3,
            rows: 3,
        };
        let (col, row, source) = clip_natural(4, 0, 4, 4, area, w).unwrap();
        assert_eq!((col, row), (4, 2));
        let source = source.unwrap();
        assert_eq!(
            (source.x, source.y, source.width, source.height),
            (0, 2 * w.cell_height, w.cell_width, 2 * w.cell_height)
        );
    }

    #[test]
    fn arrows_with_different_stops_are_redrawn() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let one = paint_desired(&mut r, &[arrow_placement(vec![0], 0, 0, 0, 4, 6)]);
        let two = paint_desired(&mut r, &[arrow_placement(vec![0, 2], 0, 0, 0, 4, 6)]);
        assert_ne!(
            canvas_of(&mut r, &one[0]).pixels,
            canvas_of(&mut r, &two[0]).pixels
        );
    }

    #[test]
    fn the_cache_is_bounded() {
        let limit = 2;
        let source = Box::new(FakeGlyphSource::new(1, 1));
        let mut r = TerminalRenderer::new(window(20, 5, 1, 1), source, limit);
        for width in 0..(limit as i64 + 2) {
            paint_desired(
                &mut r,
                &[box_placement(
                    &box_node(None, None, false),
                    0,
                    0,
                    width + 1,
                    3,
                )],
            );
        }
        assert!(r.cache.len() <= limit);
    }

    fn box_outline(
        r: &TerminalRenderer,
        node: &PlacementNode<'static>,
        width: i64,
        height: i64,
    ) -> Canvas {
        let PlacementNode::Box {
            colour,
            fill,
            opacity,
            solid_fill,
            rounded,
            sides,
            border,
            gap,
            ..
        } = node
        else {
            panic!("expected a Box")
        };
        r.box_canvas(
            width,
            height,
            BoxStyle {
                colour: *colour,
                fill: *fill,
                fill_alpha: quantized_alpha(*opacity),
                solid_fill: *solid_fill,
                rounded: *rounded,
                sides: *sides,
                border: *border,
                gap: *gap,
            },
        )
    }

    fn pixel_of(sprite: &Canvas, x: i64, y: i64) -> (u8, u8, u8, u8) {
        pixel_at(&sprite.pixels, sprite.width, x, y)
    }

    fn overlaid(under: &Canvas, over: &Canvas, x: i64, y: i64) -> Canvas {
        let mut pixels = under.pixels.clone();
        for line in 0..over.height {
            for column in 0..over.width {
                let from = ((line * over.width + column) * 4) as usize;
                if over.pixels[from + 3] != 0 {
                    let to = (((y + line) * under.width + x + column) * 4) as usize;
                    pixels[to..to + 4].copy_from_slice(&over.pixels[from..from + 4]);
                }
            }
        }
        Canvas {
            pixels,
            width: under.width,
            height: under.height,
        }
    }

    fn ink_runs(canvas: &Canvas, at: i64, horizontal: bool) -> Vec<(i64, i64)> {
        let extent = if horizontal {
            canvas.width
        } else {
            canvas.height
        };
        let mut runs: Vec<(i64, i64)> = Vec::new();
        let mut open: Option<i64> = None;
        for offset in 0..extent {
            let (x, y) = if horizontal {
                (offset, at)
            } else {
                (at, offset)
            };
            if pixel_of(canvas, x, y).3 == 0 {
                if let Some(from) = open.take() {
                    runs.push((from, offset - 1));
                }
            } else if open.is_none() {
                open = Some(offset);
            }
        }
        if let Some(from) = open {
            runs.push((from, extent - 1));
        }
        runs
    }

    fn transparent_between(nearer: (i64, i64), further: (i64, i64)) -> i64 {
        further.0 - nearer.1 - 1
    }

    #[test]
    fn a_box_with_only_top_and_left_sides_draws_a_one_pixel_line_on_those_edges() {
        let r = renderer(1, 1);
        let size = 10 * BORDER;
        let node = with_border(
            &with_sides(&box_node(None, None, false), (true, false, false, true)),
            1,
        );
        let sprite = box_outline(&r, &node, size, size);
        let ink = edge_rgba(None);
        let last = size - 1;
        let middle = size / 2;
        assert_eq!(pixel_of(&sprite, middle, 0), ink);
        assert_eq!(pixel_of(&sprite, 0, middle), ink);
        assert_eq!(pixel_of(&sprite, middle, 1), TRANSPARENT);
        assert_eq!(pixel_of(&sprite, 1, middle), TRANSPARENT);
        assert_eq!(pixel_of(&sprite, middle, last), TRANSPARENT);
        assert_eq!(pixel_of(&sprite, last, middle), TRANSPARENT);
    }

    #[test]
    fn a_gapped_box_leaves_gap_pixels_of_transparency_between_its_stroke_and_the_box_border() {
        const CELL_W: i64 = 8;
        const CELL_H: i64 = 16;
        const BOX_CELLS_W: i64 = 10;
        const BOX_CELLS_H: i64 = 3;
        const OUTLINE_CELLS: i64 = 1;
        const OUTLINE_BORDER: i64 = 1;
        let r = renderer(CELL_W, CELL_H);
        let border = with_border(&box_node(Some(0), None, false), OUTLINE_BORDER);
        let outline = with_gap(
            &with_border(&box_node(Some(1), None, false), OUTLINE_BORDER),
            true,
        );
        let box_sprite = box_outline(&r, &border, BOX_CELLS_W, BOX_CELLS_H);
        let outline_sprite = box_outline(
            &r,
            &outline,
            BOX_CELLS_W + 2 * OUTLINE_CELLS,
            BOX_CELLS_H + 2 * OUTLINE_CELLS,
        );
        let canvas = overlaid(&outline_sprite, &box_sprite, CELL_W, CELL_H);
        let (middle_x, middle_y) = (canvas.width / 2, canvas.height / 2);
        let row = ink_runs(&canvas, middle_y, true);
        let column = ink_runs(&canvas, middle_x, false);
        assert_eq!(row.len(), 4);
        assert_eq!(column.len(), 4);
        for (runs, horizontal) in [(row, true), (column, false)] {
            assert!(runs[0].0 > 0);
            assert!(
                runs[3].1
                    < if horizontal {
                        canvas.width
                    } else {
                        canvas.height
                    } - 1
            );
            for (run, ink) in runs.iter().zip([
                edge_rgba(Some(1)),
                edge_rgba(Some(0)),
                edge_rgba(Some(0)),
                edge_rgba(Some(1)),
            ]) {
                assert_eq!(run.1 - run.0 + 1, OUTLINE_BORDER);
                let (x, y) = if horizontal {
                    (run.0, middle_y)
                } else {
                    (middle_x, run.0)
                };
                assert_eq!(pixel_of(&canvas, x, y), ink);
            }
            assert_eq!(transparent_between(runs[0], runs[1]), GAP_PX);
            assert_eq!(transparent_between(runs[2], runs[3]), GAP_PX);
        }
    }

    #[test]
    fn a_box_without_a_gap_draws_its_stroke_at_the_canvas_edge() {
        let r = renderer(1, 1);
        let cells = 10 * BORDER;
        let sprite = box_outline(&r, &box_node(None, None, false), cells, cells);
        let ink = edge_rgba(None);
        let (middle_x, middle_y) = (sprite.width / 2, sprite.height / 2);
        let (last_x, last_y) = (sprite.width - 1, sprite.height - 1);
        assert_eq!(pixel_of(&sprite, 0, middle_y), ink);
        assert_eq!(pixel_of(&sprite, last_x, middle_y), ink);
        assert_eq!(pixel_of(&sprite, middle_x, 0), ink);
        assert_eq!(pixel_of(&sprite, middle_x, last_y), ink);
    }

    #[test]
    fn plain_fill_renders_transparent_interior_via_outline_box() {
        let r = renderer(1, 1);
        let size = 2 * BORDER + 3;
        let sprite = box_outline(&r, &box_node(None, None, false), size, size);
        assert_eq!(pixel_of(&sprite, BORDER + 1, BORDER + 1), TRANSPARENT);
    }

    #[test]
    fn a_fill_colour_is_composited_over_black_via_outline_box() {
        let r = renderer(1, 1);
        let size = 2 * BORDER + 3;
        let sprite = box_outline(&r, &box_node(Some(2), Some(2), false), size, size);
        assert_eq!(
            pixel_of(&sprite, BORDER + 1, BORDER + 1),
            fill_colour(Some(2), quantized_alpha(Some(BOX_FILL_OPACITY)))
        );
    }

    #[test]
    fn a_rounded_box_cuts_away_its_extreme_corners_via_outline_box() {
        let r = renderer(8, 8);
        let sprite = box_outline(&r, &box_node(Some(1), Some(1), true), 10, 10);
        let (last_x, last_y) = (sprite.width - 1, sprite.height - 1);
        assert_eq!(pixel_of(&sprite, 0, 0), TRANSPARENT);
        assert_eq!(pixel_of(&sprite, last_x, 0), TRANSPARENT);
        assert_eq!(pixel_of(&sprite, 0, last_y), TRANSPARENT);
        assert_eq!(pixel_of(&sprite, last_x, last_y), TRANSPARENT);
    }

    #[test]
    fn the_radius_leaves_the_sprite_size_alone() {
        let r = renderer(8, 8);
        let square = box_outline(&r, &box_node(Some(1), Some(1), false), 10, 10);
        let rounded = box_outline(&r, &box_node(Some(1), Some(1), true), 10, 10);
        assert_eq!(
            (square.width, square.height),
            (rounded.width, rounded.height)
        );
    }

    fn arrow_alpha() -> u8 {
        (ARROW_OPACITY * OPAQUE as f64).round() as u8
    }

    fn arrow_outline(
        r: &TerminalRenderer,
        stops: Vec<i64>,
        shaft: i64,
        width: i64,
        height: i64,
    ) -> Canvas {
        r.arrow_canvas(width, height, &ArrowStyle { stops, shaft })
    }

    #[test]
    fn the_shaft_is_arrow_stroke_pixels_thick() {
        let r = renderer(10, 10);
        let ink = colour(None);
        let ink = (ink.0, ink.1, ink.2, arrow_alpha());
        let sprite = arrow_outline(&r, vec![0, 2], 1, 4, 3);
        let shaft_row = 10 + 5;
        let rows: Vec<i64> = centered_span(shaft_row, ARROW_STROKE).collect();
        for &y in &rows {
            assert_eq!(pixel_of(&sprite, 5, y), ink);
        }
        assert_eq!(pixel_of(&sprite, 5, rows[0] - 1), TRANSPARENT);
        assert_eq!(pixel_of(&sprite, 5, rows[rows.len() - 1] + 1), TRANSPARENT);
    }

    #[test]
    fn the_trunk_is_arrow_stroke_pixels_thick() {
        let r = renderer(10, 10);
        let ink = colour(None);
        let ink = (ink.0, ink.1, ink.2, arrow_alpha());
        let sprite = arrow_outline(&r, vec![0, 2], 1, 4, 3);
        let midpoint = (4 * 10) / 2;
        let columns: Vec<i64> = centered_span(midpoint, ARROW_STROKE).collect();
        for &x in &columns {
            assert_eq!(pixel_of(&sprite, x, 10), ink);
        }
        assert_eq!(pixel_of(&sprite, columns[0] - 1, 10), TRANSPARENT);
        assert_eq!(
            pixel_of(&sprite, columns[columns.len() - 1] + 1, 10),
            TRANSPARENT
        );
    }

    #[test]
    fn the_arrowhead_tip_sits_at_the_stop_row() {
        let r = renderer(10, 10);
        let ink = colour(None);
        let ink = (ink.0, ink.1, ink.2, arrow_alpha());
        let sprite = arrow_outline(&r, vec![0, 2], 1, 4, 3);
        let right_edge = 4 * 10 - 1;
        for stop_row in [5, 25] {
            assert_eq!(pixel_of(&sprite, right_edge, stop_row), ink);
            assert_eq!(pixel_of(&sprite, right_edge - 1, stop_row), ink);
        }
    }

    #[test]
    fn a_single_stop_arrow_is_a_straight_line_across_every_column() {
        let r = renderer(4, 5);
        let sprite = arrow_outline(&r, vec![0], 0, 2, 1);
        let shaft_row = sprite.height / 2;
        for x in 0..sprite.width {
            assert_eq!(pixel_of(&sprite, x, shaft_row).3, arrow_alpha());
        }
    }

    #[test]
    fn arrow_off_shape_pixels_are_transparent() {
        let r = renderer(4, 5);
        let sprite = arrow_outline(&r, vec![0], 0, 2, 1);
        assert_eq!(pixel_of(&sprite, 0, 0), TRANSPARENT);
    }

    #[test]
    fn an_arrow_is_the_default_foreground_colour() {
        let r = renderer(4, 5);
        let sprite = arrow_outline(&r, vec![0], 0, 2, 1);
        let (pr, pg, pb) = colour(None);
        let shaft_row = sprite.height / 2;
        assert_eq!(pixel_of(&sprite, 0, shaft_row), (pr, pg, pb, arrow_alpha()));
    }

    #[test]
    fn a_branching_arrow_has_a_stub_at_every_stop() {
        let r = renderer(4, 5);
        let sprite = arrow_outline(&r, vec![0, 3], 0, 2, 4);
        let midpoint = sprite.width / 2;
        for stop in [0, 3] {
            let row = stop * 5 + 5 / 2;
            for x in midpoint..sprite.width {
                assert_eq!(pixel_of(&sprite, x, row).3, arrow_alpha());
            }
        }
    }

    #[test]
    fn a_branching_arrow_rows_between_stops_are_blank_past_the_trunk() {
        let r = renderer(4, 5);
        let sprite = arrow_outline(&r, vec![0, 3], 0, 2, 4);
        let midpoint = sprite.width / 2;
        let trunk_columns: std::collections::HashSet<i64> =
            centered_span(midpoint, ARROW_STROKE).collect();
        let row_between_stops = 5 + 5 / 2;
        for x in 0..sprite.width {
            let expected = if trunk_columns.contains(&x) {
                arrow_alpha()
            } else {
                0
            };
            assert_eq!(pixel_of(&sprite, x, row_between_stops).3, expected);
        }
    }

    #[test]
    fn an_arrow_pixel_has_the_arrow_opacity_and_the_foreground_colour() {
        let r = renderer(4, 5);
        let sprite = arrow_outline(&r, vec![0], 0, 2, 1);
        let (fr, fg, fb) = colour(None);
        let shaft_row = sprite.height / 2;
        assert_eq!(pixel_of(&sprite, 0, shaft_row), (fr, fg, fb, arrow_alpha()));
    }

    #[test]
    fn the_shaft_meeting_the_trunk_has_the_same_colour_as_the_shaft_alone() {
        let r = renderer(10, 10);
        let sprite = arrow_outline(&r, vec![0, 2], 1, 4, 3);
        let shaft_row = 10 + 5;
        let midpoint = (4 * 10) / 2;
        assert_eq!(
            pixel_of(&sprite, midpoint, shaft_row),
            pixel_of(&sprite, midpoint - ARROW_STROKE - 1, shaft_row)
        );
    }

    const _: () = {
        assert!(ARROW_STROKE < BORDER);
        assert!(ARROW_STROKE > 1);
    };

    fn one_leaf() -> State {
        crate::state::new_state(vec![crate::diagram::node("A")], Mode::Command, None)
    }

    fn leaf_box(state: &State) -> Placement<'_> {
        crate::layout::tree::diagram(state.doc().tree(), None, None)
            .into_iter()
            .find(|placement| matches!(placement.node, PlacementNode::Box { .. }))
            .unwrap()
    }

    #[test]
    fn the_diagram_is_centred_in_the_rows_above_the_last() {
        let (cols, rows) = (20, 12);
        let mut r = renderer_on(window(cols, rows, 1, 1));
        let state = one_leaf();
        let leaf = leaf_box(&state);
        let frame = framed(&mut r, &state);
        let drawn = &frame[0];
        assert_eq!(
            (drawn.col, drawn.row),
            (
                (cols - leaf.width).div_euclid(2),
                (rows - FOOTER_ROWS - leaf.height).div_euclid(2)
            )
        );
    }

    #[test]
    fn a_diagram_box_overhanging_the_body_does_not_draw_into_the_last_row() {
        let state = one_leaf();
        let leaf = leaf_box(&state);
        let body_rows = leaf.height - 1;
        let window = window(20, body_rows + FOOTER_ROWS, 1, 1);
        let mut r = renderer_on(window);
        let frame = framed(&mut r, &state);
        let body_images: Vec<&Desired> = frame.iter().filter(|d| d.row < body_rows).collect();
        assert!(!body_images.is_empty());
        for image in body_images {
            let (_, rows) = covered_cells(&mut r, image);
            assert!(image.row + rows <= body_rows);
        }
    }

    const EXTRA_CELLS: i64 = 3;

    fn tiled_window() -> Window {
        window(60, 30, CELL_WIDTH, CELL_HEIGHT)
    }

    fn tile_shape(r: &TerminalRenderer, node: &PlacementNode<'static>) -> TileShape {
        let style = match node {
            PlacementNode::Box {
                colour,
                fill,
                opacity,
                solid_fill,
                rounded,
                sides,
                border,
                gap,
                ..
            } => BoxStyle {
                colour: *colour,
                fill: *fill,
                fill_alpha: quantized_alpha(*opacity),
                solid_fill: *solid_fill,
                rounded: *rounded,
                sides: *sides,
                border: *border,
                gap: *gap,
            },
            _ => panic!("expected a box or brackets"),
        };
        TileShape {
            style,
            cell: r.cell_size(),
        }
    }

    fn smallest_tiled(r: &TerminalRenderer, node: &PlacementNode<'static>) -> (i64, i64) {
        let (column_band, row_band) = tile_shape(r, node).bands();
        (cells_with_middle(column_band), cells_with_middle(row_band))
    }

    fn brackets_node() -> PlacementNode<'static> {
        PlacementNode::Brackets { border: BORDER }
    }

    fn outlined_node(colour: Option<u8>) -> PlacementNode<'static> {
        box_node(colour, colour, true)
    }

    fn smallest_tiled_selected_box(r: &TerminalRenderer, colour: Option<u8>) -> (i64, i64) {
        smallest_tiled(r, &outlined_node(colour))
    }

    fn selected_box(
        colour: Option<u8>,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
    ) -> [Placement<'static>; 2] {
        [
            box_placement(&outlined_node(colour), x, y, width, height),
            box_placement(
                &brackets_node(),
                x - BRACKET_MARGIN,
                y - BRACKET_MARGIN,
                width + 2 * BRACKET_MARGIN,
                height + 2 * BRACKET_MARGIN,
            ),
        ]
    }

    fn cells_of(placement: &Placement) -> Vec<(i64, i64)> {
        let geometry = Geometry::from(placement);
        (geometry.y..geometry.y + geometry.height)
            .flat_map(|row| (geometry.x..geometry.x + geometry.width).map(move |col| (col, row)))
            .collect()
    }

    fn covered_positions(desired: &[Desired]) -> Vec<(i64, i64)> {
        let mut positions: Vec<(i64, i64)> = desired
            .iter()
            .flat_map(|d| {
                let (cols, rows) = d.cells.unwrap_or((1, 1));
                (d.row..d.row + rows)
                    .flat_map(move |row| (d.col..d.col + cols).map(move |col| (col, row)))
            })
            .collect();
        positions.sort();
        positions
    }

    fn commit_ops(
        vt: &mut super::super::virtual_terminal::VirtualTerminal,
        r: &mut TerminalRenderer,
        placements: &[Placement],
    ) -> Vec<super::super::virtual_terminal::Op> {
        let desired = paint_desired(r, placements);
        vt.commit(&desired, r)
    }

    fn tile_images(desired: &[Desired]) -> std::collections::HashSet<ImageKey> {
        image_keys(desired)
            .into_iter()
            .filter(|key| matches!(key, ImageKey::Tile(_)))
            .collect()
    }

    fn middle_cells(
        r: &TerminalRenderer,
        node: &PlacementNode<'static>,
        width: i64,
        height: i64,
    ) -> (i64, i64) {
        let (column_band, row_band) = tile_shape(r, node).bands();
        (width - 2 * column_band, height - 2 * row_band)
    }

    #[test]
    fn widening_a_tiled_box_keeps_the_same_placements_and_images() {
        let mut r = renderer_on(tiled_window());
        let colour = Some(1);
        let (width, height) = smallest_tiled_selected_box(&r, colour);
        let narrow = selected_box(colour, BRACKET_MARGIN, BRACKET_MARGIN, width, height);
        let wide = selected_box(colour, BRACKET_MARGIN, BRACKET_MARGIN, width + 1, height);

        let first = paint_desired(&mut r, &narrow);
        let second = paint_desired(&mut r, &wide);

        assert_eq!(first.len(), second.len());
        assert_eq!(image_keys(&first), image_keys(&second));
    }

    #[test]
    fn widening_a_tiled_box_uploads_nothing_and_deletes_nothing() {
        let mut r = renderer_on(tiled_window());
        let mut vt = super::super::virtual_terminal::VirtualTerminal::new();
        let colour = Some(1);
        let (width, height) = smallest_tiled_selected_box(&r, colour);
        let narrow = selected_box(colour, BRACKET_MARGIN, BRACKET_MARGIN, width, height);
        let wide = selected_box(colour, BRACKET_MARGIN, BRACKET_MARGIN, width + 1, height);

        commit_ops(&mut vt, &mut r, &narrow);
        let ops = commit_ops(&mut vt, &mut r, &wide);

        assert!(!ops.is_empty());
        assert!(ops
            .iter()
            .all(|op| matches!(op, super::super::virtual_terminal::Op::Place { .. })));
    }

    #[test]
    fn tiled_boxes_of_one_style_and_different_sizes_share_every_tile_image() {
        let mut r = renderer_on(tiled_window());
        let colour = Some(1);
        let (width, height) = smallest_tiled_selected_box(&r, colour);

        let small = paint_desired(
            &mut r,
            &selected_box(colour, BRACKET_MARGIN, BRACKET_MARGIN, width, height),
        );
        let large = paint_desired(
            &mut r,
            &selected_box(
                colour,
                BRACKET_MARGIN,
                BRACKET_MARGIN,
                width + EXTRA_CELLS,
                height + 1,
            ),
        );

        assert_eq!(image_keys(&small), image_keys(&large));
    }

    #[test]
    fn tiled_boxes_of_different_styles_share_no_tile_image() {
        let mut r = renderer_on(tiled_window());
        let (width, height) = smallest_tiled_selected_box(&r, Some(1));

        let one = paint_desired(
            &mut r,
            &selected_box(Some(1), BRACKET_MARGIN, BRACKET_MARGIN, width, height)[..1],
        );
        let other = paint_desired(
            &mut r,
            &selected_box(Some(2), BRACKET_MARGIN, BRACKET_MARGIN, width, height)[..1],
        );

        assert!(image_keys(&one).is_disjoint(&image_keys(&other)));
    }

    #[test]
    fn a_partly_clipped_tiled_box_shows_only_the_cells_inside_the_area() {
        let mut r = renderer_on(tiled_window());
        let node = outlined_node(Some(1));
        let (width, height) = smallest_tiled(&r, &node);
        let area = Area {
            col: 2,
            row: 2,
            cols: width,
            rows: height,
        };
        let placement = box_placement(&node, area.col - 1, area.row + 1, width, height);

        let desired = paint_desired_in(&mut r, std::slice::from_ref(&placement), area);

        let inside = |&(col, row): &(i64, i64)| {
            (area.col..area.col + area.cols).contains(&col)
                && (area.row..area.row + area.rows).contains(&row)
        };
        let mut expected: Vec<(i64, i64)> =
            cells_of(&placement).into_iter().filter(inside).collect();
        expected.sort();
        assert_eq!(covered_positions(&desired), expected);
        for d in &desired {
            let content = canvas_of(&mut r, d);
            assert_eq!(
                (content.width, content.height),
                (r.window.cell_width, r.window.cell_height)
            );
        }
    }

    #[test]
    fn tiles_keep_their_sprites_z_and_are_queued_in_scene_order() {
        let mut r = renderer_on(tiled_window());
        let colour = Some(1);
        let (width, height) = smallest_tiled_selected_box(&r, colour);
        let [boxed, brackets] = selected_box(colour, BRACKET_MARGIN, BRACKET_MARGIN, width, height);

        let images = paint_desired(&mut r, &[boxed.clone(), brackets.clone()]);

        let queued: Vec<(i64, i64, i32)> = images
            .iter()
            .map(|image| (image.col, image.row, image.z))
            .collect();
        let expected: Vec<(i64, i64, i32)> = tile_shape(&r, &boxed.node)
            .tiles(boxed.width, boxed.height)
            .unwrap()
            .into_iter()
            .map(|(col, row, ..)| (boxed.x + col, boxed.y + row, depth_z(0)))
            .chain(CORNERS.into_iter().map(|corner| {
                let (col, row) =
                    corner_offset(corner, brackets.width, brackets.height, r.cell_size());
                (brackets.x + col, brackets.y + row, BRACKETS_Z)
            }))
            .collect();
        assert_eq!(queued, expected);
    }

    #[test]
    fn a_box_too_small_to_tile_is_one_whole_sprite_uploaded_once() {
        let mut r = renderer_on(tiled_window());
        let mut vt = super::super::virtual_terminal::VirtualTerminal::new();
        let node = outlined_node(Some(1));
        let (width, height) = smallest_tiled(&r, &node);
        let placements = [box_placement(&node, 0, 0, width - 1, height)];

        let images = paint_desired(&mut r, &placements);
        let first = commit_ops(&mut vt, &mut r, &placements);
        let second = commit_ops(&mut vt, &mut r, &placements);

        let [image] = images.as_slice() else {
            panic!("expected a single whole sprite");
        };
        assert!(matches!(image.image, ImageKey::Sprite(_)));
        let content = canvas_of(&mut r, image);
        assert_eq!(
            (content.width, content.height),
            (
                (width - 1) * r.window.cell_width,
                height * r.window.cell_height
            )
        );
        assert_eq!(
            first
                .iter()
                .filter(|op| matches!(op, super::super::virtual_terminal::Op::Upload { .. }))
                .count(),
            1
        );
        assert!(second.is_empty());
    }

    #[test]
    fn resizing_the_cell_size_produces_new_tile_keys() {
        let mut r = renderer_on(tiled_window());
        let colour = Some(1);
        let (old_cell_width, old_cell_height) = (r.window.cell_width, r.window.cell_height);
        let (old_width, old_height) = smallest_tiled_selected_box(&r, colour);
        r.window.cell_width = 2 * old_cell_width;
        r.window.cell_height = 2 * old_cell_height;
        let (new_width, new_height) = smallest_tiled_selected_box(&r, colour);
        r.window.cell_width = old_cell_width;
        r.window.cell_height = old_cell_height;
        let placements = selected_box(
            colour,
            BRACKET_MARGIN,
            BRACKET_MARGIN,
            old_width.max(new_width),
            old_height.max(new_height),
        );

        let before = paint_desired(&mut r, &placements);
        r.window.cell_width = 2 * old_cell_width;
        r.window.cell_height = 2 * old_cell_height;
        let after = paint_desired(&mut r, &placements);

        assert!(!after.is_empty());
        assert!(image_keys(&before).is_disjoint(&image_keys(&after)));
        assert!(tile_images(&after)
            .iter()
            .all(|key| matches!(key, ImageKey::Tile(tile) if tile.shape.cell == r.cell_size())));
    }

    #[test]
    fn a_tileable_box_stretches_its_middle_band() {
        let mut r = renderer_on(tiled_window());
        let node = outlined_node(Some(1));
        let (width, height) = smallest_tiled(&r, &node);
        let (width, height) = (width + EXTRA_CELLS, height + EXTRA_CELLS);
        let (middle_cols, middle_rows) = middle_cells(&r, &node, width, height);

        let desired = paint_desired(&mut r, &[box_placement(&node, 0, 0, width, height)]);

        assert!(desired.len() < (width * height) as usize);
        assert!(desired.iter().any(|d| d.cells == Some((middle_cols, 1))));
        assert!(desired.iter().any(|d| d.cells == Some((1, middle_rows))));
        assert_eq!(covered_positions(&desired).len(), (width * height) as usize);
    }

    #[test]
    fn an_unfilled_box_omits_its_middle() {
        let mut r = renderer_on(tiled_window());
        let filled = outlined_node(Some(1));
        let unfilled = box_node(Some(1), None, true);
        let (width, height) = smallest_tiled(&r, &filled);
        let (width, height) = (width + EXTRA_CELLS, height + EXTRA_CELLS);
        let (middle_cols, middle_rows) = middle_cells(&r, &filled, width, height);

        let with_fill = paint_desired(&mut r, &[box_placement(&filled, 0, 0, width, height)]);
        let without_fill = paint_desired(&mut r, &[box_placement(&unfilled, 0, 0, width, height)]);

        let middle = |d: &&Desired| d.cells == Some((middle_cols, middle_rows));
        assert_eq!(with_fill.iter().filter(middle).count(), 1);
        assert_eq!(without_fill.iter().filter(middle).count(), 0);
        assert_eq!(with_fill.len(), without_fill.len() + 1);
    }

    #[test]
    fn a_flex_box_is_eight_placements_whatever_its_size() {
        const THIN_BORDER: i64 = 1;
        let mut r = renderer_on(tiled_window());
        let node = with_border(&box_node(Some(1), None, false), THIN_BORDER);
        let (column_band, row_band) = tile_shape(&r, &node).bands();
        assert_eq!((column_band, row_band), (1, 1));
        let (width, height) = smallest_tiled(&r, &node);

        let counts: Vec<usize> = [(0, 0), (3, 2), (10, 7)]
            .into_iter()
            .map(|(extra_cols, extra_rows)| {
                paint_desired(
                    &mut r,
                    &[box_placement(
                        &node,
                        0,
                        0,
                        width + extra_cols,
                        height + extra_rows,
                    )],
                )
                .len()
            })
            .collect();

        assert!(counts.iter().all(|&count| count == counts[0]));
        assert_eq!(counts[0], 2 * (column_band + row_band) as usize + 4);
    }

    #[test]
    fn a_partly_visible_run_is_shortened() {
        let mut r = renderer_on(tiled_window());
        let node = outlined_node(Some(1));
        let (width, height) = smallest_tiled(&r, &node);
        let (width, height) = (width + EXTRA_CELLS, height + 1);
        let (column_band, _) = tile_shape(&r, &node).bands();
        let overhang = column_band + 1;
        let top_run = |desired: &[Desired]| {
            desired
                .iter()
                .find(|d| d.row == 0 && matches!(d.cells, Some((_, 1))))
                .cloned()
                .unwrap()
        };

        let whole_box = paint_desired(&mut r, &[box_placement(&node, 0, 0, width, height)]);
        let clipped_x = r.window.cols - (width - overhang);
        let clipped = paint_desired(&mut r, &[box_placement(&node, clipped_x, 0, width, height)]);

        let (whole_run, clipped_run) = (top_run(&whole_box), top_run(&clipped));
        assert_eq!(whole_run.image, clipped_run.image);
        assert_eq!(clipped_run.source, None);
        let visible = r.window.cols - clipped_run.col;
        assert_eq!(clipped_run.cells.unwrap().0, visible);
        assert!(visible < whole_run.cells.unwrap().0);
    }

    #[test]
    fn the_caret_is_keyed_by_its_size() {
        let mut r = renderer_on(window(20, 10, 4, 6));
        let key_at = |r: &mut TerminalRenderer, x, width, height| {
            paint_desired(r, &[caret_placement(x, 1, width, height)])[0]
                .image
                .clone()
        };

        let one = key_at(&mut r, 2, 1, 1);
        let moved = key_at(&mut r, 5, 1, 1);
        let wide = key_at(&mut r, 2, 3, 1);

        assert_eq!(one, moved);
        assert_ne!(one, wide);
        let Content::Still(canvas) = Sprites::content(&mut r, &wide) else {
            panic!("a caret is a still image");
        };
        assert_eq!(
            (canvas.width, canvas.height),
            (3 * r.window.cell_width, r.window.cell_height)
        );
    }

    #[test]
    fn eased_starts_at_zero_and_ends_at_one() {
        assert_eq!(eased(0.0), 0.0);
        assert_eq!(eased(1.0), 1.0);
    }

    #[test]
    fn eased_is_an_ease_out_cubic() {
        assert_eq!(eased(0.5), 0.875);
    }

    #[test]
    fn eased_never_decreases() {
        let samples: Vec<f64> = (0..=1000).map(|i| eased(i as f64 / 1000.0)).collect();
        assert!(samples.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    fn grow_style() -> BoxStyle {
        BoxStyle {
            colour: colour(None),
            fill: None,
            fill_alpha: None,
            solid_fill: Some((12, 34, 56)),
            rounded: false,
            sides: ALL_SIDES,
            border: BORDER,
            gap: false,
        }
    }

    fn drawn_size(canvas: &Canvas) -> (i64, i64) {
        let mut size = (0, 0);
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                if canvas.pixels[((y * canvas.width + x) * 4 + 3) as usize] != 0 {
                    size = (size.0.max(x + 1), size.1.max(y + 1));
                }
            }
        }
        size
    }

    #[test]
    fn grow_frames_are_all_full_size() {
        let frames = grow_frames(90, 60, grow_style());
        assert_eq!(frames.len(), GROW_FRAMES);
        for frame in &frames {
            assert_eq!((frame.width, frame.height), (90, 60));
            assert_eq!(frame.pixels.len(), 90 * 60 * 4);
        }
    }

    #[test]
    fn grow_frames_drawn_size_never_decreases() {
        let sizes: Vec<(i64, i64)> = grow_frames(90, 60, grow_style())
            .iter()
            .map(drawn_size)
            .collect();
        assert!(sizes
            .windows(2)
            .all(|pair| pair[0].0 <= pair[1].0 && pair[0].1 <= pair[1].1));
        assert!(sizes[0] < sizes[GROW_FRAMES - 1]);
    }

    #[test]
    fn grow_frames_last_frame_is_the_full_box() {
        let style = grow_style();
        let frames = grow_frames(90, 60, style);
        let full = Canvas::fill(90, 60, &box_shape(90, 60, style));
        assert_eq!(frames[GROW_FRAMES - 1].pixels, full.pixels);
    }

    #[test]
    fn grow_frames_are_transparent_outside_each_eased_box() {
        let (width, height) = (90, 60);
        for (index, frame) in grow_frames(width, height, grow_style()).iter().enumerate() {
            let e = eased((index + 1) as f64 / GROW_FRAMES as f64);
            let box_width = ((width as f64 * e).round() as i64).clamp(1, width);
            let box_height = ((height as f64 * e).round() as i64).clamp(1, height);
            assert_eq!(drawn_size(frame), (box_width, box_height));
            for y in 0..height {
                for x in 0..width {
                    if x >= box_width || y >= box_height {
                        assert_eq!(frame.pixels[((y * width + x) * 4 + 3) as usize], 0);
                    }
                }
            }
        }
    }

    #[test]
    fn grow_root_is_the_full_box_with_the_stamp_in_the_lowest_colour_bits() {
        let full = Canvas::fill(90, 60, &box_shape(90, 60, grow_style()));
        let stamp = 0xABCD_1234_5678_9EF0;

        let root = grow_root(&full, stamp);

        assert_eq!((root.width, root.height), (90, 60));
        assert_eq!(root.pixels.len(), full.pixels.len());
        let mut bits = 0u64;
        let mut colour_byte = 0;
        for (index, (root_byte, full_byte)) in root.pixels.iter().zip(&full.pixels).enumerate() {
            if index % 4 == 3 {
                assert_eq!(root_byte, full_byte);
                continue;
            }
            assert_eq!(root_byte & !1, full_byte & !1);
            if colour_byte < 48 {
                bits |= u64::from(root_byte & 1) << colour_byte;
                colour_byte += 1;
            }
        }
        assert_eq!(bits, stamp & 0xFFFF_FFFF_FFFF);
    }

    #[test]
    fn grow_root_of_a_canvas_smaller_than_the_stamp_keeps_its_size() {
        let full = Canvas::fill(2, 2, &box_shape(2, 2, grow_style()));

        let root = grow_root(&full, u64::MAX);

        assert_eq!(root.pixels.len(), full.pixels.len());
    }

    fn growing_node() -> PlacementNode<'static> {
        match box_node(Some(1), Some(1), true) {
            PlacementNode::Box {
                colour,
                fill,
                opacity,
                solid_fill,
                rounded,
                sides,
                border,
                gap,
                ..
            } => PlacementNode::Box {
                colour,
                fill,
                opacity,
                solid_fill,
                rounded,
                sides,
                border,
                grow: true,
                gap,
            },
            _ => panic!("expected a Box"),
        }
    }

    fn growing_box(r: &TerminalRenderer, x: i64, y: i64) -> [Placement<'static>; 1] {
        let node = growing_node();
        let (width, height) = smallest_tiled(r, &node);
        [box_placement(&node, x, y, width + EXTRA_CELLS, height + 1)]
    }

    fn root_payload(output: &str) -> String {
        output
            .split("\x1b_G")
            .skip_while(|command| !command.starts_with("a=t,"))
            .enumerate()
            .take_while(|(index, command)| *index == 0 || command.starts_with("m="))
            .map(|(_, command)| command)
            .map(|command| {
                let (_, rest) = command.split_once(';').unwrap();
                rest.split("\x1b\\").next().unwrap().to_string()
            })
            .collect()
    }

    #[test]
    fn a_growing_box_that_fits_is_one_image_with_every_grow_frame_and_no_tiles() {
        let mut r = renderer_on(tiled_window());
        let mut vt = super::super::virtual_terminal::VirtualTerminal::new();
        let placements = growing_box(&r, 1, 1);

        let desired = paint_desired(&mut r, &placements);
        let ops = commit_ops(&mut vt, &mut r, &placements);

        assert_eq!(desired.len(), 1);
        assert!(matches!(desired[0].image, ImageKey::Grow(_)));
        let animations: Vec<usize> = ops
            .iter()
            .filter_map(|op| match op {
                super::super::virtual_terminal::Op::Upload {
                    content: Content::Animation { frames, .. },
                    ..
                } => Some(frames.len()),
                _ => None,
            })
            .collect();
        assert_eq!(animations, vec![GROW_FRAMES]);
        assert!(r.tile_canvases.is_empty());
    }

    fn grow_frame_keys(output: &str) -> Vec<&str> {
        output
            .split("\x1b_G")
            .filter(|command| command.starts_with("a=f,"))
            .map(|command| command.split_once(';').unwrap().0)
            .collect()
    }

    #[test]
    fn a_growing_box_outside_wezterm_sends_frames_without_capital_z() {
        let mut r = renderer_on(tiled_window());
        let placements = growing_box(&r, 1, 1);

        let output = rendered_placements(&mut r, &placements);

        let keys = grow_frame_keys(&output);
        assert_eq!(keys.len(), GROW_FRAMES);
        assert!(keys
            .iter()
            .all(|keys| !keys.split(',').any(|pair| pair.starts_with("Z="))));
    }

    #[test]
    fn a_growing_box_in_wezterm_sends_frames_with_capital_z() {
        let mut r = renderer_on(tiled_window());
        r.wezterm = true;
        let placements = growing_box(&r, 1, 1);

        let output = rendered_placements(&mut r, &placements);

        let keys = grow_frame_keys(&output);
        assert_eq!(keys.len(), GROW_FRAMES);
        assert!(keys
            .iter()
            .all(|keys| keys.split(',').any(|pair| pair.starts_with("Z="))));
    }

    #[test]
    fn a_cropped_growing_box_pops_in_as_tiles() {
        let mut r = renderer_on(tiled_window());
        let [placement] = growing_box(&r, 0, 0);
        let cropped = Placement {
            x: tiled_window().cols - placement.width + 1,
            ..placement
        };

        let desired = paint_desired(&mut r, &[cropped]);

        assert!(!desired.is_empty());
        assert!(desired.iter().all(|d| matches!(d.image, ImageKey::Tile(_))));
    }

    #[test]
    fn two_grows_of_the_same_size_have_different_roots() {
        let mut r = renderer_on(tiled_window());
        let placements = growing_box(&r, 1, 1);

        let first = rendered_placements(&mut r, &placements);
        let second = rendered_placements(&mut r, &placements);

        assert!(!root_payload(&first).is_empty());
        assert_ne!(root_payload(&first), root_payload(&second));
    }

    #[test]
    fn the_frame_after_a_grow_frees_it_and_draws_the_box_with_tiles() {
        let mut r = renderer_on(tiled_window());
        let mut vt = super::super::virtual_terminal::VirtualTerminal::new();
        let [growing] = growing_box(&r, 1, 1);
        let settled = Placement {
            node: box_node(Some(1), Some(1), true),
            ..growing.clone()
        };

        commit_ops(&mut vt, &mut r, &[growing]);
        let second = commit_ops(&mut vt, &mut r, &[settled]);

        assert_eq!(
            second
                .iter()
                .filter(|op| matches!(op, super::super::virtual_terminal::Op::Free { .. }))
                .count(),
            1
        );
        assert!(second.iter().any(|op| matches!(
            op,
            super::super::virtual_terminal::Op::Upload {
                content: Content::Still(_),
                ..
            }
        )));
        assert!(second.iter().all(|op| !matches!(
            op,
            super::super::virtual_terminal::Op::Upload {
                content: Content::Animation { .. },
                ..
            }
        )));
    }

    #[test]
    fn the_typing_caret_canvas_has_a_lit_column_at_its_left_edge() {
        let mut r = renderer(16, 32);
        let colour = (0x11, 0x22, 0x33);
        let Content::Still(canvas) = Sprites::content(
            &mut r,
            &ImageKey::TypingCaret(TypingCaretKey {
                colour,
                bold: false,
            }),
        ) else {
            panic!("typing caret must be still");
        };
        let (red, green, blue, alpha) = pixel_at(&canvas.pixels, canvas.width, 0, 0);
        assert_eq!(
            (red, green, blue, alpha),
            (colour.0, colour.1, colour.2, OPAQUE)
        );
    }

    #[test]
    fn the_typing_caret_canvas_is_transparent_past_the_bar() {
        let mut r = renderer(16, 32);
        let colour = (0x11, 0x22, 0x33);
        let Content::Still(canvas) = Sprites::content(
            &mut r,
            &ImageKey::TypingCaret(TypingCaretKey {
                colour,
                bold: false,
            }),
        ) else {
            panic!("typing caret must be still");
        };
        let (_, _, _, alpha) = pixel_at(&canvas.pixels, canvas.width, canvas.width - 1, 0);
        assert_eq!(alpha, 0);
    }

    #[test]
    fn the_typing_caret_canvas_spans_the_whole_cell_height() {
        let mut r = renderer(16, 32);
        let colour = (0x11, 0x22, 0x33);
        let Content::Still(canvas) = Sprites::content(
            &mut r,
            &ImageKey::TypingCaret(TypingCaretKey {
                colour,
                bold: false,
            }),
        ) else {
            panic!("typing caret must be still");
        };
        assert_eq!(canvas.height, r.cells_to_pixels_y(1));
        for y in 0..canvas.height {
            let (red, green, blue, alpha) = pixel_at(&canvas.pixels, canvas.width, 0, y);
            assert_eq!(
                (red, green, blue, alpha),
                (colour.0, colour.1, colour.2, OPAQUE)
            );
        }
    }

    #[test]
    fn the_typing_caret_image_is_still_and_not_an_animation() {
        let mut r = renderer(16, 32);
        let content = Sprites::content(
            &mut r,
            &ImageKey::TypingCaret(TypingCaretKey {
                colour: (0x11, 0x22, 0x33),
                bold: false,
            }),
        );
        assert!(matches!(content, Content::Still(_)));
    }

    #[test]
    fn the_typing_caret_desired_sits_at_the_label_end_cell() {
        let mut r = renderer_on(window(8, 3, 1, 1));
        let label = label_placement("hi", 1, 1, 2, 1);
        let caret = typing_caret_placement(label.x + 2, label.y);
        let desired = paint_desired(&mut r, &[label.clone(), caret.clone()]);
        let typing: Vec<&Desired> = desired
            .iter()
            .filter(|d| matches!(d.image, ImageKey::TypingCaret(_)))
            .collect();
        assert_eq!(typing.len(), 1);
        assert_eq!((typing[0].col, typing[0].row), (caret.x, caret.y));
    }

    #[test]
    fn an_unchanged_typing_caret_commits_no_ops() {
        let mut r = renderer_on(window(8, 3, 1, 1));
        let mut vt = super::super::virtual_terminal::VirtualTerminal::new();
        let placements = [typing_caret_placement(4, 1)];

        commit_ops(&mut vt, &mut r, &placements);
        let ops = commit_ops(&mut vt, &mut r, &placements);

        assert!(ops.is_empty());
    }

    #[test]
    fn the_typing_caret_is_re_placed_as_the_label_grows() {
        let mut r = renderer_on(window(8, 3, 1, 1));
        let mut vt = super::super::virtual_terminal::VirtualTerminal::new();

        let first = commit_ops(&mut vt, &mut r, &[typing_caret_placement(3, 1)]);
        let (original_id, original_col) = first
            .iter()
            .find_map(|op| match op {
                super::super::virtual_terminal::Op::Place { placement, col, .. } => {
                    Some((*placement, *col))
                }
                _ => None,
            })
            .expect("the first commit places the caret");
        assert_eq!(original_col, 3);

        let second = commit_ops(&mut vt, &mut r, &[typing_caret_placement(4, 1)]);

        assert!(!second
            .iter()
            .any(|op| matches!(op, super::super::virtual_terminal::Op::Delete { .. })));
        let (moved_id, moved_col) = second
            .iter()
            .find_map(|op| match op {
                super::super::virtual_terminal::Op::Place { placement, col, .. } => {
                    Some((*placement, *col))
                }
                _ => None,
            })
            .expect("the caret is re-placed");
        assert_eq!(moved_id, original_id);
        assert_eq!(moved_col, 4);
    }
}
