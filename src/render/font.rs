use std::collections::HashMap;

use crate::canvas::{Canvas, Rgba, Shape};
use crate::render::{OPAQUE, PLAIN_COLOUR};

const FONT_BYTES: &[u8] = include_bytes!("../../assets/IosevkaRegular.ttf");
const REFERENCE_PX_SIZE: f32 = 100.0;

pub(super) struct GlyphCache {
    font: fontdue::Font,
    cache: HashMap<(char, bool), Canvas>,
    cell_width: i64,
    cell_height: i64,
    px_size: f32,
    baseline_row: i64,
}

impl GlyphCache {
    pub(super) fn new(cell_width: i64, cell_height: i64) -> GlyphCache {
        let font = fontdue::Font::from_bytes(FONT_BYTES, fontdue::FontSettings::default())
            .expect("bundled Iosevka font must parse");

        // Iosevka is monospace, so every glyph shares one advance width: pick the
        // pixel size that makes that advance width equal cell_width, once, up front.
        // Also cap it so the font's full ascent+descent fits within cell_height,
        // otherwise descenders (g, q, y, p, j) get clipped at the bottom of the cell.
        let reference_metrics = font.metrics('M', REFERENCE_PX_SIZE);
        let width_px_size = REFERENCE_PX_SIZE * cell_width as f32 / reference_metrics.advance_width;

        let reference_line_metrics = font
            .horizontal_line_metrics(REFERENCE_PX_SIZE)
            .expect("Iosevka must provide horizontal line metrics");
        let reference_line_height = reference_line_metrics.ascent - reference_line_metrics.descent;
        let height_px_size = REFERENCE_PX_SIZE * cell_height as f32 / reference_line_height;

        let px_size = width_px_size.min(height_px_size);

        let line_metrics = font
            .horizontal_line_metrics(px_size)
            .expect("Iosevka must provide horizontal line metrics");
        let baseline_row = line_metrics.ascent.round() as i64;

        GlyphCache {
            font,
            cache: HashMap::new(),
            cell_width,
            cell_height,
            px_size,
            baseline_row,
        }
    }

    pub(super) fn glyph(&mut self, ch: char, hint: bool) -> &Canvas {
        let key = (ch, hint);
        if !self.cache.contains_key(&key) {
            let canvas = self.rasterize(ch, hint);
            self.cache.insert(key, canvas);
        }
        self.cache.get(&key).unwrap()
    }

    fn rasterize(&self, ch: char, hint: bool) -> Canvas {
        let (metrics, bitmap) = self.font.rasterize(ch, self.px_size);

        let dest_x0 = metrics.xmin as i64;
        let dest_y0 = self.baseline_row - metrics.ymin as i64 - metrics.height as i64;

        let mut coverage = vec![0u8; (self.cell_width * self.cell_height) as usize];
        for row in 0..metrics.height as i64 {
            let dest_y = dest_y0 + row;
            if dest_y < 0 || dest_y >= self.cell_height {
                continue;
            }
            for col in 0..metrics.width as i64 {
                let dest_x = dest_x0 + col;
                if dest_x < 0 || dest_x >= self.cell_width {
                    continue;
                }
                let source_index = (row * metrics.width as i64 + col) as usize;
                let dest_index = (dest_y * self.cell_width + dest_x) as usize;
                coverage[dest_index] = bitmap[source_index];
            }
        }

        let (r, g, b) = PLAIN_COLOUR;
        let ink: Rgba = [r, g, b, if hint { OPAQUE / 4 } else { OPAQUE }];
        Canvas::fill(
            self.cell_width,
            self.cell_height,
            &GlyphShape {
                width: self.cell_width,
                height: self.cell_height,
                coverage,
                ink,
            },
        )
    }
}

pub(super) struct GlyphShape {
    pub(super) width: i64,
    pub(super) height: i64,
    pub(super) coverage: Vec<u8>,
    pub(super) ink: Rgba,
}

impl Shape for GlyphShape {
    fn colour_at(&self, x: i64, y: i64) -> Option<Rgba> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        let coverage = self.coverage[(y * self.width + x) as usize];
        if coverage == 0 {
            return None;
        }
        let mut colour = self.ink;
        colour[3] = (self.ink[3] as u16 * coverage as u16 / 255) as u8;
        Some(colour)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{CELL_HEIGHT, CELL_WIDTH, OPAQUE};

    const INK: Rgba = [10, 20, 30, OPAQUE];

    fn glyph_shape(width: i64, height: i64, coverage: Vec<u8>) -> GlyphShape {
        GlyphShape {
            width,
            height,
            coverage,
            ink: INK,
        }
    }

    #[test]
    fn a_fully_covered_pixel_has_full_alpha_ink() {
        let shape = glyph_shape(1, 1, vec![255]);
        assert_eq!(shape.colour_at(0, 0), Some(INK));
    }

    #[test]
    fn a_zero_coverage_pixel_has_no_colour() {
        let shape = glyph_shape(1, 1, vec![0]);
        assert_eq!(shape.colour_at(0, 0), None);
    }

    #[test]
    fn an_out_of_bounds_pixel_has_no_colour() {
        let shape = glyph_shape(1, 1, vec![255]);
        assert_eq!(shape.colour_at(-1, 0), None);
        assert_eq!(shape.colour_at(0, -1), None);
        assert_eq!(shape.colour_at(1, 0), None);
        assert_eq!(shape.colour_at(0, 1), None);
    }

    #[test]
    fn a_partially_covered_pixel_has_scaled_alpha() {
        let shape = glyph_shape(1, 1, vec![128]);
        let mut expected = INK;
        expected[3] = (OPAQUE as u16 * 128 / 255) as u8;
        assert_eq!(shape.colour_at(0, 0), Some(expected));
    }

    #[test]
    fn the_same_character_rasterized_twice_is_pixel_identical() {
        let mut cache = GlyphCache::new(CELL_WIDTH, CELL_HEIGHT);
        let first = cache.glyph('B', false).pixels.clone();
        let second = cache.glyph('B', false).pixels.clone();
        assert_eq!(first, second);
    }

    #[test]
    fn a_hinted_glyph_is_faded_compared_to_a_plain_glyph() {
        let mut cache = GlyphCache::new(CELL_WIDTH, CELL_HEIGHT);
        let plain = cache.glyph('B', false).pixels.clone();
        let hinted = cache.glyph('B', true).pixels.clone();
        assert_ne!(plain, hinted);

        let plain_alpha: u32 = plain.chunks(4).map(|pixel| pixel[3] as u32).sum();
        let hinted_alpha: u32 = hinted.chunks(4).map(|pixel| pixel[3] as u32).sum();
        assert!(hinted_alpha < plain_alpha);
    }

    #[test]
    fn a_descender_is_not_clipped_at_the_bottom_of_the_cell() {
        let cache = GlyphCache::new(CELL_WIDTH, CELL_HEIGHT);
        let (_metrics, bitmap) = cache.font.rasterize('g', cache.px_size);
        let raw_coverage: u32 = bitmap.iter().map(|&byte| byte as u32).sum();
        assert!(
            raw_coverage > 0,
            "test font must actually rasterize 'g' with some ink"
        );

        let canvas = cache.rasterize('g', false);
        let placed_coverage: u32 = canvas.pixels.chunks(4).map(|pixel| pixel[3] as u32).sum();
        assert_eq!(placed_coverage, raw_coverage);
    }

    #[test]
    fn every_glyph_canvas_is_exactly_one_cell() {
        let mut cache = GlyphCache::new(CELL_WIDTH, CELL_HEIGHT);
        for ch in ['M', 'i'] {
            let canvas = cache.glyph(ch, false);
            assert_eq!(canvas.width, CELL_WIDTH);
            assert_eq!(canvas.height, CELL_HEIGHT);
        }
    }
}
