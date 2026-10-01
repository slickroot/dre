use std::io::{self, Write};
use std::num::NonZeroU32;

use super::brackets::{corner_cells, corner_offset, BracketKey, CORNERS};
use super::font::GlyphSource;
use super::shapes::{ArrowShape, BoxShape, LedShape};
use super::tiles::{CellSize, TileKey, TileShape};
use super::{colour, Renderer, ARROW_OPACITY, OPAQUE, ROUNDED_RADIUS};
use crate::canvas::Canvas;
use crate::composer::Area;
use crate::kitty;
#[cfg(test)]
use crate::style::palette;
use crate::tty::Window;
use crate::view::Scene;
use crate::view::{Geometry, Label, Placement, PlacementNode, Rgb, Sides};

const BLANK: char = ' ';
const HOME_CURSOR: &str = "\x1b[H";
const BEGIN_SYNCHRONIZED_UPDATE: &str = "\x1b[?2026h";
const END_SYNCHRONIZED_UPDATE: &str = "\x1b[?2026l";

pub(super) const ARROW_STROKE: i64 = 3;
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
struct GlyphKey {
    character: char,
    colour: Rgb,
    bold: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Crop {
    col: i64,
    row: i64,
    first_x: i64,
    last_x: i64,
    first_y: i64,
    last_y: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SpriteKey {
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

enum Image {
    Fresh { id: kitty::ImageId, canvas: Canvas },
    Cached { id: kitty::ImageId },
}

struct Placed {
    image: Image,
    col: i64,
    row: i64,
    z: i32,
}

impl Placed {
    fn command(&self, placement: kitty::PlacementId) -> kitty::Command {
        match &self.image {
            Image::Fresh { id, canvas } => kitty::show(canvas, *id, self.col, self.row, self.z),
            Image::Cached { id } => kitty::place(*id, placement, self.col, self.row, self.z),
        }
    }
}

struct ImageIds {
    next: u32,
    transient: Vec<kitty::ImageId>,
}

impl ImageIds {
    fn new() -> Self {
        ImageIds {
            next: 1,
            transient: Vec::new(),
        }
    }

    fn allocate(&mut self) -> kitty::ImageId {
        let value = NonZeroU32::new(self.next).expect("image ID allocator exhausted");
        self.next = self
            .next
            .checked_add(1)
            .expect("image ID allocator exhausted");
        kitty::ImageId::new(value)
    }

    fn allocate_transient(&mut self) -> kitty::ImageId {
        let id = self.allocate();
        self.transient.push(id);
        id
    }

    fn take_transient(&mut self) -> Vec<kitty::ImageId> {
        std::mem::take(&mut self.transient)
    }
}

struct Frame {
    window: Window,
    characters: Vec<Vec<char>>,
    images: Vec<Placed>,
    previous_transient_images: Vec<kitty::ImageId>,
}

impl Frame {
    #[cfg(test)]
    fn new(window: Window) -> Self {
        Self::with_previous(window, Vec::new())
    }

    fn with_previous(window: Window, previous_transient_images: Vec<kitty::ImageId>) -> Self {
        Frame {
            window,
            characters: vec![vec![BLANK; window.cols as usize]; window.rows as usize],
            images: Vec::new(),
            previous_transient_images,
        }
    }

    // Clipping happens by cropping: kitty::show cannot position at a negative
    // column, and sends a=T without C=1, so an overhang would shift into view
    // or scroll the screen instead of being cut off.
    fn crop<G: Into<Geometry>>(&self, geometry: G, area: Area) -> Option<Crop> {
        let geometry = geometry.into();
        let (left, top) = (geometry.x, geometry.y);
        let col = left.max(area.col).max(0);
        let row = top.max(area.row).max(0);
        let right = (left + geometry.width)
            .min(area.col + area.cols)
            .min(self.window.cols);
        let bottom = (top + geometry.height)
            .min(area.row + area.rows)
            .min(self.window.rows);
        if col >= right || row >= bottom {
            return None;
        }
        Some(Crop {
            col,
            row,
            first_x: (col - left) * self.window.cell_width,
            last_x: (right - left) * self.window.cell_width,
            first_y: (row - top) * self.window.cell_height,
            last_y: (bottom - top) * self.window.cell_height,
        })
    }

    fn shows<G: Into<Geometry>>(&self, geometry: G, area: Area) -> bool {
        self.crop(geometry, area).is_some()
    }

    fn place_fresh<G: Into<Geometry>>(
        &mut self,
        id: kitty::ImageId,
        canvas: &Canvas,
        geometry: G,
        area: Area,
        z: i32,
    ) {
        if let Some(crop) = self.crop(geometry, area) {
            self.push_fresh(id, canvas, crop, z);
        }
    }

    fn place_transient<G: Into<Geometry>>(
        &mut self,
        ids: &mut ImageIds,
        canvas: &Canvas,
        geometry: G,
        area: Area,
        z: i32,
    ) {
        if let Some(crop) = self.crop(geometry, area) {
            self.push_fresh(ids.allocate_transient(), canvas, crop, z);
        }
    }

    fn place_cached<G: Into<Geometry>>(
        &mut self,
        id: kitty::ImageId,
        geometry: G,
        area: Area,
        z: i32,
    ) {
        if let Some(crop) = self.crop(geometry, area) {
            self.push(Image::Cached { id }, crop, z);
        }
    }

    fn push_fresh(&mut self, id: kitty::ImageId, canvas: &Canvas, crop: Crop, z: i32) {
        let canvas = canvas.crop(crop.first_x, crop.last_x, crop.first_y, crop.last_y);
        self.push(Image::Fresh { id, canvas }, crop, z);
    }

    fn push(&mut self, image: Image, crop: Crop, z: i32) {
        self.images.push(Placed {
            image,
            col: crop.col,
            row: crop.row,
            z,
        });
    }

    fn into_bytes(self) -> Vec<u8> {
        let rows: Vec<String> = self
            .characters
            .into_iter()
            .map(|row| row.into_iter().collect())
            .collect();
        let mut bytes = BEGIN_SYNCHRONIZED_UPDATE.as_bytes().to_vec();
        bytes.extend_from_slice(HOME_CURSOR.as_bytes());
        bytes.extend_from_slice(rows.join("\r\n").as_bytes());
        bytes.extend_from_slice(kitty::soft_clear().to_string().as_bytes());
        for id in &self.previous_transient_images {
            bytes.extend_from_slice(kitty::delete(*id).to_string().as_bytes());
        }
        for (placement, image) in Self::placement_ids().zip(&self.images) {
            bytes.extend_from_slice(image.command(placement).to_string().as_bytes());
        }
        bytes.extend_from_slice(END_SYNCHRONIZED_UPDATE.as_bytes());
        bytes
    }

    // WezTerm removes every placement of an image when a new placement of it
    // has no placement ID, so each placement in a frame is numbered apart.
    fn placement_ids() -> impl Iterator<Item = kitty::PlacementId> {
        (1..)
            .map_while(NonZeroU32::new)
            .map(kitty::PlacementId::new)
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

pub(crate) struct TerminalRenderer {
    window: Window,
    cache: std::collections::HashMap<SpriteKey, Canvas>,
    cache_limit: usize,
    glyph_source: Box<dyn GlyphSource>,
    glyph_images: std::collections::HashMap<GlyphKey, kitty::ImageId>,
    tile_canvases: std::collections::HashMap<TileKey, Canvas>,
    tile_images: std::collections::HashMap<TileKey, kitty::ImageId>,
    bracket_images: std::collections::HashMap<BracketKey, kitty::ImageId>,
    image_ids: ImageIds,
}

#[allow(clippy::too_many_arguments)]
fn place_sprite<K, C>(
    images: &mut std::collections::HashMap<K, kitty::ImageId>,
    image_ids: &mut ImageIds,
    frame: &mut Frame,
    key: K,
    geometry: Geometry,
    area: Area,
    z: i32,
    canvas: impl FnOnce() -> C,
) where
    K: std::hash::Hash + Eq,
    C: std::borrow::Borrow<Canvas>,
{
    if let Some(id) = images.get(&key).copied() {
        frame.place_cached(id, geometry, area, z);
    } else {
        let id = image_ids.allocate();
        images.insert(key, id);
        frame.place_fresh(id, canvas().borrow(), geometry, area, z);
    }
}

impl Renderer for TerminalRenderer {
    fn render(&mut self, scene: &Scene<'_>, out: &mut impl Write) -> io::Result<()> {
        let frame = self.frame(scene);
        out.write_all(&frame.into_bytes())
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
            glyph_images: std::collections::HashMap::new(),
            tile_canvases: std::collections::HashMap::new(),
            tile_images: std::collections::HashMap::new(),
            bracket_images: std::collections::HashMap::new(),
            image_ids: ImageIds::new(),
        }
    }

    pub(crate) fn on_resize(&mut self, window: Window) {
        self.window.cols = window.cols;
        self.window.rows = window.rows;
    }

    pub(crate) fn area(&self) -> Area {
        whole(self.window)
    }

    fn frame(&mut self, scene: &Scene<'_>) -> Frame {
        let mut frame = Frame::with_previous(self.window, self.image_ids.take_transient());
        for (area, placements) in scene {
            self.paint(&mut frame, placements, *area);
        }
        frame
    }

    fn paint(&mut self, frame: &mut Frame, placements: &[Placement], area: Area) {
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
                    ..
                } => self.draw_box(
                    frame,
                    geometry,
                    area,
                    placement.depth,
                    BoxStyle {
                        colour: *colour,
                        fill: *fill,
                        fill_alpha: quantized_alpha(*opacity),
                        solid_fill: *solid_fill,
                        rounded: *rounded,
                        sides: *sides,
                        border: *border,
                    },
                ),
                PlacementNode::Brackets { border } => {
                    self.draw_brackets(frame, geometry, area, *border)
                }
                PlacementNode::Arrow(arrow) => self.draw_arrow(
                    frame,
                    geometry,
                    area,
                    ArrowStyle {
                        stops: arrow.stops.clone(),
                        shaft: arrow.shaft,
                    },
                ),
                PlacementNode::Label(label) => self.draw_label(
                    frame,
                    geometry,
                    area,
                    placement.depth,
                    LabelStyle {
                        text: label.text.to_string(),
                        colour: label.colour,
                        bold: label.bold,
                    },
                ),
                PlacementNode::Caret(_) => self.draw_caret(frame, geometry, area),
                PlacementNode::Cursor(_) => self.draw_cursor(frame, geometry, area),
                PlacementNode::Led { colour, lit } => self.draw_led(
                    frame,
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

    fn place_cached(
        &mut self,
        frame: &mut Frame,
        key: SpriteKey,
        geometry: Geometry,
        area: Area,
        z: i32,
        build: impl FnOnce(&Self) -> Canvas,
    ) {
        if !frame.shows(geometry, area) {
            return;
        }
        if !self.cache.contains_key(&key) {
            let drawn = build(self);
            self.remember(key.clone(), drawn);
        }
        frame.place_transient(&mut self.image_ids, &self.cache[&key], geometry, area, z);
    }

    fn place_tiles(
        &mut self,
        frame: &mut Frame,
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
        for (col, row, key) in tiles {
            let cell = Geometry {
                x: geometry.x + col,
                y: geometry.y + row,
                width: 1,
                height: 1,
            };
            if !frame.shows(cell, area) {
                continue;
            }
            let tile_canvases = &mut self.tile_canvases;
            place_sprite(
                &mut self.tile_images,
                &mut self.image_ids,
                frame,
                key,
                cell,
                area,
                z,
                || &*tile_canvases.entry(key).or_insert_with(|| key.canvas()),
            );
        }
        true
    }

    fn draw_box(
        &mut self,
        frame: &mut Frame,
        geometry: Geometry,
        area: Area,
        depth: u8,
        style: BoxStyle,
    ) {
        let z = depth_z(depth);
        if self.place_tiles(frame, style, geometry, area, z) {
            return;
        }
        let key = box_key(geometry.width, geometry.height, style);
        self.place_cached(frame, key, geometry, area, z, |renderer| {
            renderer.box_canvas(geometry.width, geometry.height, style)
        });
    }

    fn draw_brackets(&mut self, frame: &mut Frame, geometry: Geometry, area: Area, border: i64) {
        let cell = self.cell_size();
        let blocks = corner_cells(cell);
        for corner in CORNERS {
            let (col, row) = corner_offset(corner, geometry.width, geometry.height, cell);
            let block = Geometry {
                x: geometry.x + col,
                y: geometry.y + row,
                width: blocks,
                height: blocks,
            };
            if !frame.shows(block, area) {
                continue;
            }
            let key = BracketKey {
                corner,
                border,
                cell,
            };
            place_sprite(
                &mut self.bracket_images,
                &mut self.image_ids,
                frame,
                key,
                block,
                area,
                BRACKETS_Z,
                || key.canvas(),
            );
        }
    }

    fn draw_led(&mut self, frame: &mut Frame, geometry: Geometry, area: Area, style: LedStyle) {
        let key = led_key(geometry.width, geometry.height, style);
        self.place_cached(frame, key, geometry, area, INK_Z, |renderer| {
            renderer.led_canvas(geometry.width, geometry.height, style)
        });
    }

    fn draw_arrow(&mut self, frame: &mut Frame, geometry: Geometry, area: Area, style: ArrowStyle) {
        let key = arrow_key(geometry.width, geometry.height, &style);
        self.place_cached(frame, key, geometry, area, CONTENT_Z, |renderer| {
            renderer.arrow_canvas(geometry.width, geometry.height, &style)
        });
    }

    fn draw_label(
        &mut self,
        frame: &mut Frame,
        geometry: Geometry,
        area: Area,
        depth: u8,
        style: LabelStyle,
    ) {
        let z = depth_z(depth);
        for (offset, character) in style.text.chars().enumerate() {
            let char_placement = Placement {
                node: PlacementNode::Label(Label {
                    text: style.text.clone().into(),
                    colour: style.colour,
                    bold: style.bold,
                }),
                x: geometry.x + offset as i64,
                y: geometry.y,
                width: 1,
                height: 1,
                depth,
            };
            if !frame.shows(&char_placement, area) {
                continue;
            }
            let key = GlyphKey {
                character,
                colour: style.colour,
                bold: style.bold,
            };
            if let Some(id) = self.glyph_images.get(&key).copied() {
                frame.place_cached(id, &char_placement, area, z);
            } else {
                let id = self.image_ids.allocate();
                self.glyph_images.insert(key, id);
                let glyph = self.glyph_source.glyph(character, style.colour, style.bold);
                frame.place_fresh(id, glyph, &char_placement, area, z);
            }
        }
    }

    fn draw_caret(&mut self, frame: &mut Frame, geometry: Geometry, area: Area) {
        let width = self.cells_to_pixels_x(geometry.width);
        let height = self.cells_to_pixels_y(geometry.height);
        let (r, g, b) = colour(None);
        let canvas = Canvas::fill(
            width,
            height,
            &SolidShape {
                colour: [r, g, b, OPAQUE],
            },
        );
        frame.place_transient(&mut self.image_ids, &canvas, geometry, area, CONTENT_Z);
    }

    fn draw_cursor(&mut self, frame: &mut Frame, geometry: Geometry, area: Area) {
        self.draw_caret(frame, geometry, area);
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
        Canvas::fill(width, height, &box_shape(width, height, style))
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
}

#[cfg(test)]
mod tests {
    use super::super::font::FakeGlyphSource;
    use super::super::tiles::{cells_with_middle, TileShape};
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
            sprites(&mut r, &[box_placement(variant, 0, 0, 4, 3)]);
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
        let images = sprites(&mut r, &[box_placement(&node, 4, 4, 4, 4)]);
        assert_eq!(images.len(), 1);
    }

    #[test]
    fn a_selected_box_places_its_brackets_above_the_fill_and_below_content() {
        let mut r = renderer_on(window(20, 20, 2, 2));
        let node = selected_box_node();
        let images = sprites(
            &mut r,
            &[
                box_placement(&box_node(Some(1), None, false), 4, 4, 4, 4),
                box_placement(&node, 3, 3, 6, 6),
                caret_placement(5, 5, 1, 1),
            ],
        );
        let layers: std::collections::BTreeSet<i32> = images.iter().map(|image| image.z).collect();
        let brackets = composed(&images, BRACKETS_Z, r.window);
        let boxed = composed(&images, depth_z(0), r.window);
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

    fn z_at(images: &[Placed], cell: (i64, i64)) -> i32 {
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
        let images = sprites(
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
        let images = sprites(&mut r, &[label, deeper]);
        assert!(z_at(&images, (12, 12)) > z_at(&images, (2, 2)));
    }

    #[test]
    fn the_brackets_is_centred_on_the_box_and_extends_beyond_it() {
        let mut r = renderer_on(window(20, 20, 2, 2));
        let node = selected_box_node();
        let images = sprites(&mut r, &[box_placement(&node, 7, 7, 6, 6)]);
        let brackets = composed(&images, BRACKETS_Z, r.window);
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
        let images = sprites(&mut r, &[box_placement(&node, 3, 3, 6, 6)]);
        let brackets = composed(&images, BRACKETS_Z, r.window);
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
        let images = sprites(&mut r, &[box_placement(&node, 3, 3, 6, 6)]);
        let brackets = composed(&images, BRACKETS_Z, r.window);
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
            assert_eq!(placed_ids(&next).len(), CORNERS.len());
        }
    }

    impl Placed {
        fn canvas(&self) -> &Canvas {
            match &self.image {
                Image::Fresh { canvas, .. } => canvas,
                Image::Cached { .. } => panic!("a cached image carries no canvas"),
            }
        }
    }

    struct Sprite {
        col: i64,
        row: i64,
        canvas: Canvas,
    }

    fn composed<'a>(images: &'a [Placed], z: i32, window: Window) -> Sprite {
        let layer: Vec<&'a Placed> = images.iter().filter(|image| image.z == z).collect();
        let fresh: std::collections::HashMap<kitty::ImageId, &'a Canvas> = layer
            .iter()
            .filter_map(|image| match &image.image {
                Image::Fresh { id, canvas } => Some((*id, canvas)),
                Image::Cached { .. } => None,
            })
            .collect();
        let canvas_of = |image: &'a Placed| -> &'a Canvas {
            match &image.image {
                Image::Fresh { canvas, .. } => canvas,
                Image::Cached { id } => fresh[id],
            }
        };
        let col = layer.iter().map(|image| image.col).min().expect("a layer");
        let row = layer.iter().map(|image| image.row).min().expect("a layer");
        let origin = |image: &Placed| {
            (
                (image.col - col) * window.cell_width,
                (image.row - row) * window.cell_height,
            )
        };
        let width = layer
            .iter()
            .map(|image| origin(image).0 + canvas_of(image).width)
            .max()
            .expect("a layer");
        let height = layer
            .iter()
            .map(|image| origin(image).1 + canvas_of(image).height)
            .max()
            .expect("a layer");
        let channels = 4;
        let mut pixels = vec![0u8; (width * height * channels) as usize];
        for image in &layer {
            let (x, y) = origin(image);
            let canvas = canvas_of(image);
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

    fn drawn_frame(r: &mut TerminalRenderer, placements: &[Placement]) -> Frame {
        let mut frame = Frame::new(r.window);
        let area = whole(r.window);
        r.paint(&mut frame, placements, area);
        frame
    }

    fn rows(frame: &Frame) -> Vec<String> {
        frame
            .characters
            .iter()
            .map(|row| row.iter().collect())
            .collect()
    }

    fn grid(r: &mut TerminalRenderer, placements: &[Placement]) -> Vec<String> {
        rows(&drawn_frame(r, placements))
    }

    fn sprites(r: &mut TerminalRenderer, placements: &[Placement]) -> Vec<Placed> {
        drawn_frame(r, placements).images
    }

    fn unwrapped(frame: &str) -> &str {
        frame
            .strip_prefix(BEGIN_SYNCHRONIZED_UPDATE)
            .unwrap()
            .strip_suffix(END_SYNCHRONIZED_UPDATE)
            .unwrap()
    }

    fn lines_of(frame: &str) -> Vec<&str> {
        unwrapped(frame)
            .strip_prefix(HOME_CURSOR)
            .unwrap()
            .split("\r\n")
            .collect()
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

    #[test]
    fn a_box_on_screen_is_cropped_to_the_whole_shape() {
        let node = box_node(None, None, false);
        let window = window(20, 10, 4, 8);
        let (width, height) = (5, 4);
        let frame = Frame::new(window);
        assert_eq!(
            frame.crop(
                &box_placement(&node, 2, 3, width, height),
                whole(frame.window)
            ),
            Some(Crop {
                col: 2,
                row: 3,
                first_x: 0,
                last_x: width * window.cell_width,
                first_y: 0,
                last_y: height * window.cell_height,
            })
        );
    }

    #[test]
    fn a_crop_is_clipped_to_its_area_not_the_window() {
        let node = box_node(None, None, false);
        let window = window(20, 10, 4, 8);
        let area = Area {
            col: 2,
            row: 3,
            cols: 5,
            rows: 4,
        };
        let frame = Frame::new(window);
        let crop = frame
            .crop(&box_placement(&node, 0, 0, window.cols, window.rows), area)
            .unwrap();
        assert_eq!((crop.col, crop.row), (area.col, area.row));
        assert_eq!(
            (crop.first_x, crop.last_x),
            (
                area.col * window.cell_width,
                (area.col + area.cols) * window.cell_width
            )
        );
        assert_eq!(
            (crop.first_y, crop.last_y),
            (
                area.row * window.cell_height,
                (area.row + area.rows) * window.cell_height
            )
        );
    }

    #[test]
    fn a_crop_overhanging_the_left_drops_the_hidden_columns() {
        let node = box_node(None, None, false);
        let window = window(20, 10, 4, 8);
        let (hidden, width) = (2, 5);
        let frame = Frame::new(window);
        let crop = frame
            .crop(
                &box_placement(&node, -hidden, 0, width, 3),
                whole(frame.window),
            )
            .unwrap();
        assert_eq!(crop.col, 0);
        assert_eq!(crop.first_x, hidden * window.cell_width);
        assert_eq!(crop.last_x, width * window.cell_width);
    }

    #[test]
    fn a_crop_overhanging_the_top_drops_the_hidden_rows() {
        let node = box_node(None, None, false);
        let window = window(20, 10, 4, 8);
        let (hidden, height) = (2, 5);
        let frame = Frame::new(window);
        let crop = frame
            .crop(
                &box_placement(&node, 0, -hidden, 3, height),
                whole(frame.window),
            )
            .unwrap();
        assert_eq!(crop.row, 0);
        assert_eq!(crop.first_y, hidden * window.cell_height);
        assert_eq!(crop.last_y, height * window.cell_height);
    }

    #[test]
    fn a_crop_overhanging_the_right_stops_at_the_last_column() {
        let node = box_node(None, None, false);
        let window = window(20, 10, 4, 8);
        let x = 18;
        let frame = Frame::new(window);
        let crop = frame
            .crop(&box_placement(&node, x, 0, 5, 3), whole(frame.window))
            .unwrap();
        assert_eq!(crop.col, x);
        assert_eq!(crop.first_x, 0);
        assert_eq!(crop.last_x, (window.cols - x) * window.cell_width);
    }

    #[test]
    fn a_crop_overhanging_the_bottom_stops_at_the_last_row() {
        let node = box_node(None, None, false);
        let window = window(20, 10, 4, 8);
        let y = 8;
        let frame = Frame::new(window);
        let crop = frame
            .crop(&box_placement(&node, 0, y, 3, 5), whole(frame.window))
            .unwrap();
        assert_eq!(crop.row, y);
        assert_eq!(crop.first_y, 0);
        assert_eq!(crop.last_y, (window.rows - y) * window.cell_height);
    }

    #[test]
    fn a_box_beyond_the_right_edge_has_no_crop() {
        let node = box_node(None, None, false);
        let frame = Frame::new(window(20, 10, 4, 8));
        assert_eq!(
            frame.crop(&box_placement(&node, 20, 0, 4, 3), whole(frame.window)),
            None
        );
    }

    #[test]
    fn a_box_beyond_the_top_edge_has_no_crop() {
        let node = box_node(None, None, false);
        let frame = Frame::new(window(20, 10, 4, 8));
        assert_eq!(
            frame.crop(&box_placement(&node, 0, -3, 4, 3), whole(frame.window)),
            None
        );
    }

    #[test]
    fn line_count_is_unchanged() {
        let mut r = renderer_on(window(3, 3, 2, 4));
        assert_eq!(lines_of(&rendered(&mut r, &empty_state())).len(), 3);
    }

    #[test]
    fn the_graphics_payload_is_appended_to_the_last_line_only() {
        let frame = Frame::new(window(3, 2, 2, 4));
        let output = String::from_utf8(frame.into_bytes()).unwrap();
        let lines = lines_of(&output);
        assert_eq!(lines[0], BLANK.to_string().repeat(3));
        assert_eq!(
            lines[1],
            format!("{}{}", BLANK.to_string().repeat(3), kitty::soft_clear())
        );
    }

    #[test]
    fn a_frame_with_sprites_ends_with_a_clear_then_each_sprite_shown_in_order() {
        let node = box_node(None, None, false);
        let placements = [
            box_placement(&node, 0, 0, 4, 3),
            box_placement(&node, 6, 0, 4, 3),
        ];
        let mut r = renderer_on(window(10, 3, 2, 4));
        let frame = drawn_frame(&mut r, &placements);
        assert!(frame.images.len() > 1);
        let expected: String = std::iter::once(kitty::soft_clear())
            .chain(
                Frame::placement_ids()
                    .zip(&frame.images)
                    .map(|(placement, image)| match &image.image {
                        Image::Fresh { id, canvas } => {
                            kitty::show(canvas, *id, image.col, image.row, image.z)
                        }
                        Image::Cached { id } => {
                            kitty::place(*id, placement, image.col, image.row, image.z)
                        }
                    }),
            )
            .map(|command| command.to_string())
            .collect();
        let bytes = String::from_utf8(frame.into_bytes()).unwrap();
        assert!(unwrapped(&bytes).ends_with(&expected));
    }

    fn a_labelled_box(width: i64, height: i64) -> [Placement<'static>; 2] {
        [
            box_placement(&box_node(None, None, false), 0, 0, width, height),
            label_placement("hi", 1, height / 2, 2, 1),
        ]
    }

    #[test]
    fn an_overflowing_diagram_is_cropped_equally_on_both_sides() {
        let state = one_leaf();
        let width = leaf_box(&state).width;
        let cut_each_side = 1;
        let window = window(width - 2 * cut_each_side, 10, 1, 1);
        let frame = Frame::new(window);
        let screen = editor(&state, whole(window));
        let (body, diagram) = &screen[0];
        let placed = &diagram[0];
        let crop = frame.crop(placed, *body).unwrap();
        let cut_on_left = crop.first_x;
        let cut_on_right = placed.width * window.cell_width - crop.last_x;
        assert_eq!(cut_on_left, cut_each_side);
        assert_eq!(cut_on_right, cut_each_side);
    }

    #[test]
    fn on_resize_re_centres_the_next_render_on_the_new_size() {
        let state = one_leaf();
        let leaf = leaf_box(&state);
        let mut r = renderer_on(window(20, 10, 1, 1));
        framed(&mut r, &state);
        let (cols, rows) = (40, 20);
        r.on_resize(window(cols, rows, 1, 1));
        let frame = framed(&mut r, &state);
        assert_eq!(
            (frame.images[0].col, frame.images[0].row),
            (
                (cols - leaf.width).div_euclid(2),
                (rows - FOOTER_ROWS - leaf.height).div_euclid(2)
            )
        );
    }

    #[test]
    fn on_resize_with_a_different_rounded_cell_height_does_not_panic_on_the_next_render() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let node = box_node(None, None, false);
        let placement = box_placement(&node, 0, 0, 4, 3);
        sprites(&mut r, std::slice::from_ref(&placement));
        r.on_resize(window(40, 20, 2, 5));
        sprites(&mut r, &[placement]);
    }

    #[test]
    fn on_resize_leaves_the_sprite_cache_untouched() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 0, 4, 3)],
        );
        assert_eq!(r.cache.len(), 1);
        r.on_resize(window(80, 40, 2, 4));
        assert_eq!(r.cache.len(), 1);
    }

    fn rendered(r: &mut TerminalRenderer, state: &State) -> String {
        let mut out = Vec::new();
        r.render(&editor(state, whole(r.window)), &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn rendered_placements(r: &mut TerminalRenderer, placements: &[Placement<'static>]) -> String {
        let scene = vec![(whole(r.window), placements.to_vec())];
        String::from_utf8(r.frame(&scene).into_bytes()).unwrap()
    }

    fn framed(r: &mut TerminalRenderer, state: &State) -> Frame {
        let scene = editor(state, whole(r.window));
        r.frame(&scene)
    }

    fn empty_state() -> State {
        crate::state::new_state(vec![], Mode::Command, None)
    }

    #[test]
    fn the_cursor_goes_home_before_the_lines() {
        let frame = Frame::new(Window {
            cols: 2,
            rows: 2,
            cell_width: 1,
            cell_height: 1,
        });
        assert_eq!(
            unwrapped(&String::from_utf8(frame.into_bytes()).unwrap()),
            format!("{HOME_CURSOR}  \r\n  {}", kitty::soft_clear())
        );
    }

    #[test]
    fn a_frame_begins_a_synchronized_update_before_homing_the_cursor_and_ends_it_last() {
        let mut r = renderer_on(window(3, 3, 2, 4));
        let output = rendered(&mut r, &empty_state());
        assert!(output.starts_with(&format!("{BEGIN_SYNCHRONIZED_UPDATE}{HOME_CURSOR}")));
        assert!(output.ends_with(END_SYNCHRONIZED_UPDATE));
    }

    fn a_frame_after_one_with_a_transient_image(r: &mut TerminalRenderer) -> Frame {
        let first = [
            label_placement("aa", 0, 0, 2, 1),
            caret_placement(2, 0, 1, 1),
        ];
        let second = [
            label_placement("ab", 0, 0, 2, 1),
            caret_placement(2, 0, 1, 1),
        ];
        rendered_placements(r, &first);
        r.frame(&vec![(whole(r.window), second.to_vec())])
    }

    fn assert_one_synchronized_update_ending_after_the_images(frame: Frame) {
        let last_image = Frame::placement_ids()
            .zip(&frame.images)
            .last()
            .map(|(placement, image)| image.command(placement).to_string());
        let output = String::from_utf8(frame.into_bytes()).unwrap();
        assert_eq!(output.matches(BEGIN_SYNCHRONIZED_UPDATE).count(), 1);
        assert_eq!(output.matches(END_SYNCHRONIZED_UPDATE).count(), 1);
        let end = output.find(END_SYNCHRONIZED_UPDATE).unwrap();
        if let Some(last_image) = last_image {
            assert!(output.rfind(&last_image).unwrap() + last_image.len() <= end);
        }
    }

    #[test]
    fn an_empty_frame_holds_one_synchronized_update() {
        assert_one_synchronized_update_ending_after_the_images(Frame::new(window(3, 2, 2, 4)));
    }

    #[test]
    fn a_frame_of_fresh_cached_and_transient_images_holds_one_synchronized_update_ending_after_them(
    ) {
        let mut r = renderer_on(window(3, 1, 1, 1));
        let frame = a_frame_after_one_with_a_transient_image(&mut r);
        assert!(!frame.previous_transient_images.is_empty());
        assert_one_synchronized_update_ending_after_the_images(frame);
    }

    #[test]
    fn inside_the_synchronized_update_come_rows_soft_clear_deletes_then_images_in_scene_order() {
        let mut r = renderer_on(window(3, 1, 1, 1));
        let frame = a_frame_after_one_with_a_transient_image(&mut r);
        assert!(frame
            .images
            .iter()
            .any(|image| matches!(image.image, Image::Fresh { .. })));
        assert!(frame
            .images
            .iter()
            .any(|image| matches!(image.image, Image::Cached { .. })));
        assert!(!frame.previous_transient_images.is_empty());
        let expected: String = [HOME_CURSOR.to_string(), rows(&frame).join("\r\n")]
            .into_iter()
            .chain(std::iter::once(kitty::soft_clear().to_string()))
            .chain(
                frame
                    .previous_transient_images
                    .iter()
                    .map(|id| kitty::delete(*id).to_string()),
            )
            .chain(
                Frame::placement_ids()
                    .zip(&frame.images)
                    .map(|(placement, image)| image.command(placement).to_string()),
            )
            .collect();
        let output = String::from_utf8(frame.into_bytes()).unwrap();
        assert_eq!(unwrapped(&output), expected);
    }

    #[test]
    fn no_newline_follows_the_last_line() {
        let mut r = renderer_on(Window {
            cols: 2,
            rows: 2,
            cell_width: 1,
            cell_height: 1,
        });
        assert!(!rendered(&mut r, &empty_state()).ends_with('\n'));
    }

    #[test]
    fn repeated_glyphs_transmit_once_and_place_the_cached_image() {
        let placements = [label_placement("aa", 0, 0, 2, 1)];
        let mut r = renderer_on(window(3, 1, 1, 1));

        let output = rendered_placements(&mut r, &placements);

        let shown = shown_ids(&output);
        assert_eq!(shown.len(), 1);
        assert_eq!(placed_ids(&output), shown);
        assert!(output.find("a=T").unwrap() < output.find("a=p").unwrap());
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
        command_fields(output, "a=T,", "i=")
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

    fn image_id(value: &str) -> kitty::ImageId {
        kitty::ImageId::new(value.parse().expect("image IDs are non-zero numbers"))
    }

    fn all_distinct(ids: &[String]) -> bool {
        ids.iter().collect::<std::collections::HashSet<_>>().len() == ids.len()
    }

    #[test]
    fn repeated_glyphs_in_a_frame_each_get_their_own_placement_id() {
        let text = "aaa";
        let placements = [label_placement(text, 0, 0, text.len() as i64, 1)];
        let mut r = renderer_on(window(3, 1, 1, 1));

        let output = rendered_placements(&mut r, &placements);

        let ids = placement_ids(&output);
        assert_eq!(ids.len(), text.len() - 1);
        assert!(all_distinct(&ids));
    }

    #[test]
    fn fully_cached_glyphs_in_a_later_frame_each_get_their_own_placement_id() {
        let text = "aaa";
        let placements = [label_placement(text, 0, 0, text.len() as i64, 1)];
        let mut r = renderer_on(window(3, 1, 1, 1));

        rendered_placements(&mut r, &placements);
        let second = rendered_placements(&mut r, &placements);

        let ids = placement_ids(&second);
        assert_eq!(ids.len(), text.len());
        assert!(all_distinct(&ids));
    }

    #[test]
    fn a_glyph_is_placed_from_the_terminal_cache_in_the_next_frame() {
        let placements = [label_placement("a", 0, 0, 1, 1)];
        let mut r = renderer_on(window(3, 1, 1, 1));

        let first = rendered_placements(&mut r, &placements);
        let second = rendered_placements(&mut r, &placements);

        assert_eq!(first.matches("a=T").count(), 1);
        assert_eq!(second.matches("a=T").count(), 0);
        assert_eq!(second.matches("a=p").count(), 1);
        assert!(unwrapped(&second).starts_with(&format!("{HOME_CURSOR}   {}", kitty::soft_clear())));
        assert!(deleted_ids(&second).is_empty());
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
        assert!(placed_ids(&output).is_empty());
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
    fn a_non_glyph_image_is_retransmitted_with_a_new_id_and_its_old_id_deleted() {
        let placements = [caret_placement(0, 0, 1, 1)];
        let mut r = renderer_on(window(3, 1, 1, 1));

        let first = rendered_placements(&mut r, &placements);
        let second = rendered_placements(&mut r, &placements);

        let (first_shown, second_shown) = (shown_ids(&first), shown_ids(&second));
        assert_eq!(first_shown.len(), 1);
        assert_eq!(second_shown.len(), 1);
        assert_ne!(first_shown, second_shown);
        assert_eq!(deleted_ids(&second), first_shown);
    }

    #[test]
    fn transient_images_are_deleted_after_soft_clear_before_the_next_frame() {
        let placements = [caret_placement(0, 0, 1, 1)];
        let mut r = renderer_on(window(3, 1, 1, 1));

        let first = rendered_placements(&mut r, &placements);
        let second = rendered_placements(&mut r, &placements);

        let [first_id] = shown_ids(&first).try_into().unwrap();
        let clear_then_delete = format!(
            "{}{}",
            kitty::soft_clear(),
            kitty::delete(image_id(&first_id))
        );
        let clear_and_delete = second.find(&clear_then_delete).unwrap();
        let show = second.find("a=T").unwrap();
        assert!(clear_and_delete < show);
    }

    #[test]
    fn the_output_is_sized_by_the_terminal() {
        let (cols, rows) = (5, 4);
        let frame = Frame::new(Window {
            cols,
            rows,
            cell_width: 1,
            cell_height: 1,
        });
        let output = String::from_utf8(frame.into_bytes()).unwrap();
        let body = unwrapped(&output)
            .strip_prefix(HOME_CURSOR)
            .unwrap()
            .strip_suffix(&kitty::soft_clear().to_string())
            .unwrap();
        assert_eq!(
            body.split("\r\n").collect::<Vec<_>>(),
            vec![BLANK.to_string().repeat(cols as usize); rows as usize]
        );
    }

    #[test]
    fn the_caret_is_drawn_last_as_a_solid_sprite() {
        let mut r = renderer_on(window(20, 10, 1, 1));
        let [box_at, label] = a_labelled_box(4, 3);
        let caret = caret_placement(label.x + label.width - 1, label.y, 1, 1);
        let images = sprites(&mut r, &[box_at, label, caret]);
        let caret_image = images.last().expect("a sprite is drawn for the caret");
        let (cr, cg, cb) = colour(None);
        let solid = [cr, cg, cb, OPAQUE];
        assert!(caret_image
            .canvas()
            .pixels
            .chunks(4)
            .all(|pixel| pixel == solid));
    }

    #[test]
    fn an_empty_drawing_draws_only_in_the_footer_row() {
        let window = window(40, 10, 1, 1);
        let mut r = renderer_on(window);
        let frame = framed(&mut r, &empty_state());
        assert!(frame
            .images
            .iter()
            .all(|image| image.row >= window.rows - FOOTER_ROWS));
    }

    #[test]
    fn the_footer_box_is_filled_with_the_foreground_colour_and_has_no_border() {
        let cell = 4;
        let mut r = renderer_on(window(40, 10, cell, cell));
        let frame = framed(&mut r, &empty_state());
        let footer = composed(&frame.images, depth_z(0), r.window);
        let (er, eg, eb, ea) = fill_colour(
            Some(crate::style::FOREGROUND),
            quantized_alpha(Some(FOOTER_FILL_OPACITY)),
        );
        for pixel in footer.canvas.pixels.chunks(4) {
            assert_eq!(pixel, [er, eg, eb, ea]);
        }
    }

    #[test]
    fn empty_canvas_fills_terminal() {
        let grid = grid(&mut renderer_on(window(11, 5, 1, 1)), &[]);
        assert_eq!(grid, vec![BLANK.to_string().repeat(11); 5]);
    }

    #[test]
    fn grid_matches_the_requested_size() {
        let (cols, rows) = (20, 7);
        let grid = grid(
            &mut renderer_on(window(cols, rows, 1, 1)),
            &[box_placement(&box_node(None, None, false), 4, 4, 3, 3)],
        );
        assert_eq!(grid.len() as i64, rows);
        for line in &grid {
            assert_eq!(line.chars().count() as i64, cols);
        }
    }

    #[test]
    fn a_box_claims_its_cells_without_border_characters() {
        let grid = grid(
            &mut renderer_on(window(11, 11, 1, 1)),
            &[box_placement(&box_node(None, None, false), 4, 4, 3, 3)],
        );
        assert_eq!(&grid[4][4..7], "   ");
    }

    #[test]
    fn a_box_reaching_past_the_edge_is_clipped() {
        let grid = grid(
            &mut renderer_on(window(4, 2, 1, 1)),
            &[box_placement(&box_node(None, None, false), 3, 1, 3, 3)],
        );
        assert_eq!(grid, vec!["    ".to_string(), "    ".to_string()]);
    }

    #[test]
    fn label_is_drawn_inside_the_box() {
        let node = box_node(None, None, false);
        let placements = vec![
            box_placement(&node, 0, 0, 5, 3),
            label_placement("hi", 1, 1, 2, 1),
        ];
        let mut r = renderer_on(window(5, 3, 1, 1));
        let frame = drawn_frame(&mut r, &placements);
        let label_cols: Vec<i64> = frame
            .images
            .iter()
            .filter(|image| image.row == 1)
            .map(|image| image.col)
            .collect();
        assert_eq!(label_cols, vec![1, 2]);
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
        let regular_pixels = sprites(&mut r, &[bold_label_placement("h", 0, 0, 1, 1, false)])[0]
            .canvas()
            .pixels
            .clone();

        let mut r = renderer_on(window(2, 1, 1, 1));
        let bold_pixels = sprites(&mut r, &[bold_label_placement("h", 0, 0, 1, 1, true)])[0]
            .canvas()
            .pixels
            .clone();

        assert_ne!(regular_pixels, bold_pixels);
    }

    #[test]
    fn a_label_with_a_colour_renders_a_different_glyph_colour_than_the_default() {
        let mut r = renderer_on(window(2, 1, 1, 1));
        let default_pixels = sprites(&mut r, &[coloured_label_placement("h", 0, 0, 1, 1, None)])[0]
            .canvas()
            .pixels
            .clone();

        let mut r = renderer_on(window(2, 1, 1, 1));
        let coloured_pixels = sprites(
            &mut r,
            &[coloured_label_placement(
                "h",
                0,
                0,
                1,
                1,
                Some(crate::style::LIME),
            )],
        )[0]
        .canvas()
        .pixels
        .clone();

        assert_ne!(default_pixels, coloured_pixels);
    }

    #[test]
    fn a_label_without_a_colour_matches_todays_default_foreground_rendering() {
        let mut r = renderer_on(window(2, 1, 1, 1));
        let pixels = sprites(&mut r, &[label_placement("h", 0, 0, 1, 1)])[0]
            .canvas()
            .pixels
            .clone();

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
        let frame = drawn_frame(&mut r, &placements);
        assert_eq!(rows(&frame)[1], "     ".to_string());
        let (cr, cg, cb) = colour(None);
        let solid = [cr, cg, cb, OPAQUE];
        let is_caret =
            |image: &&Placed| image.canvas().pixels.chunks(4).all(|pixel| pixel == solid);
        let caret_cols: Vec<i64> = frame
            .images
            .iter()
            .filter(|image| image.row == 1)
            .filter(is_caret)
            .map(|image| image.col)
            .collect();
        assert_eq!(caret_cols, vec![3]);
        let label_cols: Vec<i64> = frame
            .images
            .iter()
            .filter(|image| image.row == 1)
            .filter(|image| !is_caret(image))
            .map(|image| image.col)
            .collect();
        assert_eq!(label_cols, vec![1, 2]);
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
        let frame = drawn_frame(&mut r, &placements);
        let label_cols: Vec<i64> = frame
            .images
            .iter()
            .filter(|image| image.row == 1)
            .map(|image| image.col)
            .collect();
        assert_eq!(label_cols, vec![1, 2]);
    }

    #[test]
    fn a_box_does_not_draw_a_caret() {
        let grid = grid(
            &mut renderer_on(window(5, 3, 1, 1)),
            &[box_placement(&box_node(None, None, false), 0, 0, 5, 3)],
        );
        assert!(!grid.join("").contains('\u{2588}'));
    }

    #[test]
    fn caret_placement_is_drawn_at_its_own_position() {
        let mut r = renderer_on(window(4, 3, 1, 1));
        let images = sprites(&mut r, &[caret_placement(2, 1, 1, 1)]);
        assert_eq!(images.len(), 1);
        assert_eq!((images[0].col, images[0].row), (2, 1));
        let (cr, cg, cb) = colour(None);
        assert_eq!(&images[0].canvas().pixels[0..4], &[cr, cg, cb, OPAQUE]);
    }

    #[test]
    fn a_caret_outside_the_grid_is_clipped() {
        let mut r = renderer_on(window(4, 3, 1, 1));
        let images = sprites(&mut r, &[caret_placement(9, 9, 1, 1)]);
        assert!(images.is_empty());
    }

    #[test]
    fn an_arrow_leaves_the_gap_blank() {
        let grid = grid(
            &mut renderer_on(window(4, 4, 1, 1)),
            &[arrow_placement(vec![0], 0, 2, 1, 1, 2)],
        );
        assert_eq!(grid, vec!["    ".to_string(); 4]);
    }

    #[test]
    fn a_plain_box_emits_no_escapes() {
        let node = box_node(None, None, false);
        let placements = vec![
            box_placement(&node, 0, 0, 5, 3),
            label_placement("hi", 1, 1, 2, 1),
            caret_placement(3, 1, 1, 1),
        ];
        let grid = grid(&mut renderer_on(window(5, 3, 1, 1)), &placements);
        assert!(!grid.join("").contains('\x1b'));
    }

    #[test]
    fn a_coloured_box_puts_no_colour_in_the_grid() {
        let grid = grid(
            &mut renderer_on(window(5, 3, 1, 1)),
            &[box_placement(&box_node(Some(2), None, false), 0, 0, 5, 3)],
        );
        assert_eq!(grid, vec!["     ".to_string(); 3]);
    }

    #[test]
    fn a_caret_has_a_sprite() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        assert_eq!(sprites(&mut r, &[caret_placement(1, 1, 1, 1)]).len(), 1);
    }

    #[test]
    fn a_box_off_screen_has_no_sprite() {
        let mut r = renderer_on(window(5, 20, 4, 4));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 10, 0, 4, 3)],
        );
        assert!(images.is_empty());
    }

    #[test]
    fn a_box_overhanging_the_left_is_cropped() {
        let mut r = renderer_on(window(40, 20, 4, 4));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), -2, 1, 5, 3)],
        );
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].col, 0);
        assert_eq!(images[0].canvas().width, 3 * 4);
    }

    #[test]
    fn a_box_overhanging_the_top_is_cropped() {
        let mut r = renderer_on(window(40, 20, 4, 4));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 1, -2, 4, 5)],
        );
        assert_eq!(images[0].row, 0);
        assert_eq!(images[0].canvas().height, 3 * 4);
    }

    #[test]
    fn a_box_overhanging_the_right_is_cropped() {
        let mut r = renderer_on(window(4, 20, 4, 4));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 1, 0, 6, 3)],
        );
        assert_eq!(images[0].col, 1);
        assert_eq!(images[0].canvas().width, 3 * 4);
    }

    #[test]
    fn a_box_overhanging_the_bottom_is_cropped() {
        let mut r = renderer_on(window(40, 4, 4, 4));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 1, 3, 6)],
        );
        assert_eq!(images[0].row, 1);
        assert_eq!(images[0].canvas().height, 3 * 4);
    }

    #[test]
    fn a_box_sprite_sits_at_the_placement_cell() {
        let mut r = renderer_on(window(40, 20, 6, 12));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 1, 2, 4, 3)],
        );
        let boxed = composed(&images, depth_z(0), r.window);
        assert_eq!((boxed.col, boxed.row), (1, 2));
        assert_eq!((boxed.canvas.width, boxed.canvas.height), (24, 36));
    }

    #[test]
    fn a_border_takes_the_colour_of_its_palette_index() {
        for index in (0..).take_while(|&i| palette(i).is_some()) {
            let mut r = renderer_on(window(40, 20, 2, 4));
            let images = sprites(
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
            assert_eq!(&images[0].canvas().pixels[0..4], &[px, py, pz, OPAQUE]);
        }
    }

    #[test]
    fn an_unchanged_box_is_not_redrawn() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let node = box_node(Some(1), Some(1), false);
        let placement = box_placement(&node, 0, 0, 4, 3);
        let first = sprites(&mut r, std::slice::from_ref(&placement));
        assert_eq!(r.cache.len(), 1);
        let second = sprites(&mut r, &[placement]);
        assert_eq!(first[0].canvas().pixels, second[0].canvas().pixels);
        assert_eq!(r.cache.len(), 1);
    }

    #[test]
    fn a_recoloured_box_is_redrawn() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let plain = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 0, 4, 3)],
        );
        let blue = sprites(
            &mut r,
            &[box_placement(&box_node(Some(4), None, false), 0, 0, 4, 3)],
        );
        assert_ne!(plain[0].canvas().pixels, blue[0].canvas().pixels);
        assert_eq!(r.cache.len(), 2);
    }

    #[test]
    fn rounded_and_square_are_cached_distinctly() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        for rounded in [false, true] {
            sprites(
                &mut r,
                &[box_placement(&box_node(None, None, rounded), 0, 0, 4, 3)],
            );
        }
        assert_eq!(r.cache.len(), 2);
    }

    #[test]
    fn a_relabelled_box_of_the_same_size_reuses_its_pixels() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let first = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 0, 4, 3)],
        );
        let second = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 0, 4, 3)],
        );
        assert_eq!(first[0].canvas().pixels, second[0].canvas().pixels);
        assert_eq!(r.cache.len(), 1);
    }

    #[test]
    fn a_cached_sprite_moves_to_its_own_position() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 0, 4, 3)],
        );
        let moved = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 5, 2, 4, 3)],
        );
        assert_eq!((moved[0].col, moved[0].row), (5, 2));
    }

    #[test]
    fn a_moved_box_reuses_its_cached_pixels() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let first = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 0, 4, 3)],
        );
        let moved = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 5, 2, 4, 3)],
        );
        assert_eq!(first[0].canvas().pixels, moved[0].canvas().pixels);
        assert_eq!(r.cache.len(), 1);
    }

    #[test]
    fn a_differently_cropped_box_shares_one_cache_entry() {
        let mut r = renderer_on(window(4, 20, 2, 4));
        let whole = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 0, 4, 3)],
        );
        let cropped = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 2, 0, 4, 3)],
        );
        assert_ne!(whole[0].canvas().width, cropped[0].canvas().width);
        assert_eq!(r.cache.len(), 1);
    }

    #[test]
    fn a_shape_beyond_the_screen_leaves_the_cache_empty() {
        let mut r = renderer_on(window(4, 4, 2, 4));
        sprites(
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
        let frame = Frame::new(window(20, 10, 4, 8));
        assert!(!frame.shows(&box_placement(&node, 20, 0, 4, 3), whole(frame.window)));
    }

    #[test]
    fn a_box_beyond_the_top_edge_is_not_shown() {
        let node = box_node(None, None, false);
        let frame = Frame::new(window(20, 10, 4, 8));
        assert!(!frame.shows(&box_placement(&node, 0, -3, 4, 3), whole(frame.window)));
    }

    #[test]
    fn a_box_straddling_an_edge_is_shown() {
        let node = box_node(None, None, false);
        let frame = Frame::new(window(20, 10, 4, 8));
        assert!(frame.shows(&box_placement(&node, 18, 0, 4, 3), whole(frame.window)));
        assert!(frame.shows(&box_placement(&node, -2, 8, 4, 3), whole(frame.window)));
    }

    #[test]
    fn a_block_poking_out_of_the_placement_is_cropped_to_the_screen() {
        let frame = Frame::new(window(20, 10, 4, 8));
        let area = whole(frame.window);
        let poking = Geometry {
            x: -1,
            y: -1,
            width: 2,
            height: 2,
        };
        let crop = frame.crop(poking, area).unwrap();
        assert_eq!((crop.col, crop.row), (0, 0));
        assert_eq!((crop.first_x, crop.first_y), (4, 8));
        assert_eq!((crop.last_x, crop.last_y), (8, 16));
        let outside = Geometry {
            x: -2,
            y: 0,
            width: 2,
            height: 2,
        };
        assert!(!frame.shows(outside, area));
    }

    #[test]
    fn a_block_beyond_the_area_is_clipped_to_the_area() {
        let frame = Frame::new(window(20, 10, 4, 8));
        let area = Area {
            col: 2,
            row: 2,
            cols: 3,
            rows: 3,
        };
        let block = Geometry {
            x: 4,
            y: 0,
            width: 4,
            height: 4,
        };
        let crop = frame.crop(block, area).unwrap();
        assert_eq!((crop.col, crop.row), (4, 2));
        assert_eq!((crop.first_x, crop.last_x), (0, 4));
        assert_eq!((crop.first_y, crop.last_y), (16, 32));
    }

    #[test]
    fn arrows_with_different_stops_are_redrawn() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let one = sprites(&mut r, &[arrow_placement(vec![0], 0, 0, 0, 4, 6)]);
        let two = sprites(&mut r, &[arrow_placement(vec![0, 2], 0, 0, 0, 4, 6)]);
        assert_ne!(one[0].canvas().pixels, two[0].canvas().pixels);
    }

    #[test]
    fn the_cache_is_bounded() {
        let limit = 2;
        let source = Box::new(FakeGlyphSource::new(1, 1));
        let mut r = TerminalRenderer::new(window(20, 5, 1, 1), source, limit);
        for width in 0..(limit as i64 + 2) {
            sprites(
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
            },
        )
    }

    fn pixel_of(sprite: &Canvas, x: i64, y: i64) -> (u8, u8, u8, u8) {
        pixel_at(&sprite.pixels, sprite.width, x, y)
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
        let drawn = &frame.images[0];
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
        let body_images: Vec<&Placed> = frame
            .images
            .iter()
            .filter(|image| image.row < body_rows)
            .collect();
        assert!(!body_images.is_empty());
        for image in body_images {
            assert!(image.row + image.canvas().height / window.cell_height <= body_rows);
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
                ..
            } => BoxStyle {
                colour: *colour,
                fill: *fill,
                fill_alpha: quantized_alpha(*opacity),
                solid_fill: *solid_fill,
                rounded: *rounded,
                sides: *sides,
                border: *border,
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

    fn image_ids(output: &str) -> std::collections::HashSet<String> {
        shown_ids(output)
            .into_iter()
            .chain(placed_ids(output))
            .collect()
    }

    fn cells_of(placement: &Placement) -> Vec<(i64, i64)> {
        let geometry = Geometry::from(placement);
        (geometry.y..geometry.y + geometry.height)
            .flat_map(|row| (geometry.x..geometry.x + geometry.width).map(move |col| (col, row)))
            .collect()
    }

    #[test]
    fn widening_a_tiled_box_transmits_no_tile_and_places_one_more_per_tiled_row() {
        let mut r = renderer_on(tiled_window());
        let colour = Some(1);
        let (width, height) = smallest_tiled_selected_box(&r, colour);
        let narrow = selected_box(colour, BRACKET_MARGIN, BRACKET_MARGIN, width, height);
        let wide = selected_box(colour, BRACKET_MARGIN, BRACKET_MARGIN, width + 1, height);

        let first = rendered_placements(&mut r, &narrow);
        let second = rendered_placements(&mut r, &wide);

        let tiled_rows = narrow[0].height;
        let first_placements = shown_ids(&first).len() + placed_ids(&first).len();
        assert!(shown_ids(&second).is_empty());
        assert!(deleted_ids(&second).is_empty());
        assert_eq!(
            placed_ids(&second).len(),
            first_placements + tiled_rows as usize
        );
    }

    #[test]
    fn tiled_boxes_of_one_style_and_different_sizes_share_every_tile_image() {
        let mut r = renderer_on(tiled_window());
        let colour = Some(1);
        let (width, height) = smallest_tiled_selected_box(&r, colour);

        let small = rendered_placements(
            &mut r,
            &selected_box(colour, BRACKET_MARGIN, BRACKET_MARGIN, width, height),
        );
        let large = rendered_placements(
            &mut r,
            &selected_box(
                colour,
                BRACKET_MARGIN,
                BRACKET_MARGIN,
                width + EXTRA_CELLS,
                height + 1,
            ),
        );

        assert_eq!(image_ids(&small), image_ids(&large));
    }

    #[test]
    fn tiled_boxes_of_different_styles_share_no_tile_image() {
        let mut r = renderer_on(tiled_window());
        let (width, height) = smallest_tiled_selected_box(&r, Some(1));

        let one = rendered_placements(
            &mut r,
            &selected_box(Some(1), BRACKET_MARGIN, BRACKET_MARGIN, width, height)[..1],
        );
        let other = rendered_placements(
            &mut r,
            &selected_box(Some(2), BRACKET_MARGIN, BRACKET_MARGIN, width, height)[..1],
        );

        assert!(image_ids(&one).is_disjoint(&image_ids(&other)));
    }

    #[test]
    fn a_partly_clipped_tiled_box_shows_only_its_whole_tiles_inside_the_area() {
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
        let mut frame = Frame::new(r.window);

        r.paint(&mut frame, std::slice::from_ref(&placement), area);

        let inside = |&(col, row): &(i64, i64)| {
            (area.col..area.col + area.cols).contains(&col)
                && (area.row..area.row + area.rows).contains(&row)
        };
        let expected: Vec<(i64, i64)> = cells_of(&placement).into_iter().filter(inside).collect();
        let positions: Vec<(i64, i64)> = frame
            .images
            .iter()
            .map(|image| (image.col, image.row))
            .collect();
        assert_eq!(positions, expected);
        for image in &frame.images {
            if let Image::Fresh { canvas, .. } = &image.image {
                assert_eq!(
                    (canvas.width, canvas.height),
                    (r.window.cell_width, r.window.cell_height)
                );
            }
        }
    }

    #[test]
    fn tiles_keep_their_sprites_z_and_are_queued_row_major_in_scene_order() {
        let mut r = renderer_on(tiled_window());
        let colour = Some(1);
        let (width, height) = smallest_tiled_selected_box(&r, colour);
        let [boxed, brackets] = selected_box(colour, BRACKET_MARGIN, BRACKET_MARGIN, width, height);

        let images = sprites(&mut r, &[boxed.clone(), brackets.clone()]);

        let queued: Vec<(i64, i64, i32)> = images
            .iter()
            .map(|image| (image.col, image.row, image.z))
            .collect();
        let expected: Vec<(i64, i64, i32)> = cells_of(&boxed)
            .into_iter()
            .map(|(col, row)| (col, row, depth_z(0)))
            .chain(CORNERS.into_iter().map(|corner| {
                let (col, row) =
                    corner_offset(corner, brackets.width, brackets.height, r.cell_size());
                (brackets.x + col, brackets.y + row, BRACKETS_Z)
            }))
            .collect();
        assert_eq!(queued, expected);
    }

    #[test]
    fn a_box_too_small_to_tile_is_one_transient_whole_sprite() {
        let mut r = renderer_on(tiled_window());
        let node = outlined_node(Some(1));
        let (width, height) = smallest_tiled(&r, &node);
        let placements = [box_placement(&node, 0, 0, width - 1, height)];

        let images = sprites(&mut r, &placements);
        let first = rendered_placements(&mut r, &placements);
        let second = rendered_placements(&mut r, &placements);

        let [image] = images.as_slice() else {
            panic!("expected a single whole sprite");
        };
        assert_eq!(
            (image.canvas().width, image.canvas().height),
            (
                (width - 1) * r.window.cell_width,
                height * r.window.cell_height
            )
        );
        assert_eq!(shown_ids(&first).len(), 1);
        assert!(placed_ids(&second).is_empty());
        assert_eq!(deleted_ids(&second), shown_ids(&first));
    }

    #[test]
    fn resizing_the_cell_size_produces_new_tile_keys_and_fresh_transmissions() {
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

        let before = rendered_placements(&mut r, &placements);
        let keys_before = r.tile_images.len();
        r.window.cell_width = 2 * old_cell_width;
        r.window.cell_height = 2 * old_cell_height;
        let after = rendered_placements(&mut r, &placements);

        let shown_after: std::collections::HashSet<String> =
            shown_ids(&after).into_iter().collect();
        assert!(!shown_after.is_empty());
        assert!(placed_ids(&after).iter().all(|id| shown_after.contains(id)));
        assert!(image_ids(&before).is_disjoint(&image_ids(&after)));
        assert!(r.tile_images.len() > keys_before);
        assert!(r
            .tile_images
            .keys()
            .any(|key| key.shape.cell == r.cell_size()));
    }
}
