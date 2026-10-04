pub(crate) type Rgba = [u8; 4];

const CHANNELS: usize = 4;

pub(crate) trait Shape {
    fn colour_at(&self, x: i64, y: i64) -> Option<Rgba>;
}

#[derive(Clone)]
pub struct Canvas {
    pub(crate) pixels: Vec<u8>,
    pub(crate) width: i64,
    pub(crate) height: i64,
}

impl Canvas {
    pub(crate) fn fill(width: i64, height: i64, shape: &impl Shape) -> Canvas {
        let mut pixels = vec![0u8; (width * height) as usize * CHANNELS];
        for y in 0..height {
            for x in 0..width {
                if let Some(colour) = shape.colour_at(x, y) {
                    let start = ((y * width + x) as usize) * CHANNELS;
                    pixels[start..start + CHANNELS].copy_from_slice(&colour);
                }
            }
        }
        Canvas {
            pixels,
            width,
            height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Gradient;

    impl Shape for Gradient {
        fn colour_at(&self, x: i64, y: i64) -> Option<Rgba> {
            Some([x as u8, y as u8, 7, 255])
        }
    }

    struct OnlyAt(i64, i64);

    impl Shape for OnlyAt {
        fn colour_at(&self, x: i64, y: i64) -> Option<Rgba> {
            (x == self.0 && y == self.1).then_some([9, 8, 7, 6])
        }
    }

    fn pixel(canvas: &Canvas, x: i64, y: i64) -> Rgba {
        let start = ((y * canvas.width + x) as usize) * CHANNELS;
        canvas.pixels[start..start + CHANNELS].try_into().unwrap()
    }

    #[test]
    fn fill_has_one_pixel_per_cell() {
        let canvas = Canvas::fill(5, 3, &Gradient);
        assert_eq!((canvas.width, canvas.height), (5, 3));
        assert_eq!(canvas.pixels.len(), 5 * 3 * CHANNELS);
    }

    #[test]
    fn fill_stores_what_the_shape_returns_at_every_pixel() {
        let canvas = Canvas::fill(5, 3, &Gradient);
        for y in 0..3 {
            for x in 0..5 {
                assert_eq!(Some(pixel(&canvas, x, y)), Gradient.colour_at(x, y));
            }
        }
    }

    #[test]
    fn a_pixel_the_shape_declines_stays_transparent() {
        let canvas = Canvas::fill(4, 4, &OnlyAt(2, 1));
        for y in 0..4 {
            for x in 0..4 {
                let expected = if (x, y) == (2, 1) {
                    [9, 8, 7, 6]
                } else {
                    [0; 4]
                };
                assert_eq!(pixel(&canvas, x, y), expected);
            }
        }
    }
}
