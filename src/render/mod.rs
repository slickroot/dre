use std::io::{self, Write};

use crate::diagram::{palette, Document};

#[cfg(not(target_arch = "wasm32"))]
mod shapes;
mod svg;
#[cfg(not(target_arch = "wasm32"))]
mod terminal;
pub use svg::SvgRenderer;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use terminal::TerminalRenderer;

pub trait Renderer {
    fn render(&mut self, doc: &Document, out: &mut impl Write) -> io::Result<()>;
}

const ARROWHEAD_ANGLE_DEG: f64 = 30.0;
const ARROWHEAD_EDGE_LENGTH: f64 = 15.0;
fn arrowhead_depth() -> f64 {
    ARROWHEAD_EDGE_LENGTH * ARROWHEAD_ANGLE_DEG.to_radians().cos()
}
fn arrowhead_slope() -> f64 {
    ARROWHEAD_ANGLE_DEG.to_radians().tan()
}

const CELL_WIDTH: i64 = 8;
const CELL_HEIGHT: i64 = 16;

const ROUNDED_RADIUS: i64 = 20;
const BORDER: i64 = 4;

const OPAQUE: u8 = 255;
const FILL_ALPHA: u16 = 77;
const PLAIN_COLOUR: (u8, u8, u8) = (128, 128, 128);

fn colour(colour: Option<u8>) -> (u8, u8, u8) {
    match colour {
        None => PLAIN_COLOUR,
        Some(i) => palette(i).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_of_plain_is_the_plain_grey() {
        assert_eq!(colour(None), PLAIN_COLOUR);
    }

    #[test]
    fn colour_of_a_palette_index_is_the_palette_entry() {
        assert_eq!(colour(Some(2)), palette(2).unwrap());
    }
}
