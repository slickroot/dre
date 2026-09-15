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
}
