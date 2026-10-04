use super::placements::{CellRect, Placement};
use crate::canvas::{Canvas, Rgba, Shape};
use crate::tty::Window;

const BORDER_COLOUR: Rgba = [0x2A, 0x2A, 0x2E, 0xFF];
const SELECTED_COLOUR: Rgba = [0x8A, 0xB4, 0xF8, 0xFF];
const TEXT_COLOUR: (u8, u8, u8) = (0xC9, 0xC9, 0xCF);
const FILL_COLOUR: Rgba = [0x14, 0x14, 0x16, 0xFF];
const HIGHLIGHT_COLOUR: Rgba = [0xE8, 0xEA, 0xED, HIGHLIGHT_ALPHA];
const HIGHLIGHT_ALPHA: u8 = 140;
const BORDER_PX: i64 = 1;
const OUTLINE_GAP_PX: i64 = 2;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextRun {
    pub(crate) col: i64,
    pub(crate) row: i64,
    pub(crate) text: String,
    pub(crate) colour: (u8, u8, u8),
}

pub(crate) struct Frame {
    pub(crate) pixels: Canvas,
    pub(crate) texts: Vec<TextRun>,
    pub(crate) caret: Option<(i64, i64)>,
}

#[derive(Clone, Copy)]
struct PixelRect {
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
}

impl PixelRect {
    fn of(rect: &CellRect, window: Window) -> PixelRect {
        PixelRect {
            left: rect.x * window.cell_width,
            top: rect.y * window.cell_height,
            right: (rect.x + rect.width) * window.cell_width,
            bottom: (rect.y + rect.height) * window.cell_height,
        }
    }

    fn inset(self, dx: i64, dy: i64) -> PixelRect {
        PixelRect {
            left: self.left + dx,
            top: self.top + dy,
            right: self.right - dx,
            bottom: self.bottom - dy,
        }
    }

    fn contains(self, x: i64, y: i64) -> bool {
        (self.left..self.right).contains(&x) && (self.top..self.bottom).contains(&y)
    }
}

struct BoxShape {
    rect: PixelRect,
    filled: bool,
}

impl Shape for BoxShape {
    fn colour_at(&self, x: i64, y: i64) -> Option<Rgba> {
        if !self.rect.contains(x, y) {
            None
        } else if !self.rect.inset(BORDER_PX, BORDER_PX).contains(x, y) {
            Some(BORDER_COLOUR)
        } else {
            self.filled.then_some(FILL_COLOUR)
        }
    }
}

struct HighlightShape {
    rect: PixelRect,
}

impl Shape for HighlightShape {
    fn colour_at(&self, x: i64, y: i64) -> Option<Rgba> {
        self.rect.contains(x, y).then_some(HIGHLIGHT_COLOUR)
    }
}

struct OutlineShape {
    rect: PixelRect,
    inset: (i64, i64),
}

impl Shape for OutlineShape {
    fn colour_at(&self, x: i64, y: i64) -> Option<Rgba> {
        let (dx, dy) = self.inset;
        let stroke = self.rect.inset(dx, dy);
        let inner = stroke.inset(BORDER_PX, BORDER_PX);
        (stroke.contains(x, y) && !inner.contains(x, y)).then_some(SELECTED_COLOUR)
    }
}

struct Layers(Vec<Box<dyn Shape>>);

impl Shape for Layers {
    fn colour_at(&self, x: i64, y: i64) -> Option<Rgba> {
        self.0
            .iter()
            .fold(None, |below, shape| match shape.colour_at(x, y) {
                Some(colour) => Some(colour),
                None => below,
            })
    }
}

fn shape(placement: &Placement, window: Window) -> Option<Box<dyn Shape>> {
    match placement {
        Placement::Box { rect, filled } => Some(Box::new(BoxShape {
            rect: PixelRect::of(rect, window),
            filled: *filled,
        })),
        Placement::Highlight { rect } => Some(Box::new(HighlightShape {
            rect: PixelRect::of(rect, window),
        })),
        Placement::Outline { rect } => Some(Box::new(OutlineShape {
            rect: PixelRect::of(rect, window),
            inset: (
                window.cell_width - BORDER_PX - OUTLINE_GAP_PX,
                window.cell_height - BORDER_PX - OUTLINE_GAP_PX,
            ),
        })),
        Placement::Text { .. } | Placement::Caret { .. } => None,
    }
}

pub(crate) fn draw(placements: &[Placement], window: Window) -> Frame {
    let layers = Layers(
        placements
            .iter()
            .filter_map(|placement| shape(placement, window))
            .collect(),
    );
    Frame {
        pixels: Canvas::fill(
            window.cols * window.cell_width,
            window.rows * window.cell_height,
            &layers,
        ),
        texts: placements
            .iter()
            .filter_map(|placement| match placement {
                Placement::Text { at, text } => Some(TextRun {
                    col: at.0,
                    row: at.1,
                    text: text.clone(),
                    colour: TEXT_COLOUR,
                }),
                _ => None,
            })
            .collect(),
        caret: placements.iter().find_map(|placement| match placement {
            Placement::Caret { at } => Some(*at),
            _ => None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Window = Window {
        cols: 12,
        rows: 8,
        cell_width: 10,
        cell_height: 20,
    };

    fn rect(x: i64, y: i64, width: i64, height: i64) -> CellRect {
        CellRect {
            x,
            y,
            width,
            height,
        }
    }

    fn pixel(frame: &Frame, x: i64, y: i64) -> Rgba {
        let start = ((y * frame.pixels.width + x) * 4) as usize;
        frame.pixels.pixels[start..start + 4].try_into().unwrap()
    }

    const CLEAR: Rgba = [0; 4];

    #[test]
    fn the_pixels_cover_the_window() {
        let frame = draw(&[], WINDOW);

        assert_eq!((frame.pixels.width, frame.pixels.height), (120, 160));
        assert_eq!(pixel(&frame, 5, 5), CLEAR);
    }

    #[test]
    fn a_box_has_a_border_on_its_edge_pixels() {
        let frame = draw(
            &[Placement::Box {
                rect: rect(2, 1, 4, 3),
                filled: false,
            }],
            WINDOW,
        );

        for (x, y) in [(20, 20), (59, 20), (20, 79), (59, 79), (40, 20), (20, 50)] {
            assert_eq!(pixel(&frame, x, y), BORDER_COLOUR, "{x},{y}");
        }
        assert_eq!(pixel(&frame, 19, 20), CLEAR);
        assert_eq!(pixel(&frame, 60, 50), CLEAR);
        assert_eq!(pixel(&frame, 40, 19), CLEAR);
        assert_eq!(pixel(&frame, 40, 80), CLEAR);
    }

    #[test]
    fn an_unfilled_box_is_transparent_inside() {
        let frame = draw(
            &[Placement::Box {
                rect: rect(2, 1, 4, 3),
                filled: false,
            }],
            WINDOW,
        );

        assert_eq!(pixel(&frame, 21, 21), CLEAR);
        assert_eq!(pixel(&frame, 40, 50), CLEAR);
    }

    #[test]
    fn a_filled_box_is_filled_inside_its_border() {
        let frame = draw(
            &[Placement::Box {
                rect: rect(2, 1, 4, 3),
                filled: true,
            }],
            WINDOW,
        );

        assert_eq!(pixel(&frame, 21, 21), FILL_COLOUR);
        assert_eq!(pixel(&frame, 40, 50), FILL_COLOUR);
        assert_eq!(pixel(&frame, 20, 20), BORDER_COLOUR);
    }

    #[test]
    fn a_highlight_tints_exactly_its_cells() {
        let frame = draw(
            &[Placement::Highlight {
                rect: rect(3, 2, 2, 1),
            }],
            WINDOW,
        );

        assert_eq!(pixel(&frame, 30, 40), HIGHLIGHT_COLOUR);
        assert_eq!(pixel(&frame, 49, 59), HIGHLIGHT_COLOUR);
        assert_eq!(pixel(&frame, 29, 40), CLEAR);
        assert_eq!(pixel(&frame, 50, 40), CLEAR);
        assert_eq!(pixel(&frame, 30, 60), CLEAR);
        assert_eq!(pixel(&frame, 30, 39), CLEAR);
    }

    #[test]
    fn an_outline_keeps_a_gap_around_the_box_it_surrounds() {
        let frame = draw(
            &[Placement::Outline {
                rect: rect(1, 1, 6, 4),
            }],
            WINDOW,
        );
        let stroke_x = WINDOW.cell_width - BORDER_PX - OUTLINE_GAP_PX;
        let box_left = 2 * WINDOW.cell_width;

        assert_eq!(pixel(&frame, 10 + stroke_x, 60), SELECTED_COLOUR);
        for x in 10 + stroke_x + BORDER_PX..box_left {
            assert_eq!(pixel(&frame, x, 60), CLEAR, "x={x}");
        }
        assert_eq!(box_left - (10 + stroke_x + BORDER_PX), OUTLINE_GAP_PX);
        assert_eq!(pixel(&frame, 10, 60), CLEAR);
    }

    #[test]
    fn an_outline_surrounds_the_box_at_the_gap_on_every_side() {
        let boxed = rect(2, 2, 4, 3);
        let frame = draw(
            &[Placement::Outline {
                rect: rect(1, 1, 6, 5),
            }],
            WINDOW,
        );
        let gap = OUTLINE_GAP_PX;
        let (left, top) = (2 * WINDOW.cell_width, 2 * WINDOW.cell_height);
        let right = (boxed.x + boxed.width) * WINDOW.cell_width;
        let bottom = (boxed.y + boxed.height) * WINDOW.cell_height;

        assert_eq!(pixel(&frame, left - gap - 1, 60), SELECTED_COLOUR);
        assert_eq!(pixel(&frame, right + gap, 60), SELECTED_COLOUR);
        assert_eq!(pixel(&frame, 40, top - gap - 1), SELECTED_COLOUR);
        assert_eq!(pixel(&frame, 40, bottom + gap), SELECTED_COLOUR);
    }

    #[test]
    fn an_outline_is_drawn_over_a_box_that_it_overlaps() {
        let frame = draw(
            &[
                Placement::Box {
                    rect: rect(1, 1, 6, 4),
                    filled: true,
                },
                Placement::Outline {
                    rect: rect(1, 1, 6, 4),
                },
            ],
            WINDOW,
        );
        let stroke_x = 10 + WINDOW.cell_width - BORDER_PX - OUTLINE_GAP_PX;

        assert_eq!(pixel(&frame, stroke_x, 50), SELECTED_COLOUR);
        assert_eq!(pixel(&frame, 10, 50), BORDER_COLOUR);
    }

    #[test]
    fn later_placements_paint_over_earlier_ones() {
        let frame = draw(
            &[
                Placement::Highlight {
                    rect: rect(0, 0, 2, 2),
                },
                Placement::Box {
                    rect: rect(0, 0, 2, 2),
                    filled: true,
                },
            ],
            WINDOW,
        );

        assert_eq!(pixel(&frame, 5, 5), FILL_COLOUR);
    }

    #[test]
    fn a_text_becomes_a_run_with_the_theme_colour_at_its_cell() {
        let frame = draw(
            &[Placement::Text {
                at: (4, 3),
                text: "hi".to_string(),
            }],
            WINDOW,
        );

        assert_eq!(
            frame.texts,
            vec![TextRun {
                col: 4,
                row: 3,
                text: "hi".to_string(),
                colour: TEXT_COLOUR,
            }]
        );
        assert!(frame.pixels.pixels.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn the_caret_is_passed_through_and_absent_otherwise() {
        assert_eq!(
            draw(&[Placement::Caret { at: (5, 2) }], WINDOW).caret,
            Some((5, 2))
        );
        assert_eq!(draw(&[], WINDOW).caret, None);
    }
}
