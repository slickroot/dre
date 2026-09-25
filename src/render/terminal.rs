use std::io::{self, Write};

use super::font::GlyphSource;
use super::shapes::{ArrowShape, BoxShape};
use super::{colour, Renderer, ARROW_OPACITY, BORDER, FILL_ALPHA, OPAQUE, ROUNDED_RADIUS};
use crate::canvas::Canvas;
use crate::kitty;
use crate::layout::{with_cursor, Label, Placement, PlacementNode};
use crate::palette::{palette, BACKGROUND};
use crate::state::State;
use crate::status_line::{status_line, Segment, StatusLine};
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

fn fill_colour(colour: Option<u8>, filled: bool) -> (u8, u8, u8, u8) {
    match filled.then_some(colour).flatten() {
        None => TRANSPARENT,
        Some(colour) => {
            let (r, g, b) = palette(colour).unwrap();
            let composite =
                |channel: u8| (channel as f64 * FILL_ALPHA as f64 / OPAQUE as f64).round() as u8;
            (composite(r), composite(g), composite(b), OPAQUE)
        }
    }
}

fn composite(overlay: (u8, u8, u8, u8), backdrop: (u8, u8, u8)) -> (u8, u8, u8) {
    let (r, g, b, alpha) = overlay;
    let blend = |over: u8, under: u8| {
        let weight = alpha as f64 / OPAQUE as f64;
        (over as f64 * weight + under as f64 * (1.0 - weight)).round() as u8
    };
    (
        blend(r, backdrop.0),
        blend(g, backdrop.1),
        blend(b, backdrop.2),
    )
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
        PlacementNode::Node(node) => SpriteKey::Box {
            width: placement.width,
            height: placement.height,
            colour: node.colour,
            fill: if node.filled { node.colour } else { None },
            rounded: node.rounded,
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
    origin: (i64, i64),
    characters: Vec<Vec<char>>,
    images: Vec<Placed>,
}

impl Frame {
    fn new(window: Window) -> Self {
        Frame {
            window,
            origin: (0, 0),
            characters: vec![vec![BLANK; window.cols as usize]; window.rows as usize],
            images: Vec::new(),
        }
    }

    fn centre_on(&mut self, placements: &[Placement]) {
        if placements.is_empty() {
            self.origin = (0, 0);
            return;
        }
        let min_x = placements
            .iter()
            .map(|placement| placement.x)
            .min()
            .unwrap();
        let span = placements
            .iter()
            .map(|placement| placement.x + placement.width)
            .max()
            .unwrap()
            - min_x;
        let height = placements
            .iter()
            .map(|placement| placement.y + placement.height)
            .max()
            .unwrap();
        let horizontal = (self.window.cols - span).div_euclid(2);
        self.origin = (horizontal, (self.window.rows - height).div_euclid(2));
    }

    // Clipping happens by cropping: kitty::show cannot position at a negative
    // column, and sends a=T without C=1, so an overhang would shift into view
    // or scroll the screen instead of being cut off.
    fn crop(&self, placement: &Placement) -> Option<Crop> {
        let left = placement.x + self.origin.0;
        let top = placement.y + self.origin.1;
        let col = left.max(0);
        let row = top.max(0);
        let right = (left + placement.width).min(self.window.cols);
        let bottom = (top + placement.height).min(self.window.rows);
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

    fn shows(&self, placement: &Placement) -> bool {
        let left = placement.x + self.origin.0;
        let top = placement.y + self.origin.1;
        left.max(0) < (left + placement.width).min(self.window.cols)
            && top.max(0) < (top + placement.height).min(self.window.rows)
    }

    fn place(&mut self, canvas: &Canvas, placement: &Placement) {
        let Some(crop) = self.crop(placement) else {
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
        self.render_diagram(state, out)?;
        self.render_status_line(state, out)
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

    fn render_diagram(&mut self, state: &State, out: &mut impl Write) -> io::Result<()> {
        let placements = with_cursor(
            crate::layout::layout(state.doc.tree()),
            state.selected.clone(),
        );
        let mut frame = Frame::new(self.window);
        frame.centre_on(&placements);
        for placement in &placements {
            match &placement.node {
                PlacementNode::Node(_) => self.draw_box(&mut frame, placement),
                PlacementNode::Arrow(_) => self.draw_arrow(&mut frame, placement),
                PlacementNode::Label(label) => self.draw_label(&mut frame, placement, label),
                PlacementNode::Cursor(_) => self.draw_cursor(&mut frame, placement),
            }
        }
        out.write_all(&frame.into_bytes())
    }

    fn render_status_line(&mut self, state: &State, out: &mut impl Write) -> io::Result<()> {
        let StatusLine { left, right } = status_line(&state.status_input());
        let Window { cols, rows, .. } = self.window;
        let content_width: usize = left
            .iter()
            .chain(&right)
            .map(|segment| segment.text.chars().count())
            .sum();
        let filler = Segment {
            text: BLANK
                .to_string()
                .repeat((cols as usize).saturating_sub(content_width)),
            style: right
                .last()
                .map(|segment| segment.style)
                .unwrap_or_default(),
        };
        let backdrop = palette(BACKGROUND).unwrap();
        write!(out, "\x1b[{rows};1H")?;
        let mut remaining = cols as usize;
        for segment in left.iter().chain(std::iter::once(&filler)).chain(&right) {
            let text: String = segment.text.chars().take(remaining).collect();
            remaining -= text.chars().count();
            if text.is_empty() {
                continue;
            }
            write!(out, "\x1b[0m")?;
            if segment.style.bold {
                write!(out, "\x1b[1m")?;
            }
            if let Some(overlay) = segment.style.background {
                let (r, g, b) = composite(overlay, backdrop);
                write!(out, "\x1b[48;2;{r};{g};{b}m")?;
            }
            if let Some((r, g, b)) = segment.style.foreground {
                write!(out, "\x1b[38;2;{r};{g};{b}m")?;
            }
            write!(out, "{text}")?;
        }
        write!(out, "\x1b[0m")
    }

    fn draw_box(&mut self, frame: &mut Frame, placement: &Placement) {
        if !frame.shows(placement) {
            return;
        }
        let key = sprite_key(placement);
        if !self.cache.contains_key(&key) {
            let drawn = self.outline_box(placement);
            self.remember(key.clone(), drawn);
        }
        frame.place(&self.cache[&key], placement);
    }

    fn draw_arrow(&mut self, frame: &mut Frame, placement: &Placement) {
        if !frame.shows(placement) {
            return;
        }
        let key = sprite_key(placement);
        if !self.cache.contains_key(&key) {
            let drawn = self.outline_arrow(placement);
            self.remember(key.clone(), drawn);
        }
        frame.place(&self.cache[&key], placement);
    }

    fn draw_label(&mut self, frame: &mut Frame, placement: &Placement, label: &Label) {
        for (offset, character) in label.text.chars().enumerate() {
            let char_placement = Placement {
                node: PlacementNode::Label(label.clone()),
                x: placement.x + offset as i64,
                y: placement.y,
                width: 1,
                height: 1,
            };
            if !frame.shows(&char_placement) {
                continue;
            }
            let glyph = self.glyph_source.glyph(character);
            frame.place(glyph, &char_placement);
        }
    }

    fn draw_cursor(&mut self, frame: &mut Frame, placement: &Placement) {
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
        frame.place(&canvas, placement);
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
        let node = match &placement.node {
            PlacementNode::Node(node) => node,
            _ => unreachable!("outline_box is only called for Box placements"),
        };
        let width = self.cells_to_pixels_x(placement.width);
        let height = self.cells_to_pixels_y(placement.height);
        let (r, g, b) = colour(node.colour);
        let (fill_r, fill_g, fill_b, fill_a) = fill_colour(node.colour, node.filled);
        let shape = BoxShape {
            width,
            height,
            border: BORDER,
            radius: if node.rounded { ROUNDED_RADIUS } else { 0 },
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
    use crate::diagram::{labelled, node, node_with_children, Document};
    use crate::state::Mode;
    use types::Tree;

    #[test]
    fn fill_colour_of_plain_is_transparent() {
        assert_eq!(fill_colour(None, false), TRANSPARENT);
        assert_eq!(fill_colour(None, true), TRANSPARENT);
    }

    #[test]
    fn fill_colour_of_a_palette_index_is_alpha_composited_and_opaque() {
        let (r, g, b) = palette(2).unwrap();
        let round =
            |channel: u8| (channel as f64 * FILL_ALPHA as f64 / OPAQUE as f64).round() as u8;
        let expected = (round(r), round(g), round(b), OPAQUE);
        assert_eq!(fill_colour(Some(2), true), expected);
    }

    #[test]
    fn coloured_but_not_filled_is_transparent() {
        assert_eq!(fill_colour(Some(2), false), TRANSPARENT);
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
        let fill = fill_colour(None, false);
        let pixels = box_pixels(size, size, 0, edge, fill);
        assert_eq!(pixel_at(&pixels, size, BORDER + 1, BORDER + 1), TRANSPARENT);
    }

    #[test]
    fn a_fill_colour_is_composited_over_black_and_made_opaque() {
        let size = 2 * BORDER + 3;
        let edge = edge_rgba(None);
        let fill = fill_colour(Some(2), true);
        let pixels = box_pixels(size, size, 0, edge, fill);
        assert_eq!(
            pixel_at(&pixels, size, BORDER + 1, BORDER + 1),
            fill_colour(Some(2), true)
        );
    }

    #[test]
    fn border_pixels_are_unaffected_by_fill() {
        let size = 2 * BORDER + 3;
        let edge = edge_rgba(Some(3));
        let fill = fill_colour(Some(2), true);
        let pixels = box_pixels(size, size, 0, edge, fill);
        assert_eq!(pixel_at(&pixels, size, 0, 0), edge_rgba(Some(3)));
        assert_eq!(
            pixel_at(&pixels, size, BORDER + 1, BORDER + 1),
            fill_colour(Some(2), true)
        );
    }

    #[test]
    fn a_border_is_bold_at_every_edge() {
        let size = 3 * 4;
        let edge = edge_rgba(Some(1));
        let fill = fill_colour(Some(2), true);
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
        let fill = fill_colour(Some(2), true);
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
        let fill = fill_colour(Some(2), true);
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
        let fill = fill_colour(Some(2), true);
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
        let fill = fill_colour(Some(2), true);
        let pixels = box_pixels(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, edge, fill);
        assert_eq!(
            pixel_at(&pixels, CORNER_SIZE, 20, 4),
            (55, 108, 108, OPAQUE)
        );
    }

    #[test]
    fn a_rounded_box_cuts_away_more_than_a_square_one() {
        let edge = edge_rgba(Some(1));
        let fill = fill_colour(Some(2), true);
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
        let fill = fill_colour(Some(2), true);
        let pixels = box_pixels(SMALL_SIZE, SMALL_SIZE, ROUNDED_RADIUS, edge, fill);
        assert_eq!(pixels.len() as i64, SMALL_SIZE * SMALL_SIZE * 4);
    }

    fn box_node(colour: Option<u8>, filled: bool, rounded: bool) -> crate::diagram::Node {
        crate::diagram::Node {
            label: String::new(),
            colour,
            filled,
            rounded,
        }
    }

    fn box_placement(
        node: &crate::diagram::Node,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
    ) -> crate::layout::Placement<'_> {
        crate::layout::Placement {
            node: crate::layout::PlacementNode::Node(node),
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
        let node_a = box_node(Some(1), true, true);
        let node_b = box_node(Some(1), true, true);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_eq!(sprite_key(&a), sprite_key(&b));
    }

    #[test]
    fn sprite_key_differs_by_colour() {
        let node_a = box_node(Some(1), true, true);
        let node_b = box_node(Some(2), true, true);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_ne!(sprite_key(&a), sprite_key(&b));
    }

    #[test]
    fn sprite_key_differs_by_fill() {
        let node_a = box_node(Some(1), true, true);
        let node_b = box_node(Some(1), false, true);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_ne!(sprite_key(&a), sprite_key(&b));
    }

    #[test]
    fn sprite_key_differs_by_rounded() {
        let node_a = box_node(Some(1), true, true);
        let node_b = box_node(Some(1), true, false);
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

    fn draw_all(r: &mut TerminalRenderer, frame: &mut Frame, placements: &[Placement]) {
        for placement in placements {
            match &placement.node {
                PlacementNode::Node(_) => r.draw_box(frame, placement),
                PlacementNode::Arrow(_) => r.draw_arrow(frame, placement),
                PlacementNode::Label(label) => r.draw_label(frame, placement, label),
                PlacementNode::Cursor(_) => r.draw_cursor(frame, placement),
            }
        }
    }

    fn drawn_frame(r: &mut TerminalRenderer, placements: &[Placement]) -> Frame {
        let mut frame = Frame::new(r.window);
        draw_all(r, &mut frame, placements);
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
    fn centre_on_puts_the_diagram_in_the_middle_of_the_terminal() {
        let node = box_node(None, false, false);
        let (cols, rows) = (20, 10);
        let (width, height) = (6, 4);
        let mut frame = Frame::new(window(cols, rows, 1, 1));
        frame.centre_on(&[box_placement(&node, 0, 0, width, height)]);
        assert_eq!(
            frame.origin,
            ((cols - width).div_euclid(2), (rows - height).div_euclid(2))
        );
    }

    #[test]
    fn centre_on_measures_the_whole_bounding_box() {
        let node = box_node(None, false, false);
        let (cols, rows) = (20, 10);
        let placements = vec![
            box_placement(&node, 0, 0, 3, 2),
            box_placement(&node, 5, 4, 3, 2),
        ];
        let mut frame = Frame::new(window(cols, rows, 1, 1));
        frame.centre_on(&placements);
        assert_eq!(
            frame.origin,
            ((cols - 8).div_euclid(2), (rows - 6).div_euclid(2))
        );
    }

    #[test]
    fn centre_on_nothing_leaves_the_origin_at_the_corner() {
        let mut frame = Frame::new(window(20, 10, 1, 1));
        frame.centre_on(&[]);
        assert_eq!(frame.origin, (0, 0));
    }

    #[test]
    fn centre_on_centres_an_overflowing_diagram_with_a_negative_origin() {
        let node = box_node(None, false, false);
        let (cols, rows) = (10, 10);
        let (span, height) = (30, 4);
        let mut frame = Frame::new(window(cols, rows, 1, 1));
        frame.centre_on(&[box_placement(&node, 0, 0, span, height)]);
        assert_eq!(
            frame.origin,
            ((cols - span).div_euclid(2), (rows - height).div_euclid(2))
        );
        assert!(frame.origin.0 < 0);
    }

    #[test]
    fn centre_on_cuts_the_extra_column_of_an_odd_overflow_on_the_left() {
        let node = box_node(None, false, false);
        let (cols, rows) = (10, 10);
        let span = cols + 3;
        let mut frame = Frame::new(window(cols, rows, 1, 1));
        frame.centre_on(&[box_placement(&node, 0, 0, span, 4)]);
        let cut_on_left = -frame.origin.0;
        let cut_on_right = span - cols - cut_on_left;
        assert_eq!(cut_on_left, cut_on_right + 1);
    }

    #[test]
    fn centre_on_centres_a_diagram_that_fits() {
        let node = box_node(None, false, false);
        let (cols, rows) = (20, 10);
        let (width, height) = (6, 4);
        let mut frame = Frame::new(window(cols, rows, 1, 1));
        frame.centre_on(&[box_placement(&node, 0, 0, width, height)]);
        assert_eq!(
            frame.origin,
            ((cols - width).div_euclid(2), (rows - height).div_euclid(2))
        );
    }

    #[test]
    fn a_box_on_screen_is_cropped_to_the_whole_shape() {
        let node = box_node(None, false, false);
        let window = window(20, 10, 4, 8);
        let (width, height) = (5, 4);
        let frame = Frame::new(window);
        assert_eq!(
            frame.crop(&box_placement(&node, 2, 3, width, height)),
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
    fn a_crop_sits_at_the_placement_shifted_by_the_origin() {
        let node = box_node(None, false, false);
        let mut frame = Frame::new(window(20, 10, 4, 8));
        frame.centre_on(&[box_placement(&node, 0, 0, 6, 4)]);
        let crop = frame.crop(&box_placement(&node, 1, 1, 3, 2)).unwrap();
        assert_eq!(
            (crop.col, crop.row),
            (1 + frame.origin.0, 1 + frame.origin.1)
        );
    }

    #[test]
    fn a_crop_overhanging_the_left_drops_the_hidden_columns() {
        let node = box_node(None, false, false);
        let window = window(20, 10, 4, 8);
        let (hidden, width) = (2, 5);
        let frame = Frame::new(window);
        let crop = frame
            .crop(&box_placement(&node, -hidden, 0, width, 3))
            .unwrap();
        assert_eq!(crop.col, 0);
        assert_eq!(crop.first_x, hidden * window.cell_width);
        assert_eq!(crop.last_x, width * window.cell_width);
    }

    #[test]
    fn a_crop_overhanging_the_top_drops_the_hidden_rows() {
        let node = box_node(None, false, false);
        let window = window(20, 10, 4, 8);
        let (hidden, height) = (2, 5);
        let frame = Frame::new(window);
        let crop = frame
            .crop(&box_placement(&node, 0, -hidden, 3, height))
            .unwrap();
        assert_eq!(crop.row, 0);
        assert_eq!(crop.first_y, hidden * window.cell_height);
        assert_eq!(crop.last_y, height * window.cell_height);
    }

    #[test]
    fn a_crop_overhanging_the_right_stops_at_the_last_column() {
        let node = box_node(None, false, false);
        let window = window(20, 10, 4, 8);
        let x = 18;
        let frame = Frame::new(window);
        let crop = frame.crop(&box_placement(&node, x, 0, 5, 3)).unwrap();
        assert_eq!(crop.col, x);
        assert_eq!(crop.first_x, 0);
        assert_eq!(crop.last_x, (window.cols - x) * window.cell_width);
    }

    #[test]
    fn a_crop_overhanging_the_bottom_stops_at_the_last_row() {
        let node = box_node(None, false, false);
        let window = window(20, 10, 4, 8);
        let y = 8;
        let frame = Frame::new(window);
        let crop = frame.crop(&box_placement(&node, 0, y, 3, 5)).unwrap();
        assert_eq!(crop.row, y);
        assert_eq!(crop.first_y, 0);
        assert_eq!(crop.last_y, (window.rows - y) * window.cell_height);
    }

    #[test]
    fn a_box_beyond_the_right_edge_has_no_crop() {
        let node = box_node(None, false, false);
        let frame = Frame::new(window(20, 10, 4, 8));
        assert_eq!(frame.crop(&box_placement(&node, 20, 0, 4, 3)), None);
    }

    #[test]
    fn a_box_beyond_the_top_edge_has_no_crop() {
        let node = box_node(None, false, false);
        let frame = Frame::new(window(20, 10, 4, 8));
        assert_eq!(frame.crop(&box_placement(&node, 0, -3, 4, 3)), None);
    }

    #[test]
    fn line_count_is_unchanged() {
        let mut r = renderer_on(window(3, 3, 2, 4));
        assert_eq!(lines_of(&rendered(&mut r, &empty_doc())).len(), 3);
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
        let nodes = Tree::root(vec![node_with_children(
            "parent",
            vec![node("a"), node("b")],
        )]);
        let placements = crate::layout::layout(&nodes);
        let cols = placements
            .iter()
            .map(|placement| placement.x + placement.width)
            .max()
            .unwrap();
        let rows = placements
            .iter()
            .map(|placement| placement.y + placement.height)
            .max()
            .unwrap();
        let window = window(cols, rows, 2, 4);
        let images = sprites(&mut renderer_on(window), &placements);
        assert!(images.len() > 1);
        let expected: String = std::iter::once(kitty::clear())
            .chain(
                images
                    .iter()
                    .map(|image| kitty::show(&image.canvas, image.col, image.row)),
            )
            .map(|command| command.to_string())
            .collect();
        let doc = Document {
            root: nodes.clone(),
        };
        assert!(rendered_diagram(&mut renderer_on(window), &doc).ends_with(&expected));
    }

    #[test]
    fn centres_a_leaf_box_within_the_terminal() {
        let leaf = labelled("hi");
        let nodes = Tree::root(vec![Tree::leaf(leaf.clone())]);
        let placements = crate::layout::layout(&nodes);
        let cols = 20;
        let rows_count = 10;
        let left = (cols - crate::layout::width(&leaf)).div_euclid(2);
        let top = (rows_count - crate::layout::BOX_HEIGHT).div_euclid(2);
        let mut r = renderer_on(window(cols, rows_count, 1, 1));
        let mut frame = Frame::new(r.window);
        frame.centre_on(&placements);
        draw_all(&mut r, &mut frame, &placements);
        let label_x = left + crate::layout::centre(crate::layout::width(&leaf), "hi");
        let label_row = top + crate::layout::BOX_HEIGHT / 2;
        let label_cols: Vec<i64> = frame
            .images
            .iter()
            .filter(|image| image.row == label_row)
            .map(|image| image.col)
            .collect();
        assert_eq!(label_cols, vec![label_x, label_x + 1]);
        let lines = rows(&frame);
        assert_eq!(lines[top as usize], " ".repeat(cols as usize));
        assert_eq!(
            lines[(top + crate::layout::BOX_HEIGHT) as usize],
            " ".repeat(cols as usize)
        );
    }

    #[test]
    fn an_overflowing_diagram_is_cropped_equally_on_both_sides() {
        let leaf = labelled("hi");
        let nodes = Tree::root(vec![Tree::leaf(leaf.clone())]);
        let placements = crate::layout::layout(&nodes);
        let width = crate::layout::width(&leaf);
        let cut_each_side = 1;
        let cols = width - 2 * cut_each_side;
        let mut r = renderer_on(window(cols, 10, 1, 1));
        let mut frame = Frame::new(r.window);
        frame.centre_on(&placements);
        draw_all(&mut r, &mut frame, &placements);
        let box_placement = &placements[0];
        let crop = frame.crop(box_placement).unwrap();
        let cut_on_left = crop.first_x;
        let cut_on_right = box_placement.width * frame.window.cell_width - crop.last_x;
        assert_eq!(cut_on_left, cut_each_side);
        assert_eq!(cut_on_right, cut_each_side);
    }

    #[test]
    fn centres_a_parent_and_children_as_a_group() {
        let nodes = Tree::root(vec![node_with_children(
            "parent",
            vec![node("a"), node("b")],
        )]);
        let placements = crate::layout::layout(&nodes);
        let cols = 40;
        let rows_count = 12;
        let span = crate::layout::width(nodes.value(&[0]))
            + crate::layout::GAP_WIDTH
            + crate::layout::width(nodes.value(&[0, 0]));
        let height =
            crate::layout::LEAF_STRIDE * crate::layout::HALF_PITCH + crate::layout::BOX_HEIGHT;
        let left = (cols - span).div_euclid(2);
        let top = (rows_count - height).div_euclid(2);

        let mut r = renderer_on(window(cols, rows_count, 1, 1));
        let mut frame = Frame::new(r.window);
        frame.centre_on(&placements);
        draw_all(&mut r, &mut frame, &placements);

        let child_base_x = crate::layout::width(nodes.value(&[0])) + crate::layout::GAP_WIDTH;
        let parent_label_x =
            left + crate::layout::centre(crate::layout::width(nodes.value(&[0])), "parent");
        let parent_label_row = top + crate::layout::HALF_PITCH + crate::layout::BOX_HEIGHT / 2;
        let child_a_label_x = left
            + child_base_x
            + crate::layout::centre(crate::layout::width(nodes.value(&[0, 0])), "a");
        let child_a_label_row = top + crate::layout::BOX_HEIGHT / 2;

        let is_glyph = |image: &&Placed| image.canvas.width == 1 && image.canvas.height == 1;
        let parent_label_cols: Vec<i64> = frame
            .images
            .iter()
            .filter(|image| image.row == parent_label_row)
            .filter(is_glyph)
            .map(|image| image.col)
            .collect();
        let expected_parent_cols: Vec<i64> =
            (parent_label_x..parent_label_x + "parent".len() as i64).collect();
        assert_eq!(parent_label_cols, expected_parent_cols);

        let child_a_label_cols: Vec<i64> = frame
            .images
            .iter()
            .filter(|image| image.row == child_a_label_row)
            .filter(is_glyph)
            .map(|image| image.col)
            .collect();
        assert_eq!(child_a_label_cols, vec![child_a_label_x]);

        let lines = rows(&frame);
        assert_eq!(lines[top as usize], " ".repeat(cols as usize));
        assert_eq!(lines[(top + height) as usize], " ".repeat(cols as usize));
    }

    #[test]
    fn on_resize_re_centres_the_next_render_on_the_new_size() {
        let leaf = labelled("hi");
        let doc = Document {
            root: Tree::root(vec![Tree::leaf(leaf.clone())]),
        };
        let mut r = renderer_on(window(20, 10, 1, 1));
        rendered(&mut r, &doc);
        let (cols, rows_count) = (40, 20);
        r.on_resize(window(cols, rows_count, 1, 1));

        let placements = crate::layout::layout(doc.tree());
        let mut frame = Frame::new(r.window);
        frame.centre_on(&placements);
        draw_all(&mut r, &mut frame, &placements);

        let left = (cols - crate::layout::width(&leaf)).div_euclid(2);
        let top = (rows_count - crate::layout::BOX_HEIGHT).div_euclid(2);
        let label_x = left + crate::layout::centre(crate::layout::width(&leaf), "hi");
        let label_row = top + crate::layout::BOX_HEIGHT / 2;
        let label_cols: Vec<i64> = frame
            .images
            .iter()
            .filter(|image| image.row == label_row)
            .map(|image| image.col)
            .collect();
        assert_eq!(label_cols, vec![label_x, label_x + 1]);
    }

    #[test]
    fn on_resize_with_a_different_rounded_cell_height_does_not_panic_on_the_next_render() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let node = box_node(None, false, false);
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
            &[box_placement(&box_node(None, false, false), 0, 0, 4, 3)],
        );
        assert_eq!(r.cache.len(), 1);
        r.on_resize(window(80, 40, 2, 4));
        assert_eq!(r.cache.len(), 1);
    }

    fn rendered(r: &mut TerminalRenderer, doc: &Document) -> String {
        let mut out = Vec::new();
        let mut state = State::default();
        state.doc = doc.clone();
        r.render(&state, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn rendered_diagram(r: &mut TerminalRenderer, doc: &Document) -> String {
        let mut out = Vec::new();
        let mut state = State::default();
        state.doc = doc.clone();
        r.render_diagram(&state, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn empty_doc() -> Document {
        Document::default()
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
        assert!(!rendered(&mut r, &empty_doc()).ends_with('\n'));
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
    fn the_cursor_is_drawn_for_the_selected_box() {
        let boxes = Tree::root(vec![node("hi")]);
        let mut r = renderer_on(Window {
            cols: 20,
            rows: 10,
            cell_width: 1,
            cell_height: 1,
        });
        let placements = with_cursor(crate::layout::layout(&boxes), Some(vec![0]));
        assert!(placements
            .iter()
            .any(|placement| matches!(placement.node, PlacementNode::Cursor(_))));
        // with_cursor always appends the cursor placement last, so draw order
        // (and thus frame.images order) puts its sprite at the end.
        let images = sprites(&mut r, &placements);
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
    fn no_cursor_is_drawn_without_a_selection() {
        let mut r = renderer_on(Window {
            cols: 20,
            rows: 10,
            cell_width: 1,
            cell_height: 1,
        });
        let boxes = Tree::root(vec![node("hi")]);
        let placements = with_cursor(crate::layout::layout(&boxes), None);
        assert!(!placements
            .iter()
            .any(|placement| matches!(placement.node, PlacementNode::Cursor(_))));
        sprites(&mut r, &placements);
    }

    #[test]
    fn an_empty_drawing_draws_no_placements() {
        let window = window(40, 10, 1, 1);
        let mut r = renderer_on(window);
        let state = crate::state::new_state(vec![], Mode::Command, None);
        let mut out = Vec::new();
        r.render_diagram(&state, &mut out).unwrap();
        let empty_frame = String::from_utf8(Frame::new(window).into_bytes()).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), empty_frame);
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
            &[box_placement(&box_node(None, false, false), 4, 4, 3, 3)],
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
            &[box_placement(&box_node(None, false, false), 4, 4, 3, 3)],
        );
        assert_eq!(&grid[4][4..7], "   ");
    }

    #[test]
    fn a_box_reaching_past_the_edge_is_clipped() {
        let grid = grid(
            &mut renderer_on(window(4, 2, 1, 1)),
            &[box_placement(&box_node(None, true, false), 3, 1, 3, 3)],
        );
        assert_eq!(grid, vec!["    ".to_string(), "    ".to_string()]);
    }

    #[test]
    fn label_is_drawn_inside_the_box() {
        let node = box_node(None, false, false);
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
        let node = box_node(None, false, false);
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
        let node = box_node(None, false, false);
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
            &[box_placement(&box_node(None, false, false), 0, 0, 5, 3)],
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
        let node = box_node(None, false, false);
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
            &[box_placement(&box_node(Some(2), false, false), 0, 0, 5, 3)],
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
            &[box_placement(&box_node(None, false, false), 10, 0, 4, 3)],
        );
        assert!(images.is_empty());
    }

    #[test]
    fn a_box_overhanging_the_left_is_cropped() {
        let mut r = renderer_on(window(40, 20, 4, 4));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), -2, 1, 5, 3)],
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
            &[box_placement(&box_node(None, false, false), 1, -2, 4, 5)],
        );
        assert_eq!(images[0].row, 0);
        assert_eq!(images[0].canvas.height, 3 * 4);
    }

    #[test]
    fn a_box_overhanging_the_right_is_cropped() {
        let mut r = renderer_on(window(4, 20, 4, 4));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 1, 0, 6, 3)],
        );
        assert_eq!(images[0].col, 1);
        assert_eq!(images[0].canvas.width, 3 * 4);
    }

    #[test]
    fn a_box_overhanging_the_bottom_is_cropped() {
        let mut r = renderer_on(window(40, 4, 4, 4));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 0, 1, 3, 6)],
        );
        assert_eq!(images[0].row, 1);
        assert_eq!(images[0].canvas.height, 3 * 4);
    }

    #[test]
    fn a_box_sprite_sits_at_the_placement_cell() {
        let mut r = renderer_on(window(40, 20, 6, 12));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 1, 2, 4, 3)],
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
                    &box_node(Some(index), false, false),
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
        let node = box_node(Some(1), true, false);
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
            &[box_placement(&box_node(None, false, false), 0, 0, 4, 3)],
        );
        let blue = sprites(
            &mut r,
            &[box_placement(&box_node(Some(4), false, false), 0, 0, 4, 3)],
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
                &[box_placement(&box_node(None, false, rounded), 0, 0, 4, 3)],
            );
        }
        assert_eq!(r.cache.len(), 2);
    }

    #[test]
    fn a_relabelled_box_of_the_same_size_reuses_its_pixels() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let first = sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 0, 0, 4, 3)],
        );
        let second = sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 0, 0, 4, 3)],
        );
        assert_eq!(first[0].canvas.pixels, second[0].canvas.pixels);
        assert_eq!(r.cache.len(), 1);
    }

    #[test]
    fn a_cached_sprite_moves_to_its_own_position() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 0, 0, 4, 3)],
        );
        let moved = sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 5, 2, 4, 3)],
        );
        assert_eq!((moved[0].col, moved[0].row), (5, 2));
    }

    #[test]
    fn a_moved_box_reuses_its_cached_pixels() {
        let mut r = renderer_on(window(40, 20, 2, 4));
        let first = sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 0, 0, 4, 3)],
        );
        let moved = sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 5, 2, 4, 3)],
        );
        assert_eq!(first[0].canvas.pixels, moved[0].canvas.pixels);
        assert_eq!(r.cache.len(), 1);
    }

    #[test]
    fn a_differently_cropped_box_shares_one_cache_entry() {
        let mut r = renderer_on(window(4, 20, 2, 4));
        let whole = sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 0, 0, 4, 3)],
        );
        let cropped = sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 2, 0, 4, 3)],
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
                box_placement(&box_node(None, false, false), 10, 0, 4, 3),
                arrow_placement(vec![0], 0, 0, 10, 4, 2),
            ],
        );
        assert!(r.cache.is_empty());
    }

    #[test]
    fn a_box_beyond_the_right_edge_is_not_shown() {
        let node = box_node(None, false, false);
        let frame = Frame::new(window(20, 10, 4, 8));
        assert!(!frame.shows(&box_placement(&node, 20, 0, 4, 3)));
    }

    #[test]
    fn a_box_beyond_the_top_edge_is_not_shown() {
        let node = box_node(None, false, false);
        let frame = Frame::new(window(20, 10, 4, 8));
        assert!(!frame.shows(&box_placement(&node, 0, -3, 4, 3)));
    }

    #[test]
    fn a_box_straddling_an_edge_is_shown() {
        let node = box_node(None, false, false);
        let frame = Frame::new(window(20, 10, 4, 8));
        assert!(frame.shows(&box_placement(&node, 18, 0, 4, 3)));
        assert!(frame.shows(&box_placement(&node, -2, 8, 4, 3)));
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
                    &box_node(None, false, false),
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
        node: &crate::diagram::Node,
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
        let sprite = box_outline(&r, &box_node(None, false, false), size, size);
        assert_eq!(pixel_of(&sprite, BORDER + 1, BORDER + 1), TRANSPARENT);
    }

    #[test]
    fn a_fill_colour_is_composited_over_black_via_outline_box() {
        let r = renderer(1, 1);
        let size = 2 * BORDER + 3;
        let sprite = box_outline(&r, &box_node(Some(2), true, false), size, size);
        assert_eq!(
            pixel_of(&sprite, BORDER + 1, BORDER + 1),
            fill_colour(Some(2), true)
        );
    }

    #[test]
    fn a_rounded_box_cuts_away_its_extreme_corners_via_outline_box() {
        let r = renderer(8, 8);
        let sprite = box_outline(&r, &box_node(Some(1), true, true), 10, 10);
        let (last_x, last_y) = (sprite.width - 1, sprite.height - 1);
        assert_eq!(pixel_of(&sprite, 0, 0), TRANSPARENT);
        assert_eq!(pixel_of(&sprite, last_x, 0), TRANSPARENT);
        assert_eq!(pixel_of(&sprite, 0, last_y), TRANSPARENT);
        assert_eq!(pixel_of(&sprite, last_x, last_y), TRANSPARENT);
    }

    #[test]
    fn the_radius_leaves_the_sprite_size_alone() {
        let r = renderer(8, 8);
        let square = box_outline(&r, &box_node(Some(1), true, false), 10, 10);
        let rounded = box_outline(&r, &box_node(Some(1), true, true), 10, 10);
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

    fn status_line_output(r: &mut TerminalRenderer, state: &State) -> String {
        let mut out = Vec::new();
        r.render_status_line(state, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn strip_escapes(output: &str) -> String {
        let mut visible = String::new();
        let mut chars = output.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                for skipped in chars.by_ref() {
                    if skipped.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                visible.push(c);
            }
        }
        visible
    }

    fn status_line_text(r: &mut TerminalRenderer, state: &State) -> String {
        strip_escapes(&status_line_output(r, state))
    }

    fn status_line_runs(r: &mut TerminalRenderer, state: &State) -> Vec<String> {
        let output = status_line_output(r, state);
        let body = output
            .strip_prefix(&format!("\x1b[{};1H", r.window.rows))
            .unwrap();
        body.strip_suffix("\x1b[0m")
            .unwrap()
            .split("\x1b[0m")
            .skip(1)
            .map(str::to_string)
            .collect()
    }

    fn background_escape(style: crate::status_line::Style) -> String {
        let (r, g, b) = composite(style.background.unwrap(), palette(BACKGROUND).unwrap());
        format!("\x1b[48;2;{r};{g};{b}m")
    }

    fn dim_run_prefix(state: &State) -> String {
        background_escape(status_line(&state.status_input()).right[0].style)
    }

    #[test]
    fn composite_blends_the_overlay_over_the_backdrop_by_its_alpha() {
        assert_eq!(composite((200, 100, 0, 255), (10, 20, 30)), (200, 100, 0));
        assert_eq!(composite((200, 100, 0, 0), (10, 20, 30)), (10, 20, 30));
        assert_eq!(composite((0, 0, 0, 128), (100, 50, 10)), (50, 25, 5));
    }

    #[test]
    fn the_status_line_puts_the_mode_and_filename_on_the_left_and_the_box_count_on_the_right() {
        let mut r = renderer_on(window(40, 2, 1, 1));
        let state = crate::state::new_state(vec![], Mode::Command, None);
        let text = status_line_text(&mut r, &state);
        assert!(text.starts_with(" COMMANDING  \u{2502} diagram.dre"));
        assert!(text.ends_with("0 boxes \u{2022} dre"));
        let left = " COMMANDING  \u{2502} diagram.dre";
        let right = "0 boxes \u{2022} dre";
        let middle: String = text
            .chars()
            .skip(left.chars().count())
            .take(text.chars().count() - left.chars().count() - right.chars().count())
            .collect();
        assert!(!middle.is_empty());
        assert!(middle.chars().all(|c| c == BLANK));
    }

    #[test]
    fn the_status_line_shows_a_dirty_marker_after_the_filename() {
        let mut r = renderer_on(window(40, 2, 1, 1));
        let mut state = crate::state::new_state(vec![], Mode::Command, None);
        state.dirty = true;
        let text = status_line_text(&mut r, &state);
        assert!(text.starts_with(" COMMANDING  \u{2502} diagram.dre[+]"));
    }

    #[test]
    fn the_status_line_fills_the_terminal_width() {
        let mut r = renderer_on(window(40, 2, 1, 1));
        let state = crate::state::new_state(vec![], Mode::Command, None);
        assert_eq!(status_line_text(&mut r, &state).chars().count(), 40);
    }

    #[test]
    fn the_status_line_is_cut_to_the_terminal_width() {
        let mut r = renderer_on(window(3, 2, 1, 1));
        let state = crate::state::new_state(vec![], Mode::Command, None);
        assert_eq!(status_line_text(&mut r, &state), " CO");
    }

    #[test]
    fn the_status_line_is_cut_inside_a_multibyte_segment() {
        let mut r = renderer_on(window(13, 2, 1, 1));
        let state = crate::state::new_state(vec![], Mode::Command, None);
        assert_eq!(status_line_text(&mut r, &state), " COMMANDING  ");
        let mut r = renderer_on(window(14, 2, 1, 1));
        assert_eq!(status_line_text(&mut r, &state), " COMMANDING  \u{2502}");
    }

    #[test]
    fn the_status_line_is_written_to_the_last_row() {
        let mut r = renderer_on(window(5, 4, 1, 1));
        let state = crate::state::new_state(vec![], Mode::Command, None);
        assert!(status_line_output(&mut r, &state).starts_with("\x1b[4;1H"));
    }

    #[test]
    fn on_resize_moves_the_status_line_to_the_new_last_row() {
        let mut r = renderer_on(window(5, 4, 1, 1));
        r.on_resize(window(5, 9, 1, 1));
        let state = crate::state::new_state(vec![], Mode::Command, None);
        assert!(status_line_output(&mut r, &state).starts_with("\x1b[9;1H"));
    }

    #[test]
    fn the_status_line_moves_the_cursor_once() {
        let mut r = renderer_on(window(40, 4, 1, 1));
        let state = crate::state::new_state(vec![], Mode::Command, None);
        let output = status_line_output(&mut r, &state);
        assert_eq!(output.matches("H").count(), 1);
        assert!(output.starts_with("\x1b[4;1H"));
    }

    #[test]
    fn the_status_line_never_uses_reverse_video() {
        let mut r = renderer_on(window(40, 4, 1, 1));
        let state = crate::state::new_state(vec![], Mode::Command, None);
        assert!(!status_line_output(&mut r, &state).contains("\x1b[7m"));
    }

    #[test]
    fn the_status_line_ends_by_resetting_the_style() {
        let mut r = renderer_on(window(40, 4, 1, 1));
        let state = crate::state::new_state(vec![], Mode::Command, None);
        assert!(status_line_output(&mut r, &state).ends_with("\x1b[0m"));
    }

    #[test]
    fn the_mode_cell_is_bold_on_lime_with_the_background_colour_as_text() {
        let mut r = renderer_on(window(40, 4, 1, 1));
        let state = crate::state::new_state(vec![], Mode::Command, None);
        let (lr, lg, lb) = palette(0).unwrap();
        let (br, bg, bb) = palette(BACKGROUND).unwrap();
        let expected =
            format!("\x1b[1m\x1b[48;2;{lr};{lg};{lb}m\x1b[38;2;{br};{bg};{bb}m COMMANDING ");
        assert_eq!(status_line_runs(&mut r, &state)[0], expected);
    }

    #[test]
    fn everything_after_the_mode_cell_has_a_dim_background_and_no_foreground() {
        let mut r = renderer_on(window(40, 4, 1, 1));
        let state = crate::state::new_state(vec![], Mode::Command, None);
        let runs = status_line_runs(&mut r, &state);
        let dim = dim_run_prefix(&state);
        assert!(runs.len() > 1);
        for run in &runs[1..] {
            assert!(run.starts_with(&dim), "{run:?}");
            assert!(!run.contains("\x1b[38;"));
            assert!(!run.contains("\x1b[1m"));
        }
    }

    #[test]
    fn the_filler_between_left_and_right_is_dim() {
        let mut r = renderer_on(window(40, 4, 1, 1));
        let state = crate::state::new_state(vec![], Mode::Command, None);
        let runs = status_line_runs(&mut r, &state);
        let content_width: usize = status_line(&state.status_input())
            .left
            .iter()
            .chain(&status_line(&state.status_input()).right)
            .map(|segment| segment.text.chars().count())
            .sum();
        let filler = format!(
            "{}{}",
            dim_run_prefix(&state),
            " ".repeat(40 - content_width)
        );
        assert!(runs.contains(&filler), "{runs:?}");
    }

    #[test]
    fn the_status_line_can_be_cut_inside_the_mode_cell_keeping_its_style() {
        let mut r = renderer_on(window(3, 2, 1, 1));
        let state = crate::state::new_state(vec![], Mode::Command, None);
        let runs = status_line_runs(&mut r, &state);
        assert_eq!(runs.len(), 1);
        let (lr, lg, lb) = palette(0).unwrap();
        assert!(runs[0].starts_with(&format!("\x1b[1m\x1b[48;2;{lr};{lg};{lb}m")));
        assert!(runs[0].ends_with(" CO"));
    }

    #[test]
    fn the_save_prompt_keeps_the_mode_cell_styling_and_dims_the_rest() {
        let mut r = renderer_on(window(60, 4, 1, 1));
        let mode = Mode::SavePrompt {
            filename: "diagram.dre".to_string(),
        };
        let state = crate::state::new_state(vec![], mode, None);
        let runs = status_line_runs(&mut r, &state);
        let mode_style = status_line(&state.status_input()).left[0].style;
        let mode_prefix = format!("\x1b[1m{}", background_escape(mode_style));
        assert!(runs[0].starts_with(&mode_prefix));
        let dim = dim_run_prefix(&state);
        for run in &runs[1..] {
            assert!(run.starts_with(&dim));
        }
        assert_eq!(strip_escapes(&runs.concat()).chars().count(), 60);
    }

    fn render_output(r: &mut TerminalRenderer, state: &State) -> String {
        let mut out = Vec::new();
        r.render(state, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn render_shows_editing_in_insert_mode() {
        let mut r = renderer_on(window(20, 4, 1, 1));
        let state = crate::state::new_state(vec![], Mode::Insert, None);
        assert!(render_output(&mut r, &state).contains("EDITING"));
    }

    #[test]
    fn render_shows_commanding_in_command_mode() {
        let mut r = renderer_on(window(20, 4, 1, 1));
        let state = crate::state::new_state(vec![], Mode::Command, None);
        assert!(render_output(&mut r, &state).contains("COMMANDING"));
    }

    #[test]
    fn render_shows_the_filename_being_typed_in_save_prompt_mode() {
        let mut r = renderer_on(window(60, 4, 1, 1));
        let mode = Mode::SavePrompt {
            filename: "diagram.dre".to_string(),
        };
        let state = crate::state::new_state(vec![], mode, None);
        let expected_left: String = status_line(&state.status_input())
            .left
            .iter()
            .map(|s| s.text.as_str())
            .collect();
        let text = status_line_text(&mut r, &state);
        assert!(text.starts_with(&expected_left));
        assert!(strip_escapes(&render_output(&mut r, &state)).contains(&expected_left));
    }
}
