use std::io::{self, Write};

use super::font::GlyphSource;
use super::shapes::{ArrowShape, BoxShape};
use super::{colour, editor, Renderer, ARROW_OPACITY, BORDER, FILL_ALPHA, OPAQUE, ROUNDED_RADIUS};
use crate::canvas::Canvas;
use crate::composer::Area;
use crate::kitty;
use crate::layout::{Label, Placement, PlacementNode};
use crate::palette::palette;
use crate::state::State;
use crate::tty::Window;

const BLANK: char = ' ';
const HOME_CURSOR: &str = "\x1b[H";

pub(super) const ARROW_STROKE: i64 = 3;

pub(crate) const CACHE_LIMIT: usize = 512;

const TRANSPARENT: (u8, u8, u8, u8) = (0, 0, 0, 0);

pub(super) fn centered_span(c: i64, width: i64) -> std::ops::Range<i64> {
    let start = c - (width - 1).div_euclid(2);
    start..(start + width)
}

fn fill_colour(fill: Option<u8>) -> (u8, u8, u8, u8) {
    match fill {
        None => TRANSPARENT,
        Some(colour) => {
            let (r, g, b) = palette(colour).unwrap();
            let composite =
                |channel: u8| (channel as f64 * FILL_ALPHA as f64 / OPAQUE as f64).round() as u8;
            (composite(r), composite(g), composite(b), OPAQUE)
        }
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
        colour: Option<u8>,
        fill: Option<u8>,
        rounded: bool,
    },
    Arrow {
        width: i64,
        height: i64,
        stops: Vec<i64>,
        shaft: i64,
    },
}

fn sprite_key(placement: &Placement) -> SpriteKey {
    match &placement.node {
        PlacementNode::Box {
            colour,
            fill,
            rounded,
        } => SpriteKey::Box {
            width: placement.width,
            height: placement.height,
            colour: *colour,
            fill: *fill,
            rounded: *rounded,
        },
        PlacementNode::Arrow(arrow) => SpriteKey::Arrow {
            width: placement.width,
            height: placement.height,
            stops: arrow.stops.clone(),
            shaft: arrow.shaft,
        },
        _ => unreachable!("sprite_key is only called for Box and Arrow placements"),
    }
}

struct Placed {
    canvas: Canvas,
    col: i64,
    row: i64,
}

struct Frame {
    window: Window,
    characters: Vec<Vec<char>>,
    images: Vec<Placed>,
}

impl Frame {
    fn new(window: Window) -> Self {
        Frame {
            window,
            characters: vec![vec![BLANK; window.cols as usize]; window.rows as usize],
            images: Vec::new(),
        }
    }

    // Clipping happens by cropping: kitty::show cannot position at a negative
    // column, and sends a=T without C=1, so an overhang would shift into view
    // or scroll the screen instead of being cut off.
    fn crop(&self, placement: &Placement, area: Area) -> Option<Crop> {
        let (left, top) = (placement.x, placement.y);
        let col = left.max(area.col).max(0);
        let row = top.max(area.row).max(0);
        let right = (left + placement.width)
            .min(area.col + area.cols)
            .min(self.window.cols);
        let bottom = (top + placement.height)
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

    fn shows(&self, placement: &Placement, area: Area) -> bool {
        self.crop(placement, area).is_some()
    }

    fn place(&mut self, canvas: &Canvas, placement: &Placement, area: Area) {
        let Some(crop) = self.crop(placement, area) else {
            return;
        };
        self.images.push(Placed {
            canvas: canvas.crop(crop.first_x, crop.last_x, crop.first_y, crop.last_y),
            col: crop.col,
            row: crop.row,
        });
    }

    fn into_bytes(self) -> Vec<u8> {
        let rows: Vec<String> = self
            .characters
            .into_iter()
            .map(|row| row.into_iter().collect())
            .collect();
        let mut bytes = HOME_CURSOR.as_bytes().to_vec();
        bytes.extend_from_slice(rows.join("\r\n").as_bytes());
        bytes.extend_from_slice(kitty::clear().to_string().as_bytes());
        for image in &self.images {
            bytes.extend_from_slice(
                kitty::show(&image.canvas, image.col, image.row)
                    .to_string()
                    .as_bytes(),
            );
        }
        bytes
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
}

impl Renderer for TerminalRenderer {
    fn render(&mut self, state: &State, out: &mut impl Write) -> io::Result<()> {
        let frame = self.frame(state);
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
        }
    }

    pub(crate) fn on_resize(&mut self, window: Window) {
        self.window.cols = window.cols;
        self.window.rows = window.rows;
    }

    fn frame(&mut self, state: &State) -> Frame {
        let mut frame = Frame::new(self.window);
        for (area, placements) in editor(state, whole(self.window)) {
            self.paint(&mut frame, &placements, area);
        }
        frame
    }

    fn paint(&mut self, frame: &mut Frame, placements: &[Placement], area: Area) {
        for placement in placements {
            match &placement.node {
                PlacementNode::Box { .. } => self.draw_box(frame, placement, area),
                PlacementNode::Arrow(_) => self.draw_arrow(frame, placement, area),
                PlacementNode::Label(label) => self.draw_label(frame, placement, label, area),
                PlacementNode::Cursor(_) => self.draw_cursor(frame, placement, area),
            }
        }
    }

    fn draw_box(&mut self, frame: &mut Frame, placement: &Placement, area: Area) {
        if !frame.shows(placement, area) {
            return;
        }
        let key = sprite_key(placement);
        if !self.cache.contains_key(&key) {
            let drawn = self.outline_box(placement);
            self.remember(key.clone(), drawn);
        }
        frame.place(&self.cache[&key], placement, area);
    }

    fn draw_arrow(&mut self, frame: &mut Frame, placement: &Placement, area: Area) {
        if !frame.shows(placement, area) {
            return;
        }
        let key = sprite_key(placement);
        if !self.cache.contains_key(&key) {
            let drawn = self.outline_arrow(placement);
            self.remember(key.clone(), drawn);
        }
        frame.place(&self.cache[&key], placement, area);
    }

    fn draw_label(&mut self, frame: &mut Frame, placement: &Placement, label: &Label, area: Area) {
        for (offset, character) in label.text.chars().enumerate() {
            let char_placement = Placement {
                node: PlacementNode::Label(label.clone()),
                x: placement.x + offset as i64,
                y: placement.y,
                width: 1,
                height: 1,
            };
            if !frame.shows(&char_placement, area) {
                continue;
            }
            let glyph = self.glyph_source.glyph(character);
            frame.place(glyph, &char_placement, area);
        }
    }

    fn draw_cursor(&mut self, frame: &mut Frame, placement: &Placement, area: Area) {
        let width = self.cells_to_pixels_x(placement.width);
        let height = self.cells_to_pixels_y(placement.height);
        let (r, g, b) = colour(None);
        let canvas = Canvas::fill(
            width,
            height,
            &SolidShape {
                colour: [r, g, b, OPAQUE],
            },
        );
        frame.place(&canvas, placement, area);
    }

    fn remember(&mut self, key: SpriteKey, drawn: Canvas) {
        if self.cache.len() >= self.cache_limit {
            self.cache.clear();
        }
        self.cache.insert(key, drawn);
    }

    fn cells_to_pixels_x(&self, cells: i64) -> i64 {
        cells * self.window.cell_width
    }

    fn cells_to_pixels_y(&self, cells: i64) -> i64 {
        cells * self.window.cell_height
    }

    fn outline_box(&self, placement: &Placement) -> Canvas {
        let (edge, fill, rounded) = match &placement.node {
            PlacementNode::Box {
                colour,
                fill,
                rounded,
            } => (*colour, *fill, *rounded),
            _ => unreachable!("outline_box is only called for Box placements"),
        };
        let width = self.cells_to_pixels_x(placement.width);
        let height = self.cells_to_pixels_y(placement.height);
        let (r, g, b) = colour(edge);
        let (fill_r, fill_g, fill_b, fill_a) = fill_colour(fill);
        let shape = BoxShape {
            width,
            height,
            border: BORDER,
            radius: if rounded { ROUNDED_RADIUS } else { 0 },
            edge: [r, g, b, OPAQUE],
            fill: [fill_r, fill_g, fill_b, fill_a],
        };
        Canvas::fill(width, height, &shape)
    }

    fn outline_arrow(&self, placement: &Placement) -> Canvas {
        let arrow = match &placement.node {
            PlacementNode::Arrow(arrow) => arrow,
            _ => unreachable!("outline_arrow is only called for Arrow placements"),
        };
        let width = self.cells_to_pixels_x(placement.width);
        let height = self.cells_to_pixels_y(placement.height);
        let stop_rows: Vec<i64> = arrow
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
            shaft_row: self.cells_to_pixels_y(arrow.shaft) + self.window.cell_height / 2,
            trunk,
            stroke: ARROW_STROKE,
            ink: [r, g, b, (ARROW_OPACITY * OPAQUE as f64).round() as u8],
        };
        Canvas::fill(width, height, &shape)
    }
}

#[cfg(test)]
mod tests {
    use super::super::font::FakeGlyphSource;
    use super::*;
    use crate::layout::FOOTER_ROWS;
    use crate::state::Mode;

    #[test]
    fn fill_colour_of_plain_is_transparent() {
        assert_eq!(fill_colour(None), TRANSPARENT);
    }

    #[test]
    fn fill_colour_of_a_palette_index_is_alpha_composited_and_opaque() {
        let (r, g, b) = palette(2).unwrap();
        let round =
            |channel: u8| (channel as f64 * FILL_ALPHA as f64 / OPAQUE as f64).round() as u8;
        let expected = (round(r), round(g), round(b), OPAQUE);
        assert_eq!(fill_colour(Some(2)), expected);
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
        let fill = fill_colour(None);
        let pixels = box_pixels(size, size, 0, edge, fill);
        assert_eq!(pixel_at(&pixels, size, BORDER + 1, BORDER + 1), TRANSPARENT);
    }

    #[test]
    fn a_fill_colour_is_composited_over_black_and_made_opaque() {
        let size = 2 * BORDER + 3;
        let edge = edge_rgba(None);
        let fill = fill_colour(Some(2));
        let pixels = box_pixels(size, size, 0, edge, fill);
        assert_eq!(
            pixel_at(&pixels, size, BORDER + 1, BORDER + 1),
            fill_colour(Some(2))
        );
    }

    #[test]
    fn border_pixels_are_unaffected_by_fill() {
        let size = 2 * BORDER + 3;
        let edge = edge_rgba(Some(3));
        let fill = fill_colour(Some(2));
        let pixels = box_pixels(size, size, 0, edge, fill);
        assert_eq!(pixel_at(&pixels, size, 0, 0), edge_rgba(Some(3)));
        assert_eq!(
            pixel_at(&pixels, size, BORDER + 1, BORDER + 1),
            fill_colour(Some(2))
        );
    }

    #[test]
    fn a_border_is_bold_at_every_edge() {
        let size = 3 * 4;
        let edge = edge_rgba(Some(1));
        let fill = fill_colour(Some(2));
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
        let fill = fill_colour(Some(2));
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
        let fill = fill_colour(Some(2));
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
        let fill = fill_colour(Some(2));
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
        let fill = fill_colour(Some(2));
        let pixels = box_pixels(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, edge, fill);
        assert_eq!(
            pixel_at(&pixels, CORNER_SIZE, 20, 4),
            (55, 108, 108, OPAQUE)
        );
    }

    #[test]
    fn a_rounded_box_cuts_away_more_than_a_square_one() {
        let edge = edge_rgba(Some(1));
        let fill = fill_colour(Some(2));
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
        let fill = fill_colour(Some(2));
        let pixels = box_pixels(SMALL_SIZE, SMALL_SIZE, ROUNDED_RADIUS, edge, fill);
        assert_eq!(pixels.len() as i64, SMALL_SIZE * SMALL_SIZE * 4);
    }

    fn box_node(colour: Option<u8>, fill: Option<u8>, rounded: bool) -> PlacementNode<'static> {
        PlacementNode::Box {
            colour,
            fill,
            rounded,
        }
    }

    fn box_placement(
        node: &PlacementNode<'static>,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
    ) -> crate::layout::Placement<'static> {
        crate::layout::Placement {
            node: node.clone(),
            x,
            y,
            width,
            height,
        }
    }

    fn arrow_placement(
        stops: Vec<i64>,
        shaft: i64,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
    ) -> crate::layout::Placement<'static> {
        crate::layout::Placement {
            node: crate::layout::PlacementNode::Arrow(crate::layout::Arrow { stops, shaft }),
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn sprite_key_of_two_identically_shaped_boxes_is_equal() {
        let node_a = box_node(Some(1), Some(1), true);
        let node_b = box_node(Some(1), Some(1), true);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_eq!(sprite_key(&a), sprite_key(&b));
    }

    #[test]
    fn sprite_key_differs_by_colour() {
        let node_a = box_node(Some(1), None, true);
        let node_b = box_node(Some(2), None, true);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_ne!(sprite_key(&a), sprite_key(&b));
    }

    #[test]
    fn sprite_key_differs_by_fill() {
        let node_a = box_node(Some(1), Some(1), true);
        let node_b = box_node(Some(1), None, true);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_ne!(sprite_key(&a), sprite_key(&b));
    }

    #[test]
    fn sprite_key_differs_by_rounded() {
        let node_a = box_node(Some(1), Some(1), true);
        let node_b = box_node(Some(1), Some(1), false);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_ne!(sprite_key(&a), sprite_key(&b));
    }

    #[test]
    fn sprite_key_of_arrows_with_different_stops_differs() {
        let a = arrow_placement(vec![0, 2], 1, 0, 0, 4, 3);
        let b = arrow_placement(vec![0, 3], 1, 0, 0, 4, 3);
        assert_ne!(sprite_key(&a), sprite_key(&b));
    }

    #[test]
    fn sprite_key_of_identical_arrows_is_equal() {
        let a = arrow_placement(vec![0, 2], 1, 0, 0, 4, 3);
        let b = arrow_placement(vec![0, 2], 1, 0, 0, 4, 3);
        assert_eq!(sprite_key(&a), sprite_key(&b));
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

    fn lines_of(frame: &str) -> Vec<&str> {
        frame
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
    ) -> crate::layout::Placement<'_> {
        crate::layout::Placement {
            node: crate::layout::PlacementNode::Label(crate::layout::Label {
                text,
                path: vec![0],
            }),
            x,
            y,
            width,
            height,
        }
    }

    fn cursor_placement(
        x: i64,
        y: i64,
        width: i64,
        height: i64,
    ) -> crate::layout::Placement<'static> {
        crate::layout::Placement {
            node: crate::layout::PlacementNode::Cursor(crate::layout::Cursor),
            x,
            y,
            width,
            height,
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
            format!("{}{}", BLANK.to_string().repeat(3), kitty::clear())
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
        let expected: String = std::iter::once(kitty::clear())
            .chain(
                frame
                    .images
                    .iter()
                    .map(|image| kitty::show(&image.canvas, image.col, image.row)),
            )
            .map(|command| command.to_string())
            .collect();
        let bytes = String::from_utf8(frame.into_bytes()).unwrap();
        assert!(bytes.ends_with(&expected));
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
        r.frame(&state);
        let (cols, rows) = (40, 20);
        r.on_resize(window(cols, rows, 1, 1));
        let frame = r.frame(&state);
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
        r.render(state, &mut out).unwrap();
        String::from_utf8(out).unwrap()
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
            String::from_utf8(frame.into_bytes()).unwrap(),
            format!("{HOME_CURSOR}  \r\n  {}", kitty::clear())
        );
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
    fn the_output_is_sized_by_the_terminal() {
        let (cols, rows) = (5, 4);
        let frame = Frame::new(Window {
            cols,
            rows,
            cell_width: 1,
            cell_height: 1,
        });
        let output = String::from_utf8(frame.into_bytes()).unwrap();
        let body = output
            .strip_prefix(HOME_CURSOR)
            .unwrap()
            .strip_suffix(&kitty::clear().to_string())
            .unwrap();
        assert_eq!(
            body.split("\r\n").collect::<Vec<_>>(),
            vec![BLANK.to_string().repeat(cols as usize); rows as usize]
        );
    }

    #[test]
    fn the_cursor_is_drawn_last_as_a_solid_sprite() {
        let mut r = renderer_on(window(20, 10, 1, 1));
        let [box_at, label] = a_labelled_box(4, 3);
        let cursor = cursor_placement(label.x + label.width - 1, label.y, 1, 1);
        let images = sprites(&mut r, &[box_at, label, cursor]);
        let cursor_image = images.last().expect("a sprite is drawn for the cursor");
        let (cr, cg, cb) = colour(None);
        let solid = [cr, cg, cb, OPAQUE];
        assert!(cursor_image
            .canvas
            .pixels
            .chunks(4)
            .all(|pixel| pixel == solid));
    }

    #[test]
    fn an_empty_drawing_draws_nothing_without_a_file() {
        let mut r = renderer_on(window(40, 10, 1, 1));
        let frame = r.frame(&empty_state());
        assert!(frame.images.is_empty());
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

    #[test]
    fn cursor_is_drawn_after_the_label() {
        let node = box_node(None, None, false);
        let placements = vec![
            box_placement(&node, 0, 0, 5, 3),
            label_placement("hi", 1, 1, 2, 1),
            cursor_placement(3, 1, 1, 1),
        ];
        let mut r = renderer_on(window(5, 3, 1, 1));
        let frame = drawn_frame(&mut r, &placements);
        assert_eq!(rows(&frame)[1], "     ".to_string());
        let (cr, cg, cb) = colour(None);
        let solid = [cr, cg, cb, OPAQUE];
        let is_cursor = |image: &&Placed| image.canvas.pixels.chunks(4).all(|pixel| pixel == solid);
        let cursor_cols: Vec<i64> = frame
            .images
            .iter()
            .filter(|image| image.row == 1)
            .filter(is_cursor)
            .map(|image| image.col)
            .collect();
        assert_eq!(cursor_cols, vec![3]);
        let label_cols: Vec<i64> = frame
            .images
            .iter()
            .filter(|image| image.row == 1)
            .filter(|image| !is_cursor(image))
            .map(|image| image.col)
            .collect();
        assert_eq!(label_cols, vec![1, 2]);
    }

    #[test]
    fn label_and_cursor_past_the_edge_are_clipped() {
        let node = box_node(None, None, false);
        let placements = vec![
            box_placement(&node, 0, 0, 5, 3),
            label_placement("hi", 1, 1, 2, 1),
            cursor_placement(3, 1, 1, 1),
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
    fn a_box_does_not_draw_a_cursor() {
        let grid = grid(
            &mut renderer_on(window(5, 3, 1, 1)),
            &[box_placement(&box_node(None, None, false), 0, 0, 5, 3)],
        );
        assert!(!grid.join("").contains('\u{2588}'));
    }

    #[test]
    fn cursor_placement_is_drawn_at_its_own_position() {
        let mut r = renderer_on(window(4, 3, 1, 1));
        let images = sprites(&mut r, &[cursor_placement(2, 1, 1, 1)]);
        assert_eq!(images.len(), 1);
        assert_eq!((images[0].col, images[0].row), (2, 1));
        let (cr, cg, cb) = colour(None);
        assert_eq!(&images[0].canvas.pixels[0..4], &[cr, cg, cb, OPAQUE]);
    }

    #[test]
    fn a_cursor_outside_the_grid_is_clipped() {
        let mut r = renderer_on(window(4, 3, 1, 1));
        let images = sprites(&mut r, &[cursor_placement(9, 9, 1, 1)]);
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
            cursor_placement(3, 1, 1, 1),
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
    fn a_cursor_has_a_sprite() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        assert_eq!(sprites(&mut r, &[cursor_placement(1, 1, 1, 1)]).len(), 1);
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
        assert_eq!(images[0].canvas.width, 3 * 4);
    }

    #[test]
    fn a_box_overhanging_the_top_is_cropped() {
        let mut r = renderer_on(window(40, 20, 4, 4));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 1, -2, 4, 5)],
        );
        assert_eq!(images[0].row, 0);
        assert_eq!(images[0].canvas.height, 3 * 4);
    }

    #[test]
    fn a_box_overhanging_the_right_is_cropped() {
        let mut r = renderer_on(window(4, 20, 4, 4));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 1, 0, 6, 3)],
        );
        assert_eq!(images[0].col, 1);
        assert_eq!(images[0].canvas.width, 3 * 4);
    }

    #[test]
    fn a_box_overhanging_the_bottom_is_cropped() {
        let mut r = renderer_on(window(40, 4, 4, 4));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 0, 1, 3, 6)],
        );
        assert_eq!(images[0].row, 1);
        assert_eq!(images[0].canvas.height, 3 * 4);
    }

    #[test]
    fn a_box_sprite_sits_at_the_placement_cell() {
        let mut r = renderer_on(window(40, 20, 6, 12));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, None, false), 1, 2, 4, 3)],
        );
        assert_eq!((images[0].col, images[0].row), (1, 2));
        assert_eq!((images[0].canvas.width, images[0].canvas.height), (24, 36));
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
            assert_eq!(&images[0].canvas.pixels[0..4], &[px, py, pz, OPAQUE]);
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
        assert_eq!(first[0].canvas.pixels, second[0].canvas.pixels);
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
        assert_ne!(plain[0].canvas.pixels, blue[0].canvas.pixels);
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
        assert_eq!(first[0].canvas.pixels, second[0].canvas.pixels);
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
        assert_eq!(first[0].canvas.pixels, moved[0].canvas.pixels);
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
        assert_ne!(whole[0].canvas.width, cropped[0].canvas.width);
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
    fn arrows_with_different_stops_are_redrawn() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let one = sprites(&mut r, &[arrow_placement(vec![0], 0, 0, 0, 4, 6)]);
        let two = sprites(&mut r, &[arrow_placement(vec![0, 2], 0, 0, 0, 4, 6)]);
        assert_ne!(one[0].canvas.pixels, two[0].canvas.pixels);
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
        r.outline_box(&box_placement(node, 0, 0, width, height))
    }

    fn pixel_of(sprite: &Canvas, x: i64, y: i64) -> (u8, u8, u8, u8) {
        pixel_at(&sprite.pixels, sprite.width, x, y)
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
            fill_colour(Some(2))
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
        r.outline_arrow(&arrow_placement(stops, shaft, 0, 0, width, height))
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
        crate::layout::diagram(state.doc.tree())
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
        let frame = r.frame(&state);
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
        let frame = r.frame(&state);
        assert!(!frame.images.is_empty());
        for image in &frame.images {
            assert!(image.row + image.canvas.height / window.cell_height <= body_rows);
        }
    }
}
