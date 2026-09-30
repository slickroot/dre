#![allow(dead_code)]

use super::tiles::CellSize;
use crate::canvas::{Canvas, Rgba, Shape};
use crate::view::BRACKET_MARGIN;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    fn is_right(self) -> bool {
        matches!(self, Corner::TopRight | Corner::BottomRight)
    }

    fn is_bottom(self) -> bool {
        matches!(self, Corner::BottomLeft | Corner::BottomRight)
    }
}

fn cells_to_hold(pixels: i64, cell_size: i64) -> i64 {
    (pixels + cell_size - 1) / cell_size
}

pub(super) fn corner_cells(cell: CellSize) -> i64 {
    let reach_x = BRACKET_MARGIN * cell.width + super::BRACKET_ARM - super::BRACKET_OFFSET;
    let reach_y = BRACKET_MARGIN * cell.height + super::BRACKET_ARM - super::BRACKET_OFFSET;
    cells_to_hold(reach_x, cell.width).max(cells_to_hold(reach_y, cell.height))
}

pub(super) fn corner_offset(corner: Corner, cols: i64, rows: i64, cell: CellSize) -> (i64, i64) {
    let blocks = corner_cells(cell);
    (
        if corner.is_right() { cols - blocks } else { 0 },
        if corner.is_bottom() { rows - blocks } else { 0 },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct BracketKey {
    pub(super) corner: Corner,
    pub(super) border: i64,
    pub(super) cell: CellSize,
}

impl BracketKey {
    pub(super) fn canvas(&self) -> Canvas {
        let (r, g, b) = super::colour(None);
        let width = corner_cells(self.cell) * self.cell.width;
        let height = corner_cells(self.cell) * self.cell.height;
        let shape = BracketCornerShape {
            corner: self.corner,
            block_width: width,
            block_height: height,
            margin_x: BRACKET_MARGIN * self.cell.width,
            margin_y: BRACKET_MARGIN * self.cell.height,
            offset: super::BRACKET_OFFSET,
            arm: super::BRACKET_ARM,
            thickness: self.border,
            colour: [r, g, b, super::OPAQUE],
        };
        Canvas::fill(width, height, &shape)
    }
}

pub(super) struct BracketCornerShape {
    pub(super) corner: Corner,
    pub(super) block_width: i64,
    pub(super) block_height: i64,
    pub(super) margin_x: i64,
    pub(super) margin_y: i64,
    pub(super) offset: i64,
    pub(super) arm: i64,
    pub(super) thickness: i64,
    pub(super) colour: Rgba,
}

impl Shape for BracketCornerShape {
    fn colour_at(&self, x: i64, y: i64) -> Option<Rgba> {
        let from_corner_x = if self.corner.is_right() {
            self.block_width - 1 - x
        } else {
            x
        } - (self.margin_x - self.offset);
        let from_corner_y = if self.corner.is_bottom() {
            self.block_height - 1 - y
        } else {
            y
        } - (self.margin_y - self.offset);
        if from_corner_x < 0 || from_corner_y < 0 {
            return None;
        }
        let in_horizontal_arm = from_corner_x < self.arm && from_corner_y < self.thickness;
        let in_vertical_arm = from_corner_y < self.arm && from_corner_x < self.thickness;
        (in_horizontal_arm || in_vertical_arm).then_some(self.colour)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Canvas;
    use crate::render::shapes::BracketsShape;
    use crate::render::{BRACKET_ARM, BRACKET_OFFSET, OPAQUE};
    use crate::style::{CELL_HEIGHT, CELL_WIDTH};
    use crate::view::BRACKET_MARGIN;

    const CELL: CellSize = CellSize {
        width: CELL_WIDTH,
        height: CELL_HEIGHT,
    };
    const ODD_CELL: CellSize = CellSize {
        width: 11,
        height: 23,
    };
    const CELLS: [CellSize; 2] = [CELL, ODD_CELL];
    const CORNERS: [Corner; 4] = [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ];
    const COLOUR: Rgba = [40, 50, 60, OPAQUE];
    const THICKNESS: i64 = 2;
    const SPARE_CELLS: i64 = 4;

    fn corner_shape(corner: Corner, cell: CellSize) -> BracketCornerShape {
        BracketCornerShape {
            corner,
            block_width: corner_cells(cell) * cell.width,
            block_height: corner_cells(cell) * cell.height,
            margin_x: BRACKET_MARGIN * cell.width,
            margin_y: BRACKET_MARGIN * cell.height,
            offset: BRACKET_OFFSET,
            arm: BRACKET_ARM,
            thickness: THICKNESS,
            colour: COLOUR,
        }
    }

    fn corner_canvas(corner: Corner, cell: CellSize) -> Canvas {
        Canvas::fill(
            corner_cells(cell) * cell.width,
            corner_cells(cell) * cell.height,
            &corner_shape(corner, cell),
        )
    }

    fn whole_canvas(cols: i64, rows: i64, cell: CellSize) -> Canvas {
        let shape = BracketsShape {
            width: cols * cell.width,
            height: rows * cell.height,
            margin_x: BRACKET_MARGIN * cell.width,
            margin_y: BRACKET_MARGIN * cell.height,
            offset: BRACKET_OFFSET,
            arm: BRACKET_ARM,
            thickness: THICKNESS,
            colour: COLOUR,
        };
        Canvas::fill(shape.width, shape.height, &shape)
    }

    fn block_origin(corner: Corner, cols: i64, rows: i64, cell: CellSize) -> (i64, i64) {
        let blocks = corner_cells(cell);
        let col = if corner.is_right() { cols - blocks } else { 0 };
        let row = if corner.is_bottom() { rows - blocks } else { 0 };
        (col * cell.width, row * cell.height)
    }

    fn crop_block(whole: &Canvas, corner: Corner, cols: i64, rows: i64, cell: CellSize) -> Canvas {
        let (x, y) = block_origin(corner, cols, rows, cell);
        whole.crop(
            x,
            x + corner_cells(cell) * cell.width,
            y,
            y + corner_cells(cell) * cell.height,
        )
    }

    #[test]
    fn a_corner_block_holds_the_margin_and_the_whole_arm() {
        for cell in CELLS {
            let reach_x = BRACKET_MARGIN * cell.width + BRACKET_ARM - BRACKET_OFFSET;
            let reach_y = BRACKET_MARGIN * cell.height + BRACKET_ARM - BRACKET_OFFSET;
            assert!(corner_cells(cell) * cell.width >= reach_x);
            assert!(corner_cells(cell) * cell.height >= reach_y);
            assert!(
                (corner_cells(cell) - 1) * cell.width < reach_x
                    || (corner_cells(cell) - 1) * cell.height < reach_y
            );
        }
    }

    #[test]
    fn each_corner_matches_the_whole_placement_pixels_in_its_block() {
        for cell in CELLS {
            let blocks = corner_cells(cell);
            for (cols, rows) in [
                (blocks + SPARE_CELLS, blocks + SPARE_CELLS),
                (2 * blocks, 2 * blocks),
                (3 * blocks + SPARE_CELLS, 2 * blocks + 1),
            ] {
                let whole = whole_canvas(cols, rows, cell);
                for corner in CORNERS {
                    assert_eq!(
                        corner_canvas(corner, cell).pixels,
                        crop_block(&whole, corner, cols, rows, cell).pixels,
                        "{corner:?} {cell:?} {cols}x{rows}"
                    );
                }
            }
        }
    }

    #[test]
    fn overlapping_corners_add_up_to_the_whole_placement_pixels() {
        for cell in CELLS {
            let blocks = corner_cells(cell);
            for (cols, rows) in [(blocks, blocks), (blocks + 1, blocks), (blocks, blocks + 1)] {
                let whole = whole_canvas(cols, rows, cell);
                let mut union = vec![0u8; whole.pixels.len()];
                for corner in CORNERS {
                    let (origin_x, origin_y) = block_origin(corner, cols, rows, cell);
                    let block = corner_canvas(corner, cell);
                    for y in 0..block.height {
                        for x in 0..block.width {
                            let from = ((y * block.width + x) * 4) as usize;
                            let to = (((origin_y + y) * whole.width + origin_x + x) * 4) as usize;
                            if block.pixels[from + 3] != 0 {
                                union[to..to + 4].copy_from_slice(&block.pixels[from..from + 4]);
                            }
                        }
                    }
                }
                assert_eq!(union, whole.pixels, "{cell:?} {cols}x{rows}");
            }
        }
    }

    fn top_left(cell: CellSize) -> BracketCornerShape {
        corner_shape(Corner::TopLeft, cell)
    }

    fn start(shape: &BracketCornerShape) -> (i64, i64) {
        (shape.margin_x - shape.offset, shape.margin_y - shape.offset)
    }

    #[test]
    fn a_bracket_is_an_l_of_two_arms_meeting_at_the_outer_corner() {
        for cell in CELLS {
            let shape = top_left(cell);
            let (start_x, start_y) = start(&shape);
            assert_eq!(shape.colour_at(start_x, start_y), Some(COLOUR));
            assert_eq!(
                shape.colour_at(start_x + shape.arm - 1, start_y),
                Some(COLOUR)
            );
            assert_eq!(
                shape.colour_at(start_x, start_y + shape.arm - 1),
                Some(COLOUR)
            );
            assert_eq!(shape.colour_at(start_x + shape.arm, start_y), None);
            assert_eq!(shape.colour_at(start_x, start_y + shape.arm), None);
            assert_eq!(shape.colour_at(start_x - 1, start_y), None);
            assert_eq!(shape.colour_at(start_x, start_y - 1), None);
        }
    }

    #[test]
    fn an_arm_is_as_thick_as_the_border() {
        let shape = top_left(CELL);
        let (start_x, start_y) = start(&shape);
        let arm_end = shape.arm - 1;
        assert_eq!(
            shape.colour_at(start_x + arm_end, start_y + shape.thickness - 1),
            Some(COLOUR)
        );
        assert_eq!(
            shape.colour_at(start_x + arm_end, start_y + shape.thickness),
            None
        );
        assert_eq!(
            shape.colour_at(start_x + shape.thickness - 1, start_y + arm_end),
            Some(COLOUR)
        );
        assert_eq!(
            shape.colour_at(start_x + shape.thickness, start_y + arm_end),
            None
        );
    }

    #[test]
    fn the_four_corners_mirror_each_other() {
        for cell in CELLS {
            let block = corner_canvas(Corner::TopLeft, cell);
            let pixel = |canvas: &Canvas, x: i64, y: i64| {
                let at = ((y * canvas.width + x) * 4) as usize;
                canvas.pixels[at..at + 4].to_vec()
            };
            let right = corner_canvas(Corner::TopRight, cell);
            let bottom = corner_canvas(Corner::BottomLeft, cell);
            let both = corner_canvas(Corner::BottomRight, cell);
            for y in 0..block.height {
                for x in 0..block.width {
                    let (mirror_x, mirror_y) = (block.width - 1 - x, block.height - 1 - y);
                    let original = pixel(&block, x, y);
                    assert_eq!(original, pixel(&right, mirror_x, y));
                    assert_eq!(original, pixel(&bottom, x, mirror_y));
                    assert_eq!(original, pixel(&both, mirror_x, mirror_y));
                }
            }
        }
    }

    #[test]
    fn the_inside_of_the_bracket_is_transparent() {
        let shape = top_left(CELL);
        let (start_x, start_y) = start(&shape);
        assert_eq!(
            shape.colour_at(start_x + shape.thickness, start_y + shape.thickness),
            None
        );
        assert_eq!(
            shape.colour_at(shape.block_width - 1, shape.block_height - 1),
            None
        );
    }

    #[test]
    fn brackets_are_solid_with_no_antialiasing() {
        for cell in CELLS {
            for corner in CORNERS {
                let block = corner_canvas(corner, cell);
                for pixel in block.pixels.chunks(4) {
                    assert!(pixel == [0; 4] || pixel == COLOUR);
                }
            }
        }
    }

    fn key(corner: Corner, border: i64, cell: CellSize) -> BracketKey {
        BracketKey {
            corner,
            border,
            cell,
        }
    }

    #[test]
    fn a_key_canvas_is_the_size_of_a_corner_block() {
        for cell in CELLS {
            let canvas = key(Corner::TopLeft, THICKNESS, cell).canvas();
            assert_eq!(canvas.width, corner_cells(cell) * cell.width);
            assert_eq!(canvas.height, corner_cells(cell) * cell.height);
        }
    }

    #[test]
    fn a_key_canvas_is_the_corner_shape_in_the_foreground_colour() {
        let (r, g, b) = crate::render::colour(None);
        let ink = [r, g, b, OPAQUE];
        for cell in CELLS {
            for corner in CORNERS {
                let canvas = key(corner, THICKNESS, cell).canvas();
                let expected = Canvas::fill(
                    canvas.width,
                    canvas.height,
                    &BracketCornerShape {
                        colour: ink,
                        ..corner_shape(corner, cell)
                    },
                );
                assert_eq!(canvas.pixels, expected.pixels, "{corner:?} {cell:?}");
            }
        }
    }

    #[test]
    fn keys_differ_by_corner_border_and_cell_only() {
        let base = key(Corner::TopLeft, THICKNESS, CELL);
        assert_eq!(base, key(Corner::TopLeft, THICKNESS, CELL));
        let mut seen = std::collections::HashSet::new();
        seen.insert(base);
        seen.insert(key(Corner::TopLeft, THICKNESS, CELL));
        assert_eq!(seen.len(), 1);
        for corner in CORNERS.into_iter().filter(|c| *c != Corner::TopLeft) {
            assert_ne!(base, key(corner, THICKNESS, CELL));
        }
        assert_ne!(base, key(Corner::TopLeft, THICKNESS + 1, CELL));
        assert_ne!(base, key(Corner::TopLeft, THICKNESS, ODD_CELL));
    }

    #[test]
    fn corner_offsets_sit_at_the_corners_of_a_large_placement() {
        let cell = CELL;
        let blocks = corner_cells(cell);
        let (cols, rows) = (blocks + SPARE_CELLS, blocks + SPARE_CELLS + 1);
        assert_eq!(corner_offset(Corner::TopLeft, cols, rows, cell), (0, 0));
        assert_eq!(
            corner_offset(Corner::TopRight, cols, rows, cell),
            (cols - blocks, 0)
        );
        assert_eq!(
            corner_offset(Corner::BottomLeft, cols, rows, cell),
            (0, rows - blocks)
        );
        assert_eq!(
            corner_offset(Corner::BottomRight, cols, rows, cell),
            (cols - blocks, rows - blocks)
        );
    }

    #[test]
    fn corner_offsets_go_negative_for_a_placement_smaller_than_a_block() {
        let cell = CELL;
        let blocks = corner_cells(cell);
        assert_eq!(
            corner_offset(Corner::BottomRight, 1, 1, cell),
            (1 - blocks, 1 - blocks)
        );
        assert_eq!(corner_offset(Corner::TopLeft, 1, 1, cell), (0, 0));
    }
}
