use std::io::{self, Write};

use super::font::GlyphCache;
use super::shapes::{ArrowShape, BoxShape};
use super::{colour, Renderer, BORDER, FILL_ALPHA, OPAQUE, ROUNDED_RADIUS};
use crate::canvas::Canvas;
use crate::diagram::{palette, Node};
use crate::kitty;
use crate::layout::{with_cursor, Label, Placement, PlacementNode};
use crate::state::{Mode, State};
use crate::terminal::Terminal;

const BLANK: char = ' ';
const CURSOR: char = '\u{2588}';
const HOME_CURSOR: &str = "\x1b[H";

pub(super) const ARROW_STROKE: i64 = 4;

const CACHE_LIMIT: usize = 512;

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
        hint: bool,
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
            hint: node.hint,
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

struct Screen {
    terminal: Terminal,
    origin: (i64, i64),
    characters: Vec<Vec<char>>,
    images: Vec<Placed>,
}

impl Screen {
    fn new(terminal: Terminal) -> Self {
        Screen {
            terminal,
            origin: (0, 0),
            characters: vec![vec![BLANK; terminal.cols as usize]; terminal.rows as usize],
            images: Vec::new(),
        }
    }

    fn centre_on(&mut self, placements: &[Placement], scroll_x: i64) {
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
        let horizontal = if span <= self.terminal.cols {
            (self.terminal.cols - span).div_euclid(2)
        } else {
            -scroll_x
        };
        self.origin = (horizontal, (self.terminal.rows - height).div_euclid(2));
    }

    // Clipping happens by cropping: kitty::show cannot position at a negative
    // column, and sends a=T without C=1, so an overhang would shift into view
    // or scroll the screen instead of being cut off.
    fn crop(&self, placement: &Placement) -> Option<Crop> {
        let left = placement.x + self.origin.0;
        let top = placement.y + self.origin.1;
        let col = left.max(0);
        let row = top.max(0);
        let right = (left + placement.width).min(self.terminal.cols);
        let bottom = (top + placement.height).min(self.terminal.rows);
        if col >= right || row >= bottom {
            return None;
        }
        Some(Crop {
            col,
            row,
            first_x: (col - left) * self.terminal.cell_width,
            last_x: (right - left) * self.terminal.cell_width,
            first_y: (row - top) * self.terminal.cell_height,
            last_y: (bottom - top) * self.terminal.cell_height,
        })
    }

    fn shows(&self, placement: &Placement) -> bool {
        let left = placement.x + self.origin.0;
        let top = placement.y + self.origin.1;
        left.max(0) < (left + placement.width).min(self.terminal.cols)
            && top.max(0) < (top + placement.height).min(self.terminal.rows)
    }

    fn write(&mut self, x: i64, y: i64, character: char) {
        let (x, y) = (x + self.origin.0, y + self.origin.1);
        if 0 <= y && (y as usize) < self.characters.len() {
            let row = &mut self.characters[y as usize];
            if 0 <= x && (x as usize) < row.len() {
                row[x as usize] = character;
            }
        }
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

fn draw_cursor(screen: &mut Screen, placement: &Placement) {
    screen.write(placement.x, placement.y, CURSOR);
}

const HINT_TEXT: &str = "press b to add a box";

fn hint_node() -> Node {
    Node {
        label: HINT_TEXT.to_string(),
        hint: true,
        ..Default::default()
    }
}

fn status_text(mode: &Mode) -> String {
    match mode {
        Mode::Insert => "EDITING".into(),
        Mode::Command => "COMMANDING".into(),
        Mode::SavePrompt { filename } => format!("Save as: {filename}{CURSOR}"),
    }
}

pub(crate) struct TerminalRenderer {
    terminal: Terminal,
    cache: std::collections::HashMap<SpriteKey, Canvas>,
    glyph_cache: GlyphCache,
}

impl Renderer for TerminalRenderer {
    fn render(&mut self, state: &State, out: &mut impl Write) -> io::Result<()> {
        self.render_diagram(state, out)?;
        self.render_status_line(state, out)
    }
}

impl TerminalRenderer {
    pub(crate) fn new(terminal: Terminal) -> Self {
        let glyph_cache = GlyphCache::new(terminal.cell_width, terminal.cell_height);
        TerminalRenderer {
            terminal,
            cache: std::collections::HashMap::new(),
            glyph_cache,
        }
    }

    pub(crate) fn on_resize(&mut self, terminal: Terminal) {
        self.terminal.cols = terminal.cols;
        self.terminal.rows = terminal.rows;
    }

    pub(crate) fn columns(&self) -> i64 {
        self.terminal.cols
    }

    fn render_diagram(&mut self, state: &State, out: &mut impl Write) -> io::Result<()> {
        let hint_boxes = [hint_node()];
        let placements = if state.doc.boxes.is_empty() {
            crate::layout::layout(&hint_boxes)
        } else {
            with_cursor(
                crate::layout::layout(&state.doc.boxes),
                state.doc.selected.clone(),
            )
        };
        let mut screen = Screen::new(self.terminal);
        screen.centre_on(&placements, state.scroll_x);
        for placement in &placements {
            match &placement.node {
                PlacementNode::Node(_) => self.draw_box(&mut screen, placement),
                PlacementNode::Arrow(_) => self.draw_arrow(&mut screen, placement),
                PlacementNode::Label(label) => self.draw_label(&mut screen, placement, label),
                PlacementNode::Cursor(_) => draw_cursor(&mut screen, placement),
            }
        }
        out.write_all(&screen.into_bytes())
    }

    fn render_status_line(&mut self, state: &State, out: &mut impl Write) -> io::Result<()> {
        let text = status_text(&state.mode);
        let Terminal { cols, rows, .. } = self.terminal;
        let line: String = text
            .chars()
            .chain(std::iter::repeat(BLANK))
            .take(cols as usize)
            .collect();
        write!(out, "\x1b[{rows};1H\x1b[7m{line}\x1b[0m")
    }

    fn draw_box(&mut self, screen: &mut Screen, placement: &Placement) {
        if !screen.shows(placement) {
            return;
        }
        let key = sprite_key(placement);
        if !self.cache.contains_key(&key) {
            let drawn = self.outline_box(placement);
            self.remember(key.clone(), drawn);
        }
        screen.place(&self.cache[&key], placement);
    }

    fn draw_arrow(&mut self, screen: &mut Screen, placement: &Placement) {
        if !screen.shows(placement) {
            return;
        }
        let key = sprite_key(placement);
        if !self.cache.contains_key(&key) {
            let drawn = self.outline_arrow(placement);
            self.remember(key.clone(), drawn);
        }
        screen.place(&self.cache[&key], placement);
    }

    fn draw_label(&mut self, screen: &mut Screen, placement: &Placement, label: &Label) {
        for (offset, character) in label.text.chars().enumerate() {
            let char_placement = Placement {
                node: PlacementNode::Label(label.clone()),
                x: placement.x + offset as i64,
                y: placement.y,
                width: 1,
                height: 1,
            };
            if !screen.shows(&char_placement) {
                continue;
            }
            let glyph = self.glyph_cache.glyph(character, label.hint);
            screen.place(glyph, &char_placement);
        }
    }

    fn remember(&mut self, key: SpriteKey, drawn: Canvas) {
        if self.cache.len() >= CACHE_LIMIT {
            self.cache.clear();
        }
        self.cache.insert(key, drawn);
    }

    fn cells_to_pixels_x(&self, cells: i64) -> i64 {
        cells * self.terminal.cell_width
    }

    fn cells_to_pixels_y(&self, cells: i64) -> i64 {
        cells * self.terminal.cell_height
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
            edge: [r, g, b, if node.hint { OPAQUE / 4 } else { OPAQUE }],
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
            .map(|stop| self.cells_to_pixels_y(*stop) + self.terminal.cell_height / 2)
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
            shaft_row: self.cells_to_pixels_y(arrow.shaft) + self.terminal.cell_height / 2,
            trunk,
            stroke: ARROW_STROKE,
            ink: [r, g, b, OPAQUE],
        };
        Canvas::fill(width, height, &shape)
    }
}

#[cfg(test)]
mod tests {
    use super::super::PLAIN_COLOUR;
    use super::*;
    use crate::diagram::{node, node_with_children, Document};

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
            children: vec![],
            hint: false,
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
    fn sprite_key_differs_by_hint() {
        let node_a = crate::diagram::Node {
            hint: false,
            ..box_node(Some(1), true, true)
        };
        let node_b = crate::diagram::Node {
            hint: true,
            ..box_node(Some(1), true, true)
        };
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

    fn terminal(cols: i64, rows: i64, cell_width: i64, cell_height: i64) -> Terminal {
        Terminal {
            cols,
            rows,
            cell_width,
            cell_height,
        }
    }

    fn renderer(cell_width: i64, cell_height: i64) -> TerminalRenderer {
        renderer_on(terminal(0, 0, cell_width, cell_height))
    }

    fn renderer_on(terminal: Terminal) -> TerminalRenderer {
        TerminalRenderer::new(terminal)
    }

    fn draw_all(r: &mut TerminalRenderer, screen: &mut Screen, placements: &[Placement]) {
        for placement in placements {
            match &placement.node {
                PlacementNode::Node(_) => r.draw_box(screen, placement),
                PlacementNode::Arrow(_) => r.draw_arrow(screen, placement),
                PlacementNode::Label(label) => r.draw_label(screen, placement, label),
                PlacementNode::Cursor(_) => draw_cursor(screen, placement),
            }
        }
    }

    fn drawn_screen(r: &mut TerminalRenderer, placements: &[Placement]) -> Screen {
        let mut screen = Screen::new(r.terminal);
        draw_all(r, &mut screen, placements);
        screen
    }

    fn rows(screen: &Screen) -> Vec<String> {
        screen
            .characters
            .iter()
            .map(|row| row.iter().collect())
            .collect()
    }

    fn grid(r: &mut TerminalRenderer, placements: &[Placement]) -> Vec<String> {
        rows(&drawn_screen(r, placements))
    }

    fn sprites(r: &mut TerminalRenderer, placements: &[Placement]) -> Vec<Placed> {
        drawn_screen(r, placements).images
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
                path: crate::diagram::Path {
                    ancestors: vec![],
                    index: 0,
                },
                hint: false,
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
        let mut screen = Screen::new(terminal(cols, rows, 1, 1));
        screen.centre_on(&[box_placement(&node, 0, 0, width, height)], 0);
        assert_eq!(
            screen.origin,
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
        let mut screen = Screen::new(terminal(cols, rows, 1, 1));
        screen.centre_on(&placements, 0);
        assert_eq!(
            screen.origin,
            ((cols - 8).div_euclid(2), (rows - 6).div_euclid(2))
        );
    }

    #[test]
    fn centre_on_nothing_leaves_the_origin_at_the_corner() {
        let mut screen = Screen::new(terminal(20, 10, 1, 1));
        screen.centre_on(&[], 0);
        assert_eq!(screen.origin, (0, 0));
    }

    #[test]
    fn centre_on_uses_scroll_x_when_the_diagram_overflows() {
        let node = box_node(None, false, false);
        let (cols, rows) = (10, 10);
        let placements = vec![box_placement(&node, 0, 0, 30, 4)];
        let mut screen = Screen::new(terminal(cols, rows, 1, 1));
        screen.centre_on(&placements, 0);
        assert_eq!(screen.origin, (0, (rows - 4).div_euclid(2)));
    }

    #[test]
    fn centre_on_shifts_left_as_scroll_x_grows() {
        let node = box_node(None, false, false);
        let (cols, rows) = (10, 10);
        let placements = vec![box_placement(&node, 0, 0, 30, 4)];
        let mut screen = Screen::new(terminal(cols, rows, 1, 1));
        screen.centre_on(&placements, 5);
        assert_eq!(screen.origin, (-5, (rows - 4).div_euclid(2)));
    }

    #[test]
    fn centre_on_ignores_scroll_x_when_the_diagram_fits() {
        let node = box_node(None, false, false);
        let (cols, rows) = (20, 10);
        let (width, height) = (6, 4);
        let mut screen = Screen::new(terminal(cols, rows, 1, 1));
        screen.centre_on(&[box_placement(&node, 0, 0, width, height)], 5);
        assert_eq!(
            screen.origin,
            ((cols - width).div_euclid(2), (rows - height).div_euclid(2))
        );
    }

    #[test]
    fn a_box_on_screen_is_cropped_to_the_whole_shape() {
        let node = box_node(None, false, false);
        let window = terminal(20, 10, 4, 8);
        let (width, height) = (5, 4);
        let screen = Screen::new(window);
        assert_eq!(
            screen.crop(&box_placement(&node, 2, 3, width, height)),
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
        let mut screen = Screen::new(terminal(20, 10, 4, 8));
        screen.centre_on(&[box_placement(&node, 0, 0, 6, 4)], 0);
        let crop = screen.crop(&box_placement(&node, 1, 1, 3, 2)).unwrap();
        assert_eq!(
            (crop.col, crop.row),
            (1 + screen.origin.0, 1 + screen.origin.1)
        );
    }

    #[test]
    fn a_crop_overhanging_the_left_drops_the_hidden_columns() {
        let node = box_node(None, false, false);
        let window = terminal(20, 10, 4, 8);
        let (hidden, width) = (2, 5);
        let screen = Screen::new(window);
        let crop = screen
            .crop(&box_placement(&node, -hidden, 0, width, 3))
            .unwrap();
        assert_eq!(crop.col, 0);
        assert_eq!(crop.first_x, hidden * window.cell_width);
        assert_eq!(crop.last_x, width * window.cell_width);
    }

    #[test]
    fn a_crop_overhanging_the_top_drops_the_hidden_rows() {
        let node = box_node(None, false, false);
        let window = terminal(20, 10, 4, 8);
        let (hidden, height) = (2, 5);
        let screen = Screen::new(window);
        let crop = screen
            .crop(&box_placement(&node, 0, -hidden, 3, height))
            .unwrap();
        assert_eq!(crop.row, 0);
        assert_eq!(crop.first_y, hidden * window.cell_height);
        assert_eq!(crop.last_y, height * window.cell_height);
    }

    #[test]
    fn a_crop_overhanging_the_right_stops_at_the_last_column() {
        let node = box_node(None, false, false);
        let window = terminal(20, 10, 4, 8);
        let x = 18;
        let screen = Screen::new(window);
        let crop = screen.crop(&box_placement(&node, x, 0, 5, 3)).unwrap();
        assert_eq!(crop.col, x);
        assert_eq!(crop.first_x, 0);
        assert_eq!(crop.last_x, (window.cols - x) * window.cell_width);
    }

    #[test]
    fn a_crop_overhanging_the_bottom_stops_at_the_last_row() {
        let node = box_node(None, false, false);
        let window = terminal(20, 10, 4, 8);
        let y = 8;
        let screen = Screen::new(window);
        let crop = screen.crop(&box_placement(&node, 0, y, 3, 5)).unwrap();
        assert_eq!(crop.row, y);
        assert_eq!(crop.first_y, 0);
        assert_eq!(crop.last_y, (window.rows - y) * window.cell_height);
    }

    #[test]
    fn a_box_beyond_the_right_edge_has_no_crop() {
        let node = box_node(None, false, false);
        let screen = Screen::new(terminal(20, 10, 4, 8));
        assert_eq!(screen.crop(&box_placement(&node, 20, 0, 4, 3)), None);
    }

    #[test]
    fn a_box_beyond_the_top_edge_has_no_crop() {
        let node = box_node(None, false, false);
        let screen = Screen::new(terminal(20, 10, 4, 8));
        assert_eq!(screen.crop(&box_placement(&node, 0, -3, 4, 3)), None);
    }

    #[test]
    fn line_count_is_unchanged() {
        let mut r = renderer_on(terminal(3, 3, 2, 4));
        assert_eq!(lines_of(&rendered(&mut r, &empty_doc())).len(), 3);
    }

    #[test]
    fn the_graphics_payload_is_appended_to_the_last_line_only() {
        let screen = Screen::new(terminal(3, 2, 2, 4));
        let frame = String::from_utf8(screen.into_bytes()).unwrap();
        let lines = lines_of(&frame);
        assert_eq!(lines[0], BLANK.to_string().repeat(3));
        assert_eq!(
            lines[1],
            format!("{}{}", BLANK.to_string().repeat(3), kitty::clear())
        );
    }

    #[test]
    fn a_frame_with_sprites_ends_with_a_clear_then_each_sprite_shown_in_order() {
        let parent = node_with_children("parent", vec![node("a"), node("b")]);
        let nodes = vec![parent];
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
        let terminal = terminal(cols, rows, 2, 4);
        let images = sprites(&mut renderer_on(terminal), &placements);
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
            boxes: nodes.clone(),
            selected: None,
        };
        assert!(rendered_diagram(&mut renderer_on(terminal), &doc).ends_with(&expected));
    }

    #[test]
    fn centres_a_leaf_box_within_the_terminal() {
        let leaf = node("hi");
        let nodes = vec![leaf.clone()];
        let placements = crate::layout::layout(&nodes);
        let cols = 20;
        let rows_count = 10;
        let left = (cols - crate::layout::width(&leaf)).div_euclid(2);
        let top = (rows_count - crate::layout::BOX_HEIGHT).div_euclid(2);
        let mut r = renderer_on(terminal(cols, rows_count, 1, 1));
        let mut screen = Screen::new(r.terminal);
        screen.centre_on(&placements, 0);
        draw_all(&mut r, &mut screen, &placements);
        let label_x = left + crate::layout::centre(crate::layout::width(&leaf), "hi");
        let label_row = top + crate::layout::BOX_HEIGHT / 2;
        let label_cols: Vec<i64> = screen
            .images
            .iter()
            .filter(|image| image.row == label_row)
            .map(|image| image.col)
            .collect();
        assert_eq!(label_cols, vec![label_x, label_x + 1]);
        let lines = rows(&screen);
        assert_eq!(lines[top as usize], " ".repeat(cols as usize));
        assert_eq!(
            lines[(top + crate::layout::BOX_HEIGHT) as usize],
            " ".repeat(cols as usize)
        );
    }

    #[test]
    fn an_overflowing_diagram_uses_scroll_x_instead_of_centring() {
        let leaf = node("hi");
        let nodes = vec![leaf.clone()];
        let placements = crate::layout::layout(&nodes);
        let (cols, rows_count) = (3, 10);
        let top = (rows_count - crate::layout::BOX_HEIGHT).div_euclid(2);
        let label_row = top + crate::layout::BOX_HEIGHT / 2;
        let label_x = crate::layout::centre(crate::layout::width(&leaf), "hi");

        let mut r = renderer_on(terminal(cols, rows_count, 1, 1));
        let mut screen = Screen::new(r.terminal);
        screen.centre_on(&placements, 0);
        draw_all(&mut r, &mut screen, &placements);
        let unscrolled_cols: Vec<i64> = screen
            .images
            .iter()
            .filter(|image| image.row == label_row)
            .map(|image| image.col)
            .collect();
        assert_eq!(unscrolled_cols, vec![label_x, label_x + 1]);

        let mut r = renderer_on(terminal(cols, rows_count, 1, 1));
        let mut screen = Screen::new(r.terminal);
        screen.centre_on(&placements, 1);
        draw_all(&mut r, &mut screen, &placements);
        let scrolled_cols: Vec<i64> = screen
            .images
            .iter()
            .filter(|image| image.row == label_row)
            .map(|image| image.col)
            .collect();
        assert_eq!(scrolled_cols, vec![label_x - 1, label_x]);
    }

    #[test]
    fn centres_a_parent_and_children_as_a_group() {
        let parent = node_with_children("parent", vec![node("a"), node("b")]);
        let nodes = vec![parent.clone()];
        let placements = crate::layout::layout(&nodes);
        let cols = 40;
        let rows_count = 12;
        let span = crate::layout::width(&parent)
            + crate::layout::GAP_WIDTH
            + crate::layout::width(&node("a"));
        let height =
            crate::layout::LEAF_STRIDE * crate::layout::HALF_PITCH + crate::layout::BOX_HEIGHT;
        let left = (cols - span).div_euclid(2);
        let top = (rows_count - height).div_euclid(2);

        let mut r = renderer_on(terminal(cols, rows_count, 1, 1));
        let mut screen = Screen::new(r.terminal);
        screen.centre_on(&placements, 0);
        draw_all(&mut r, &mut screen, &placements);

        let child_base_x = crate::layout::width(&parent) + crate::layout::GAP_WIDTH;
        let parent_label_x = left + crate::layout::centre(crate::layout::width(&parent), "parent");
        let parent_label_row = top + crate::layout::HALF_PITCH + crate::layout::BOX_HEIGHT / 2;
        let child_a_label_x =
            left + child_base_x + crate::layout::centre(crate::layout::width(&node("a")), "a");
        let child_a_label_row = top + crate::layout::BOX_HEIGHT / 2;

        let is_glyph = |image: &&Placed| image.canvas.width == 1 && image.canvas.height == 1;
        let parent_label_cols: Vec<i64> = screen
            .images
            .iter()
            .filter(|image| image.row == parent_label_row)
            .filter(is_glyph)
            .map(|image| image.col)
            .collect();
        let expected_parent_cols: Vec<i64> =
            (parent_label_x..parent_label_x + "parent".len() as i64).collect();
        assert_eq!(parent_label_cols, expected_parent_cols);

        let child_a_label_cols: Vec<i64> = screen
            .images
            .iter()
            .filter(|image| image.row == child_a_label_row)
            .filter(is_glyph)
            .map(|image| image.col)
            .collect();
        assert_eq!(child_a_label_cols, vec![child_a_label_x]);

        let lines = rows(&screen);
        assert_eq!(lines[top as usize], " ".repeat(cols as usize));
        assert_eq!(lines[(top + height) as usize], " ".repeat(cols as usize));
    }

    #[test]
    fn on_resize_re_centres_the_next_render_on_the_new_size() {
        let leaf = node("hi");
        let boxes = vec![leaf.clone()];
        let doc = Document {
            boxes,
            selected: None,
        };
        let mut r = renderer_on(terminal(20, 10, 1, 1));
        rendered(&mut r, &doc);
        let (cols, rows_count) = (40, 20);
        r.on_resize(terminal(cols, rows_count, 1, 1));

        let placements = crate::layout::layout(&doc.boxes);
        let mut screen = Screen::new(r.terminal);
        screen.centre_on(&placements, 0);
        draw_all(&mut r, &mut screen, &placements);

        let left = (cols - crate::layout::width(&leaf)).div_euclid(2);
        let top = (rows_count - crate::layout::BOX_HEIGHT).div_euclid(2);
        let label_x = left + crate::layout::centre(crate::layout::width(&leaf), "hi");
        let label_row = top + crate::layout::BOX_HEIGHT / 2;
        let label_cols: Vec<i64> = screen
            .images
            .iter()
            .filter(|image| image.row == label_row)
            .map(|image| image.col)
            .collect();
        assert_eq!(label_cols, vec![label_x, label_x + 1]);
    }

    #[test]
    fn columns_reports_the_terminals_column_count() {
        let r = renderer_on(terminal(20, 10, 1, 1));
        assert_eq!(r.columns(), 20);
    }

    #[test]
    fn columns_reflects_a_resize() {
        let mut r = renderer_on(terminal(20, 10, 1, 1));
        r.on_resize(terminal(40, 10, 1, 1));
        assert_eq!(r.columns(), 40);
    }

    #[test]
    fn status_text_for_insert_mode_is_editing() {
        assert_eq!(status_text(&Mode::Insert), "EDITING");
    }

    #[test]
    fn status_text_for_command_mode_is_commanding() {
        assert_eq!(status_text(&Mode::Command), "COMMANDING");
    }

    #[test]
    fn status_text_for_save_prompt_mode_shows_the_filename_and_cursor() {
        let mode = Mode::SavePrompt {
            filename: "diagram.dre".to_string(),
        };
        assert_eq!(status_text(&mode), format!("Save as: diagram.dre{CURSOR}"));
    }

    #[test]
    fn on_resize_with_a_different_rounded_cell_height_does_not_panic_on_the_next_render() {
        let mut r = renderer_on(terminal(40, 20, 2, 4));
        let node = box_node(None, false, false);
        let placement = box_placement(&node, 0, 0, 4, 3);
        sprites(&mut r, std::slice::from_ref(&placement));
        r.on_resize(terminal(40, 20, 2, 5));
        sprites(&mut r, &[placement]);
    }

    #[test]
    fn on_resize_leaves_the_sprite_cache_untouched() {
        let mut r = renderer_on(terminal(40, 20, 2, 4));
        sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 0, 0, 4, 3)],
        );
        assert_eq!(r.cache.len(), 1);
        r.on_resize(terminal(80, 40, 2, 4));
        assert_eq!(r.cache.len(), 1);
    }

    fn rendered(r: &mut TerminalRenderer, doc: &Document) -> String {
        rendered_with_scroll(r, doc, 0)
    }

    fn rendered_diagram(r: &mut TerminalRenderer, doc: &Document) -> String {
        let mut out = Vec::new();
        let mut state = State::default();
        state.doc = doc.clone();
        r.render_diagram(&state, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn rendered_with_scroll(r: &mut TerminalRenderer, doc: &Document, scroll_x: i64) -> String {
        let mut out = Vec::new();
        let mut state = State::default();
        state.doc = doc.clone();
        state.scroll_x = scroll_x;
        r.render(&state, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn empty_doc() -> Document {
        Document {
            boxes: vec![],
            selected: None,
        }
    }

    #[test]
    fn the_cursor_goes_home_before_the_lines() {
        let screen = Screen::new(Terminal {
            cols: 2,
            rows: 2,
            cell_width: 1,
            cell_height: 1,
        });
        assert_eq!(
            String::from_utf8(screen.into_bytes()).unwrap(),
            format!("{HOME_CURSOR}  \r\n  {}", kitty::clear())
        );
    }

    #[test]
    fn no_newline_follows_the_last_line() {
        let mut r = renderer_on(Terminal {
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
        let screen = Screen::new(Terminal {
            cols,
            rows,
            cell_width: 1,
            cell_height: 1,
        });
        let output = String::from_utf8(screen.into_bytes()).unwrap();
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
        let boxes = vec![node("hi")];
        let mut r = renderer_on(Terminal {
            cols: 20,
            rows: 10,
            cell_width: 1,
            cell_height: 1,
        });
        let selected = Document {
            boxes: boxes.clone(),
            selected: Some(crate::diagram::Path {
                ancestors: vec![],
                index: 0,
            }),
        };
        assert!(rendered(&mut r, &selected).contains(CURSOR));
    }

    #[test]
    fn no_cursor_is_drawn_without_a_selection() {
        let mut r = renderer_on(Terminal {
            cols: 20,
            rows: 10,
            cell_width: 1,
            cell_height: 1,
        });
        let unselected = Document {
            boxes: vec![node("hi")],
            selected: None,
        };
        assert!(!rendered(&mut r, &unselected).contains(CURSOR));
    }

    #[test]
    fn an_empty_canvas_renders_the_hint_box() {
        let terminal = terminal(40, 10, 1, 1);
        let mut r = renderer_on(terminal);
        let output = rendered_diagram(&mut r, &empty_doc());

        let mut expected_r = renderer_on(terminal);
        let hint_boxes = [hint_node()];
        let placements = crate::layout::layout(&hint_boxes);
        let mut screen = Screen::new(terminal);
        screen.centre_on(&placements, 0);
        draw_all(&mut expected_r, &mut screen, &placements);
        let expected = String::from_utf8(screen.into_bytes()).unwrap();

        assert_eq!(output, expected);
    }

    #[test]
    fn a_canvas_with_a_box_does_not_show_the_hint() {
        let terminal = terminal(40, 10, 1, 1);
        let mut r = renderer_on(terminal);
        let doc = Document {
            boxes: vec![node("hi")],
            selected: None,
        };
        let output = rendered_diagram(&mut r, &doc);

        let mut expected_r = renderer_on(terminal);
        let placements = with_cursor(crate::layout::layout(&doc.boxes), None);
        let mut screen = Screen::new(terminal);
        screen.centre_on(&placements, 0);
        draw_all(&mut expected_r, &mut screen, &placements);
        let expected = String::from_utf8(screen.into_bytes()).unwrap();
        assert_eq!(output, expected);

        let mut hint_r = renderer_on(terminal);
        let hint_output = rendered_diagram(&mut hint_r, &empty_doc());
        assert_ne!(output, hint_output);
    }

    #[test]
    fn no_cursor_is_drawn_over_the_hint_even_with_a_selection() {
        let mut r = renderer_on(terminal(20, 10, 1, 1));
        let doc = Document {
            boxes: vec![],
            selected: Some(crate::diagram::Path {
                ancestors: vec![],
                index: 0,
            }),
        };
        assert!(!rendered(&mut r, &doc).contains(CURSOR));
    }

    #[test]
    fn empty_canvas_fills_terminal() {
        let grid = grid(&mut renderer_on(terminal(11, 5, 1, 1)), &[]);
        assert_eq!(grid, vec![BLANK.to_string().repeat(11); 5]);
    }

    #[test]
    fn grid_matches_the_requested_size() {
        let (cols, rows) = (20, 7);
        let grid = grid(
            &mut renderer_on(terminal(cols, rows, 1, 1)),
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
            &mut renderer_on(terminal(11, 11, 1, 1)),
            &[box_placement(&box_node(None, false, false), 4, 4, 3, 3)],
        );
        assert_eq!(&grid[4][4..7], "   ");
    }

    #[test]
    fn a_box_reaching_past_the_edge_is_clipped() {
        let grid = grid(
            &mut renderer_on(terminal(4, 2, 1, 1)),
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
        let mut r = renderer_on(terminal(5, 3, 1, 1));
        let screen = drawn_screen(&mut r, &placements);
        let label_cols: Vec<i64> = screen
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
        let mut r = renderer_on(terminal(5, 3, 1, 1));
        let screen = drawn_screen(&mut r, &placements);
        assert_eq!(rows(&screen)[1], format!("   {} ", CURSOR));
        let label_cols: Vec<i64> = screen
            .images
            .iter()
            .filter(|image| image.row == 1)
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
        let mut r = renderer_on(terminal(3, 3, 1, 1));
        let screen = drawn_screen(&mut r, &placements);
        let label_cols: Vec<i64> = screen
            .images
            .iter()
            .filter(|image| image.row == 1)
            .map(|image| image.col)
            .collect();
        assert_eq!(label_cols, vec![1, 2]);
        assert!(!rows(&screen)[1].contains(CURSOR));
    }

    #[test]
    fn a_box_does_not_draw_a_cursor() {
        let grid = grid(
            &mut renderer_on(terminal(5, 3, 1, 1)),
            &[box_placement(&box_node(None, false, false), 0, 0, 5, 3)],
        );
        assert!(!grid.join("").contains(CURSOR));
    }

    #[test]
    fn cursor_placement_is_drawn_at_its_own_position() {
        let grid = grid(
            &mut renderer_on(terminal(4, 3, 1, 1)),
            &[cursor_placement(2, 1, 1, 1)],
        );
        assert_eq!(
            grid,
            vec![
                "    ".to_string(),
                format!("  {} ", CURSOR),
                "    ".to_string()
            ]
        );
    }

    #[test]
    fn a_cursor_outside_the_grid_is_clipped() {
        let grid = grid(
            &mut renderer_on(terminal(4, 3, 1, 1)),
            &[cursor_placement(9, 9, 1, 1)],
        );
        assert_eq!(grid, vec!["    ".to_string(); 3]);
    }

    #[test]
    fn an_arrow_leaves_the_gap_blank() {
        let grid = grid(
            &mut renderer_on(terminal(4, 4, 1, 1)),
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
        let grid = grid(&mut renderer_on(terminal(5, 3, 1, 1)), &placements);
        assert!(!grid.join("").contains('\x1b'));
    }

    #[test]
    fn a_coloured_box_puts_no_colour_in_the_grid() {
        let grid = grid(
            &mut renderer_on(terminal(5, 3, 1, 1)),
            &[box_placement(&box_node(Some(2), false, false), 0, 0, 5, 3)],
        );
        assert_eq!(grid, vec!["     ".to_string(); 3]);
    }

    #[test]
    fn a_cursor_has_no_sprite() {
        let mut r = renderer_on(terminal(40, 20, 2, 4));
        assert!(sprites(&mut r, &[cursor_placement(1, 1, 1, 1)]).is_empty());
    }

    #[test]
    fn a_box_off_screen_has_no_sprite() {
        let mut r = renderer_on(terminal(5, 20, 4, 4));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 10, 0, 4, 3)],
        );
        assert!(images.is_empty());
    }

    #[test]
    fn a_box_overhanging_the_left_is_cropped() {
        let mut r = renderer_on(terminal(40, 20, 4, 4));
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
        let mut r = renderer_on(terminal(40, 20, 4, 4));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 1, -2, 4, 5)],
        );
        assert_eq!(images[0].row, 0);
        assert_eq!(images[0].canvas.height, 3 * 4);
    }

    #[test]
    fn a_box_overhanging_the_right_is_cropped() {
        let mut r = renderer_on(terminal(4, 20, 4, 4));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 1, 0, 6, 3)],
        );
        assert_eq!(images[0].col, 1);
        assert_eq!(images[0].canvas.width, 3 * 4);
    }

    #[test]
    fn a_box_overhanging_the_bottom_is_cropped() {
        let mut r = renderer_on(terminal(40, 4, 4, 4));
        let images = sprites(
            &mut r,
            &[box_placement(&box_node(None, false, false), 0, 1, 3, 6)],
        );
        assert_eq!(images[0].row, 1);
        assert_eq!(images[0].canvas.height, 3 * 4);
    }

    #[test]
    fn a_box_sprite_sits_at_the_placement_cell() {
        let mut r = renderer_on(terminal(40, 20, 6, 12));
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
            let mut r = renderer_on(terminal(40, 20, 2, 4));
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
        let mut r = renderer_on(terminal(40, 20, 2, 4));
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
        let mut r = renderer_on(terminal(40, 20, 2, 4));
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
        let mut r = renderer_on(terminal(40, 20, 2, 4));
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
        let mut r = renderer_on(terminal(40, 20, 2, 4));
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
        let mut r = renderer_on(terminal(40, 20, 2, 4));
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
        let mut r = renderer_on(terminal(40, 20, 2, 4));
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
        let mut r = renderer_on(terminal(4, 20, 2, 4));
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
        let mut r = renderer_on(terminal(4, 4, 2, 4));
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
        let screen = Screen::new(terminal(20, 10, 4, 8));
        assert!(!screen.shows(&box_placement(&node, 20, 0, 4, 3)));
    }

    #[test]
    fn a_box_beyond_the_top_edge_is_not_shown() {
        let node = box_node(None, false, false);
        let screen = Screen::new(terminal(20, 10, 4, 8));
        assert!(!screen.shows(&box_placement(&node, 0, -3, 4, 3)));
    }

    #[test]
    fn a_box_straddling_an_edge_is_shown() {
        let node = box_node(None, false, false);
        let screen = Screen::new(terminal(20, 10, 4, 8));
        assert!(screen.shows(&box_placement(&node, 18, 0, 4, 3)));
        assert!(screen.shows(&box_placement(&node, -2, 8, 4, 3)));
    }

    #[test]
    fn arrows_with_different_stops_are_redrawn() {
        let mut r = renderer_on(terminal(40, 20, 2, 4));
        let one = sprites(&mut r, &[arrow_placement(vec![0], 0, 0, 0, 4, 6)]);
        let two = sprites(&mut r, &[arrow_placement(vec![0, 2], 0, 0, 0, 4, 6)]);
        assert_ne!(one[0].canvas.pixels, two[0].canvas.pixels);
    }

    #[test]
    fn the_cache_is_bounded() {
        let mut r = renderer_on(terminal(4000, 20, 2, 4));
        for width in 0..(CACHE_LIMIT as i64 + 2) {
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
        assert!(r.cache.len() <= CACHE_LIMIT);
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
    fn a_hint_boxs_edge_is_fainter_than_a_normal_boxs_edge() {
        let r = renderer(1, 1);
        let size = 2 * BORDER + 3;
        let normal = box_outline(&r, &box_node(None, false, false), size, size);
        let hint_node = crate::diagram::Node {
            hint: true,
            ..box_node(None, false, false)
        };
        let hint = box_outline(&r, &hint_node, size, size);
        let (_, _, _, normal_alpha) = pixel_of(&normal, 0, size / 2);
        let (_, _, _, hint_alpha) = pixel_of(&hint, 0, size / 2);
        assert!(hint_alpha < normal_alpha);
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
        let ink = (ink.0, ink.1, ink.2, OPAQUE);
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
        let ink = (ink.0, ink.1, ink.2, OPAQUE);
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
        let ink = (ink.0, ink.1, ink.2, OPAQUE);
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
            assert_eq!(pixel_of(&sprite, x, shaft_row).3, OPAQUE);
        }
    }

    #[test]
    fn arrow_off_shape_pixels_are_transparent() {
        let r = renderer(4, 5);
        let sprite = arrow_outline(&r, vec![0], 0, 2, 1);
        assert_eq!(pixel_of(&sprite, 0, 0), TRANSPARENT);
    }

    #[test]
    fn an_arrow_is_plain_grey() {
        let r = renderer(4, 5);
        let sprite = arrow_outline(&r, vec![0], 0, 2, 1);
        let (pr, pg, pb) = PLAIN_COLOUR;
        let shaft_row = sprite.height / 2;
        assert_eq!(pixel_of(&sprite, 0, shaft_row), (pr, pg, pb, OPAQUE));
    }

    #[test]
    fn a_branching_arrow_has_a_stub_at_every_stop() {
        let r = renderer(4, 5);
        let sprite = arrow_outline(&r, vec![0, 3], 0, 2, 4);
        let midpoint = sprite.width / 2;
        for stop in [0, 3] {
            let row = stop * 5 + 5 / 2;
            for x in midpoint..sprite.width {
                assert_eq!(pixel_of(&sprite, x, row).3, OPAQUE);
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
                OPAQUE
            } else {
                0
            };
            assert_eq!(pixel_of(&sprite, x, row_between_stops).3, expected);
        }
    }

    fn status_line(r: &mut TerminalRenderer, mode: Mode) -> String {
        let state = crate::state::new_state(vec![], mode, None);
        let mut out = Vec::new();
        r.render_status_line(&state, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn status_line_text(r: &mut TerminalRenderer, mode: Mode) -> String {
        let prefix = format!("\x1b[{};1H\x1b[7m", r.terminal.rows);
        status_line(r, mode)
            .strip_prefix(&prefix)
            .unwrap()
            .strip_suffix("\x1b[0m")
            .unwrap()
            .to_string()
    }

    #[test]
    fn the_status_line_shows_the_text_as_given() {
        let mut r = renderer_on(terminal(7, 2, 1, 1));
        assert_eq!(status_line_text(&mut r, Mode::Insert), "EDITING");
    }

    #[test]
    fn the_status_line_is_padded_to_the_terminal_width() {
        let mut r = renderer_on(terminal(10, 2, 1, 1));
        assert_eq!(
            status_line_text(&mut r, Mode::Insert),
            format!("EDITING{}", BLANK.to_string().repeat(3))
        );
    }

    #[test]
    fn the_status_line_is_cut_to_the_terminal_width() {
        let mut r = renderer_on(terminal(3, 2, 1, 1));
        assert_eq!(status_line_text(&mut r, Mode::Insert), "EDI");
    }

    #[test]
    fn the_status_line_is_written_to_the_last_row() {
        let mut r = renderer_on(terminal(5, 4, 1, 1));
        assert!(status_line(&mut r, Mode::Command).starts_with("\x1b[4;1H"));
    }

    #[test]
    fn on_resize_moves_the_status_line_to_the_new_last_row() {
        let mut r = renderer_on(terminal(5, 4, 1, 1));
        r.on_resize(terminal(5, 9, 1, 1));
        assert!(status_line(&mut r, Mode::Command).starts_with("\x1b[9;1H"));
    }

    #[test]
    fn the_status_line_is_wrapped_in_reverse_video() {
        let mut r = renderer_on(terminal(5, 4, 1, 1));
        let line = status_line(&mut r, Mode::Command);
        assert!(line.contains("\x1b[7m"));
        assert!(line.contains("\x1b[0m"));
    }

    fn render_output(r: &mut TerminalRenderer, mode: Mode) -> String {
        let state = crate::state::new_state(vec![], mode, None);
        let mut out = Vec::new();
        r.render(&state, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn render_shows_editing_in_insert_mode() {
        let mut r = renderer_on(terminal(20, 4, 1, 1));
        assert!(render_output(&mut r, Mode::Insert).contains("EDITING"));
    }

    #[test]
    fn render_shows_commanding_in_command_mode() {
        let mut r = renderer_on(terminal(20, 4, 1, 1));
        assert!(render_output(&mut r, Mode::Command).contains("COMMANDING"));
    }

    #[test]
    fn render_shows_the_filename_being_typed_in_save_prompt_mode() {
        let mut r = renderer_on(terminal(30, 4, 1, 1));
        let mode = Mode::SavePrompt {
            filename: "diagram.dre".to_string(),
        };
        assert!(render_output(&mut r, mode).contains(&format!("Save as: diagram.dre{CURSOR}")));
    }
}
