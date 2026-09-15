//! Colour/cell helpers and the `Canvas` pixel-buffer primitive, ported from
//! `dre/render.py`. Later slices add `Sprite`/`TerminalRenderer`,
//! `RoundedBox`, and the arrow/box pixel generators.

pub(crate) const BLANK: char = ' ';
pub(crate) const CURSOR: char = '\u{2588}';

pub(crate) const ARROW_STROKE: i64 = 4;
pub(crate) const ARROWHEAD_ANGLE_DEG: f64 = 30.0;
pub(crate) const ARROWHEAD_EDGE_LENGTH: f64 = 15.0;
// The arrowhead is a fixed shape, so its depth and slope are constants rather
// than trigonometry repeated for every pixel.
pub(crate) fn arrowhead_depth() -> f64 {
    ARROWHEAD_EDGE_LENGTH * ARROWHEAD_ANGLE_DEG.to_radians().cos()
}
pub(crate) fn arrowhead_slope() -> f64 {
    ARROWHEAD_ANGLE_DEG.to_radians().tan()
}
pub(crate) const RESET: &str = "\x1b[0m";

// Distinct shapes on a board are few; this only bounds a pathological run.
#[allow(dead_code)]
pub(crate) const CACHE_LIMIT: usize = 512;

pub(crate) const ROUNDED_RADIUS: i64 = 20;
// Boxes always have a bold border; thickness is fixed, not configurable.
pub(crate) const BORDER: i64 = 4;

pub(crate) const OPAQUE: u8 = 255;
pub(crate) const FILL_ALPHA: u16 = 77;
pub(crate) const TRANSPARENT: (u8, u8, u8, u8) = (0, 0, 0, 0);
pub(crate) const PLAIN_COLOUR: (u8, u8, u8) = (128, 128, 128);
pub(crate) const PALETTE: [(u8, u8, u8); 5] = [
    (255, 190, 11),
    (251, 86, 7),
    (255, 0, 110),
    (131, 56, 236),
    (58, 134, 255),
];

/// Range of `width` integers centred on `c`, matching Python's
/// `_centered_span`.
pub(crate) fn centered_span(c: i64, width: i64) -> std::ops::Range<i64> {
    let start = c - (width - 1).div_euclid(2);
    start..(start + width)
}

/// Resolves a palette index (or `PLAIN`) to an RGB colour.
pub(crate) fn colour(colour: i64) -> (u8, u8, u8) {
    if colour == crate::PLAIN {
        PLAIN_COLOUR
    } else {
        PALETTE[colour as usize]
    }
}

/// Resolves a palette index (or `PLAIN`) to a fill RGBA colour: `PLAIN` is
/// fully transparent, any other index is the palette colour alpha-composited
/// at `FILL_ALPHA` and reported fully opaque.
pub(crate) fn fill_colour(fill: i64) -> (u8, u8, u8, u8) {
    if fill == crate::PLAIN {
        TRANSPARENT
    } else {
        let (r, g, b) = PALETTE[fill as usize];
        let composite =
            |channel: u8| (channel as f64 * FILL_ALPHA as f64 / OPAQUE as f64).round() as u8;
        (composite(r), composite(g), composite(b), OPAQUE)
    }
}

/// Builds the SGR-escaped grid cell for `character`, `colour`, and `fill`,
/// matching Python's `_cell`.
pub(crate) fn cell(character: char, colour: i64, fill: i64) -> String {
    let mut codes = Vec::new();
    if colour != crate::PLAIN {
        codes.push(30 + colour);
    }
    if fill != crate::PLAIN {
        codes.push(40 + fill);
    }
    if codes.is_empty() {
        return character.to_string();
    }
    let joined = codes
        .iter()
        .map(|code| code.to_string())
        .collect::<Vec<_>>()
        .join(";");
    format!("\x1b[{joined}m{character}{RESET}")
}

/// A clipped, transparent pixel field that lines are stroked onto.
pub(crate) struct Canvas {
    pub(crate) first_x: i64,
    pub(crate) last_x: i64,
    pub(crate) first_y: i64,
    pub(crate) last_y: i64,
    ink: [u8; 4],
    span: i64,
    buffer: Vec<u8>,
}

impl Canvas {
    pub(crate) fn new(first_x: i64, last_x: i64, first_y: i64, last_y: i64, ink: [u8; 4]) -> Self {
        let span = last_x - first_x;
        let buffer = vec![0u8; (span * (last_y - first_y) * 4) as usize];
        Canvas {
            first_x,
            last_x,
            first_y,
            last_y,
            ink,
            span,
            buffer,
        }
    }

    pub(crate) fn point(&mut self, x: i64, y: i64, width: i64) {
        for px in centered_span(x, width) {
            for py in centered_span(y, width) {
                if self.first_x <= px && px < self.last_x && self.first_y <= py && py < self.last_y
                {
                    let start = self.offset(px, py);
                    self.buffer[start..start + 4].copy_from_slice(&self.ink);
                }
            }
        }
    }

    pub(crate) fn horizontal(&mut self, y: i64, x0: i64, x1: i64, width: i64) {
        let start_x = x0.max(self.first_x);
        let stop_x = (x1 + 1).min(self.last_x);
        if start_x >= stop_x {
            return;
        }
        for py in centered_span(y, width) {
            if !(self.first_y <= py && py < self.last_y) {
                continue;
            }
            let start = self.offset(start_x, py);
            for i in 0..(stop_x - start_x) as usize {
                self.buffer[start + i * 4..start + i * 4 + 4].copy_from_slice(&self.ink);
            }
        }
    }

    pub(crate) fn vertical(&mut self, x: i64, y0: i64, y1: i64, width: i64) {
        for px in centered_span(x, width) {
            if !(self.first_x <= px && px < self.last_x) {
                continue;
            }
            for y in y0.max(self.first_y)..(y1 + 1).min(self.last_y) {
                let start = self.offset(px, y);
                self.buffer[start..start + 4].copy_from_slice(&self.ink);
            }
        }
    }

    pub(crate) fn pixels(&self) -> Vec<u8> {
        self.buffer.clone()
    }

    fn offset(&self, x: i64, y: i64) -> usize {
        (((y - self.first_y) * self.span + (x - self.first_x)) * 4) as usize
    }
}

/// Rounds like Python's `round`: half-to-even, rather than Rust's
/// half-away-from-zero. Values here are always small and non-negative, so a
/// simple floor-based implementation is sufficient.
fn python_round(value: f64) -> f64 {
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

/// Builds one row of a box's flat body: edge pixels on the left/right where
/// the border overlaps this span, fill pixels in between. Matches Python's
/// `_body_row`.
pub(crate) fn body_row(
    width: i64,
    border: i64,
    edge: (u8, u8, u8, u8),
    fill: (u8, u8, u8, u8),
    first_x: i64,
    last_x: i64,
) -> Vec<u8> {
    let span = last_x - first_x;
    let edge_px = [edge.0, edge.1, edge.2, edge.3];
    let fill_px = [fill.0, fill.1, fill.2, fill.3];
    if width <= 2 * border {
        return edge_px.repeat(span.max(0) as usize);
    }
    let mut row = Vec::new();
    let left_edge = (border - first_x).max(0);
    row.extend(edge_px.repeat(left_edge as usize));
    let right_edge = (border - (width - last_x)).max(0);
    let fill_count = (span - left_edge - right_edge).max(0);
    row.extend(fill_px.repeat(fill_count as usize));
    row.extend(edge_px.repeat(right_edge as usize));
    row
}

/// Builds the pixels of a square (non-rounded) box: only two kinds of row
/// exist, so each is built once and repeated. Matches Python's
/// `_square_pixels`.
pub(crate) fn square_pixels(
    width: i64,
    height: i64,
    border: i64,
    edge: (u8, u8, u8, u8),
    fill: (u8, u8, u8, u8),
    first_x: i64,
    last_x: i64,
    first_y: i64,
    last_y: i64,
) -> Vec<u8> {
    let edge_px = [edge.0, edge.1, edge.2, edge.3];
    let edge_row = edge_px.repeat((last_x - first_x).max(0) as usize);
    let body = body_row(width, border, edge, fill, first_x, last_x);
    let mut pixels = Vec::new();
    if height <= 2 * border {
        pixels.extend(edge_row.repeat((last_y - first_y).max(0) as usize));
    } else {
        let top_edge = (border - first_y).max(0);
        pixels.extend(edge_row.repeat(top_edge as usize));
        let bottom_edge = (border - (height - last_y)).max(0);
        let body_count = ((last_y - first_y) - top_edge - bottom_edge).max(0);
        pixels.extend(body.repeat(body_count as usize));
        pixels.extend(edge_row.repeat(bottom_edge as usize));
    }
    pixels
}

/// A box whose corners are cut from a rounded-rectangle distance field.
/// Matches Python's `RoundedBox`.
pub(crate) struct RoundedBox {
    width: i64,
    height: i64,
    border: i64,
    radius: i64,
    outer: i64,
    edge: (u8, u8, u8, u8),
    fill: (u8, u8, u8, u8),
}

impl RoundedBox {
    pub(crate) fn new(
        width: i64,
        height: i64,
        radius: i64,
        border: i64,
        edge: (u8, u8, u8, u8),
        fill: (u8, u8, u8, u8),
    ) -> Self {
        let outer = (radius + border).min(width / 2).min(height / 2);
        RoundedBox { width, height, border, radius, outer, edge, fill }
    }

    pub(crate) fn pixels(&self, first_x: i64, last_x: i64, first_y: i64, last_y: i64) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut straight_row: Option<Vec<u8>> = None;
        for y in first_y..last_y {
            if self.outer <= y && y < self.height - self.outer {
                if straight_row.is_none() {
                    straight_row = Some(body_row(
                        self.width, self.border, self.edge, self.fill, first_x, last_x,
                    ));
                }
                buffer.extend(straight_row.as_ref().unwrap());
            } else {
                buffer.extend(self.corner_row(y, first_x, last_x));
            }
        }
        buffer
    }

    fn corner_row(&self, y: i64, first_x: i64, last_x: i64) -> Vec<u8> {
        let mut row = Vec::new();
        for x in first_x..self.outer.min(last_x) {
            row.extend(self.pixel(x, y));
        }
        let middle = (self.width - self.outer).min(last_x) - self.outer.max(first_x);
        if middle > 0 {
            let straight = if y < self.border || y >= self.height - self.border {
                self.edge
            } else {
                self.fill
            };
            let straight_px = [straight.0, straight.1, straight.2, straight.3];
            row.extend(straight_px.repeat(middle as usize));
        }
        for x in (self.width - self.outer).max(first_x)..last_x {
            row.extend(self.pixel(x, y));
        }
        row
    }

    fn pixel(&self, x: i64, y: i64) -> [u8; 4] {
        let px = x as f64 + 0.5;
        let py = y as f64 + 0.5;
        let outer_coverage =
            Self::coverage(px, py, self.width as f64, self.height as f64, self.outer as f64);
        let inner_coverage = Self::coverage(
            px - self.border as f64,
            py - self.border as f64,
            (self.width - 2 * self.border) as f64,
            (self.height - 2 * self.border) as f64,
            self.radius as f64,
        );
        let edge_coverage = outer_coverage - inner_coverage;
        let alpha = edge_coverage * self.edge.3 as f64 + inner_coverage * self.fill.3 as f64;
        if alpha == 0.0 {
            return [0, 0, 0, 0];
        }
        let edge_channels = [self.edge.0, self.edge.1, self.edge.2];
        let fill_channels = [self.fill.0, self.fill.1, self.fill.2];
        let mut channels = [0u8; 4];
        for c in 0..3 {
            let value = (edge_channels[c] as f64 * edge_coverage * self.edge.3 as f64
                + fill_channels[c] as f64 * inner_coverage * self.fill.3 as f64)
                / alpha;
            channels[c] = python_round(value) as u8;
        }
        channels[3] = python_round(alpha) as u8;
        channels
    }

    fn coverage(px: f64, py: f64, width: f64, height: f64, radius: f64) -> f64 {
        let half_x = width / 2.0;
        let half_y = height / 2.0;
        let qx = (px - half_x).abs() - (half_x - radius);
        let qy = (py - half_y).abs() - (half_y - radius);
        let distance = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius;
        (0.5 - distance).max(0.0).min(1.0)
    }
}

/// The cache key for a box/arrow sprite: the shape and crop that determine
/// its pixels, independent of the placement's on-screen position. Matches
/// Python's `_key`. `HashMap`-suitable (`Eq` + `Hash`), unlike Python's tuple
/// key only because Rust requires the trait bound to be explicit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum SpriteKey {
    Box {
        width: i64,
        height: i64,
        left: i64,
        top: i64,
        right: i64,
        bottom: i64,
        colour: i64,
        fill: i64,
        rounded: bool,
    },
    Arrow {
        width: i64,
        height: i64,
        left: i64,
        top: i64,
        right: i64,
        bottom: i64,
        stops: Vec<i64>,
        shaft: i64,
    },
}

/// Builds the sprite cache key for a placement's crop. Only meaningful for
/// `Box`/`Arrow` placements, mirroring the only placements `TerminalRenderer`
/// ever sprites.
pub(crate) fn sprite_key(
    placement: &crate::layout::Placement,
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
) -> SpriteKey {
    use crate::layout::PlacementNode;
    match &placement.node {
        PlacementNode::Node(node) => SpriteKey::Box {
            width: placement.width,
            height: placement.height,
            left: left - placement.x,
            top: top - placement.y,
            right: right - placement.x,
            bottom: bottom - placement.y,
            colour: node.colour,
            fill: node.fill,
            rounded: node.rounded,
        },
        PlacementNode::Arrow(arrow) => SpriteKey::Arrow {
            width: placement.width,
            height: placement.height,
            left: left - placement.x,
            top: top - placement.y,
            right: right - placement.x,
            bottom: bottom - placement.y,
            stops: arrow.stops.clone(),
            shaft: arrow.shaft,
        },
        _ => unreachable!("sprite_key is only called for Box and Arrow placements"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ink() -> [u8; 4] {
        let (r, g, b) = colour(1);
        [r, g, b, OPAQUE]
    }

    fn canvas(first_x: i64, last_x: i64, first_y: i64, last_y: i64) -> Canvas {
        Canvas::new(first_x, last_x, first_y, last_y, ink())
    }

    fn pixel(canvas: &Canvas, x: i64, y: i64) -> (u8, u8, u8, u8) {
        let offset = (((y - canvas.first_y) * canvas.span + (x - canvas.first_x)) * 4) as usize;
        (
            canvas.buffer[offset],
            canvas.buffer[offset + 1],
            canvas.buffer[offset + 2],
            canvas.buffer[offset + 3],
        )
    }

    fn ink_pixel() -> (u8, u8, u8, u8) {
        let [r, g, b, a] = ink();
        (r, g, b, a)
    }

    fn blank() -> (u8, u8, u8, u8) {
        TRANSPARENT
    }

    #[test]
    fn point_with_width_one_stamps_a_single_pixel() {
        let mut canvas = canvas(0, 20, 0, 20);
        canvas.point(10, 10, 1);
        assert_eq!(pixel(&canvas, 10, 10), ink_pixel());
        for (x, y) in [(9, 10), (11, 10), (10, 9), (10, 11)] {
            assert_eq!(pixel(&canvas, x, y), blank());
        }
    }

    #[test]
    fn point_with_width_four_stamps_a_four_by_four_block() {
        let mut canvas = canvas(0, 20, 0, 20);
        canvas.point(10, 10, 4);
        for x in [9, 10, 11, 12] {
            for y in [9, 10, 11, 12] {
                assert_eq!(pixel(&canvas, x, y), ink_pixel());
            }
        }
        for (x, y) in [(8, 10), (13, 10), (10, 8), (10, 13)] {
            assert_eq!(pixel(&canvas, x, y), blank());
        }
    }

    #[test]
    fn horizontal_with_width_four_paints_four_rows() {
        let mut canvas = canvas(0, 20, 0, 20);
        canvas.horizontal(10, 2, 6, 4);
        for y in [9, 10, 11, 12] {
            for x in 2..7 {
                assert_eq!(pixel(&canvas, x, y), ink_pixel());
            }
        }
        for y in [8, 13] {
            for x in 2..7 {
                assert_eq!(pixel(&canvas, x, y), blank());
            }
        }
        assert_eq!(pixel(&canvas, 1, 10), blank());
        assert_eq!(pixel(&canvas, 7, 10), blank());
    }

    #[test]
    fn vertical_with_width_four_paints_four_columns() {
        let mut canvas = canvas(0, 20, 0, 20);
        canvas.vertical(10, 2, 6, 4);
        for x in [9, 10, 11, 12] {
            for y in 2..7 {
                assert_eq!(pixel(&canvas, x, y), ink_pixel());
            }
        }
        for x in [8, 13] {
            for y in 2..7 {
                assert_eq!(pixel(&canvas, x, y), blank());
            }
        }
        assert_eq!(pixel(&canvas, 10, 1), blank());
        assert_eq!(pixel(&canvas, 10, 7), blank());
    }

    #[test]
    fn a_thick_point_near_the_edge_is_clipped() {
        let mut canvas = canvas(0, 20, 0, 20);
        canvas.point(0, 0, 4);
        for x in [0, 1] {
            for y in [0, 1] {
                assert_eq!(pixel(&canvas, x, y), ink_pixel());
            }
        }
    }

    #[test]
    fn a_thick_horizontal_near_the_edge_is_clipped() {
        let mut canvas = canvas(0, 20, 0, 20);
        canvas.horizontal(0, 2, 6, 4);
        for x in 2..7 {
            assert_eq!(pixel(&canvas, x, 0), ink_pixel());
            assert_eq!(pixel(&canvas, x, 1), ink_pixel());
        }
    }

    #[test]
    fn a_thick_vertical_near_the_edge_is_clipped() {
        let mut canvas = canvas(0, 20, 0, 20);
        canvas.vertical(0, 2, 6, 4);
        for y in 2..7 {
            assert_eq!(pixel(&canvas, 0, y), ink_pixel());
            assert_eq!(pixel(&canvas, 1, y), ink_pixel());
        }
    }

    #[test]
    fn colour_of_plain_is_the_plain_grey() {
        assert_eq!(colour(crate::PLAIN), PLAIN_COLOUR);
    }

    #[test]
    fn colour_of_a_palette_index_is_the_palette_entry() {
        assert_eq!(colour(2), PALETTE[2]);
    }

    #[test]
    fn fill_colour_of_plain_is_transparent() {
        assert_eq!(fill_colour(crate::PLAIN), TRANSPARENT);
    }

    #[test]
    fn fill_colour_of_a_palette_index_is_alpha_composited_and_opaque() {
        let (r, g, b) = PALETTE[2];
        let round = |channel: u8| (channel as f64 * FILL_ALPHA as f64 / OPAQUE as f64).round() as u8;
        let expected = (round(r), round(g), round(b), OPAQUE);
        assert_eq!(fill_colour(2), expected);
    }

    #[test]
    fn cell_with_plain_colour_and_fill_is_the_bare_character() {
        assert_eq!(cell('x', crate::PLAIN, crate::PLAIN), "x");
    }

    #[test]
    fn cell_with_a_colour_only_emits_a_foreground_code() {
        assert_eq!(cell('x', 2, crate::PLAIN), "\x1b[32mx\x1b[0m");
    }

    #[test]
    fn cell_with_a_fill_only_emits_a_background_code() {
        assert_eq!(cell('x', crate::PLAIN, 3), "\x1b[43mx\x1b[0m");
    }

    #[test]
    fn cell_with_colour_and_fill_emits_both_codes() {
        assert_eq!(cell('x', 1, 4), "\x1b[31;44mx\x1b[0m");
    }

    fn edge_rgba(index: i64) -> (u8, u8, u8, u8) {
        let (r, g, b) = colour(index);
        (r, g, b, OPAQUE)
    }

    fn pixel_at(pixels: &[u8], width: i64, x: i64, y: i64) -> (u8, u8, u8, u8) {
        let offset = ((y * width + x) * 4) as usize;
        (pixels[offset], pixels[offset + 1], pixels[offset + 2], pixels[offset + 3])
    }

    // -- square_pixels / body_row: fill compositing (mirrors
    // TerminalRendererFillTest, cell_width=cell_height=1) --

    #[test]
    fn plain_fill_renders_transparent_interior() {
        let size = 2 * BORDER + 3;
        let edge = edge_rgba(crate::PLAIN);
        let fill = fill_colour(crate::PLAIN);
        let pixels = square_pixels(size, size, BORDER, edge, fill, 0, size, 0, size);
        assert_eq!(pixel_at(&pixels, size, BORDER + 1, BORDER + 1), TRANSPARENT);
    }

    #[test]
    fn a_fill_colour_is_composited_over_black_and_made_opaque() {
        let size = 2 * BORDER + 3;
        let edge = edge_rgba(crate::PLAIN);
        let fill = fill_colour(2);
        let pixels = square_pixels(size, size, BORDER, edge, fill, 0, size, 0, size);
        assert_eq!(pixel_at(&pixels, size, BORDER + 1, BORDER + 1), fill_colour(2));
    }

    #[test]
    fn border_pixels_are_unaffected_by_fill() {
        let size = 2 * BORDER + 3;
        let edge = edge_rgba(3);
        let fill = fill_colour(2);
        let pixels = square_pixels(size, size, BORDER, edge, fill, 0, size, 0, size);
        assert_eq!(pixel_at(&pixels, size, 0, 0), edge_rgba(3));
        assert_eq!(pixel_at(&pixels, size, BORDER + 1, BORDER + 1), fill_colour(2));
    }

    // -- square_pixels / body_row: border thickness (mirrors
    // TerminalRendererBorderTest, cell_width=cell_height=4) --

    #[test]
    fn a_border_is_bold_at_every_edge() {
        let size = 3 * 4;
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let pixels = square_pixels(size, size, BORDER, edge, fill, 0, size, 0, size);
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

    // -- RoundedBox: corner radius (mirrors TerminalRendererCornerRadiusTest,
    // cell_width=cell_height=2*ROUNDED_RADIUS/5=8, box is 10x10 cells) --

    const CORNER_SIZE: i64 = 80;

    #[test]
    fn a_square_box_is_built_from_flat_edge_and_body_rows() {
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let pixels =
            square_pixels(CORNER_SIZE, CORNER_SIZE, BORDER, edge, fill, 0, CORNER_SIZE, 0, CORNER_SIZE);

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
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let rounded = RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill);
        let pixels = rounded.pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
        let (last_x, last_y) = (CORNER_SIZE - 1, CORNER_SIZE - 1);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, 0, 0), TRANSPARENT);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, last_x, 0), TRANSPARENT);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, 0, last_y), TRANSPARENT);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, last_x, last_y), TRANSPARENT);
    }

    #[test]
    fn straight_edges_stay_as_crisp_as_a_square_box() {
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let square = square_pixels(CORNER_SIZE, CORNER_SIZE, BORDER, edge, fill, 0, CORNER_SIZE, 0, CORNER_SIZE);
        let rounded =
            RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill)
                .pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
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
        let edge = edge_rgba(1);
        let fill = TRANSPARENT;
        let square = square_pixels(CORNER_SIZE, CORNER_SIZE, BORDER, edge, fill, 0, CORNER_SIZE, 0, CORNER_SIZE);
        let rounded =
            RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill)
                .pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
        assert!(!square.iter().skip(3).step_by(4).any(|&alpha| alpha > 0 && alpha < OPAQUE));
        assert!(rounded.iter().skip(3).step_by(4).any(|&alpha| alpha > 0 && alpha < OPAQUE));
    }

    #[test]
    fn arc_coverage_is_continuous_at_the_pixel_centre() {
        let edge = edge_rgba(1);
        let fill = TRANSPARENT;
        let pixels = RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill)
            .pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
        let (r, g, b) = colour(1);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, 14, 2), (r, g, b, 254));
    }

    #[test]
    fn border_coverage_is_composed_over_the_opaque_fill() {
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let pixels = RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill)
            .pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, 20, 4), (131, 27, 25, OPAQUE));
    }

    #[test]
    fn a_rounded_box_cuts_away_more_than_a_square_one() {
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let square = square_pixels(CORNER_SIZE, CORNER_SIZE, BORDER, edge, fill, 0, CORNER_SIZE, 0, CORNER_SIZE);
        let rounded =
            RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill)
                .pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
        let alpha_total = |pixels: &[u8]| pixels.iter().skip(3).step_by(4).map(|&a| a as u64).sum::<u64>();
        assert!(alpha_total(&rounded) < alpha_total(&square));
    }

    #[test]
    fn the_fringe_keeps_the_edge_colour_instead_of_fading_to_black() {
        let edge = edge_rgba(1);
        let fill = TRANSPARENT;
        let pixels = RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill)
            .pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
        let (r, g, b) = colour(1);
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

    #[test]
    fn clipping_a_rounded_box_is_a_pure_crop_of_the_whole_box() {
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let cell_width = 8;
        let hidden_cols = 2;
        let offset = hidden_cols * cell_width;
        let rounded_box = RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill);
        let whole = rounded_box.pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
        let clipped = rounded_box.pixels(offset, CORNER_SIZE, 0, CORNER_SIZE);
        let clipped_width = CORNER_SIZE - offset;
        for y in 0..CORNER_SIZE {
            for x in 0..clipped_width {
                assert_eq!(
                    pixel_at(&clipped, clipped_width, x, y),
                    pixel_at(&whole, CORNER_SIZE, x + offset, y)
                );
            }
        }
    }

    // -- RoundedBox: small box where corner bands overlap (mirrors
    // TerminalRendererSmallBoxTest, cell = ROUNDED_RADIUS/2 = 10, box is 2x2
    // cells) --

    const SMALL_SIZE: i64 = 20;

    #[test]
    fn the_sprite_holds_exactly_one_pixel_per_cell_of_its_area() {
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let pixels = RoundedBox::new(SMALL_SIZE, SMALL_SIZE, ROUNDED_RADIUS, BORDER, edge, fill)
            .pixels(0, SMALL_SIZE, 0, SMALL_SIZE);
        assert_eq!(pixels.len() as i64, SMALL_SIZE * SMALL_SIZE * 4);
    }

    #[test]
    fn clipping_a_small_box_is_a_pure_crop_of_the_whole_box() {
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let cell_width = 10;
        let hidden_cols = 1;
        let offset = hidden_cols * cell_width;
        let small_box = RoundedBox::new(SMALL_SIZE, SMALL_SIZE, ROUNDED_RADIUS, BORDER, edge, fill);
        let whole = small_box.pixels(0, SMALL_SIZE, 0, SMALL_SIZE);
        let clipped = small_box.pixels(offset, SMALL_SIZE, 0, SMALL_SIZE);
        let clipped_width = SMALL_SIZE - offset;
        assert_eq!(clipped.len() as i64, clipped_width * SMALL_SIZE * 4);
        for y in 0..SMALL_SIZE {
            for x in 0..clipped_width {
                assert_eq!(
                    pixel_at(&clipped, clipped_width, x, y),
                    pixel_at(&whole, SMALL_SIZE, x + offset, y)
                );
            }
        }
    }

    // -- sprite_key --

    fn box_node(colour: i64, fill: i64, rounded: bool) -> crate::Node {
        crate::Node { label: String::new(), colour, fill, rounded, children: vec![] }
    }

    fn box_placement(node: crate::Node, x: i64, y: i64, width: i64, height: i64) -> crate::layout::Placement {
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
    ) -> crate::layout::Placement {
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
        let a = box_placement(box_node(1, 2, true), 0, 0, 10, 10);
        let b = box_placement(box_node(1, 2, true), 0, 0, 10, 10);
        assert_eq!(sprite_key(&a, 0, 0, 10, 10), sprite_key(&b, 0, 0, 10, 10));
    }

    #[test]
    fn sprite_key_differs_by_colour() {
        let a = box_placement(box_node(1, 2, true), 0, 0, 10, 10);
        let b = box_placement(box_node(2, 2, true), 0, 0, 10, 10);
        assert_ne!(sprite_key(&a, 0, 0, 10, 10), sprite_key(&b, 0, 0, 10, 10));
    }

    #[test]
    fn sprite_key_differs_by_fill() {
        let a = box_placement(box_node(1, 2, true), 0, 0, 10, 10);
        let b = box_placement(box_node(1, 3, true), 0, 0, 10, 10);
        assert_ne!(sprite_key(&a, 0, 0, 10, 10), sprite_key(&b, 0, 0, 10, 10));
    }

    #[test]
    fn sprite_key_differs_by_rounded() {
        let a = box_placement(box_node(1, 2, true), 0, 0, 10, 10);
        let b = box_placement(box_node(1, 2, false), 0, 0, 10, 10);
        assert_ne!(sprite_key(&a, 0, 0, 10, 10), sprite_key(&b, 0, 0, 10, 10));
    }

    #[test]
    fn sprite_key_differs_by_crop() {
        let a = box_placement(box_node(1, 2, true), 0, 0, 10, 10);
        assert_ne!(sprite_key(&a, 0, 0, 10, 10), sprite_key(&a, 1, 0, 10, 10));
    }

    #[test]
    fn sprite_key_of_arrows_with_different_stops_differs() {
        let a = arrow_placement(vec![0, 2], 1, 0, 0, 4, 3);
        let b = arrow_placement(vec![0, 3], 1, 0, 0, 4, 3);
        assert_ne!(sprite_key(&a, 0, 0, 4, 3), sprite_key(&b, 0, 0, 4, 3));
    }

    #[test]
    fn sprite_key_of_identical_arrows_is_equal() {
        let a = arrow_placement(vec![0, 2], 1, 0, 0, 4, 3);
        let b = arrow_placement(vec![0, 2], 1, 0, 0, 4, 3);
        assert_eq!(sprite_key(&a, 0, 0, 4, 3), sprite_key(&b, 0, 0, 4, 3));
    }
}
