use crate::canvas::{Rgba, Shape};

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
    use crate::render::OPAQUE;

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
}
