use super::terminal::{box_shape, brackets_shape, BoxStyle, BracketStyle};
use crate::canvas::{Canvas, Rgba, Shape};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct CellSize {
    pub(super) width: i64,
    pub(super) height: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum TileStyle {
    Box(BoxStyle),
    #[cfg_attr(not(test), allow(dead_code))]
    Brackets(BracketStyle),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct TileShape {
    pub(super) style: TileStyle,
    pub(super) cell: CellSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Band {
    Start(i64),
    Middle,
    End(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct TileKey {
    pub(super) shape: TileShape,
    pub(super) column: Band,
    pub(super) row: Band,
}

fn band(extent: i64, cell: i64) -> i64 {
    let covered = extent + 1;
    (covered + cell - 1) / cell
}

pub(super) fn cells_with_middle(band: i64) -> i64 {
    2 * band + 1
}

impl Band {
    fn of(index: i64, cells: i64, band: i64) -> Band {
        if index < band {
            Band::Start(index)
        } else if index >= cells - band {
            Band::End(cells - 1 - index)
        } else {
            Band::Middle
        }
    }

    fn reference_index(self, band: i64) -> i64 {
        match self {
            Band::Start(offset) => offset,
            Band::Middle => band,
            Band::End(offset) => cells_with_middle(band) - 1 - offset,
        }
    }
}

struct Offset<'a, S> {
    shape: &'a S,
    x: i64,
    y: i64,
}

impl<S: Shape> Shape for Offset<'_, S> {
    fn colour_at(&self, x: i64, y: i64) -> Option<Rgba> {
        self.shape.colour_at(x + self.x, y + self.y)
    }
}

impl TileShape {
    fn edge_extents(&self) -> (i64, i64) {
        match self.style {
            TileStyle::Box(style) => box_shape(0, 0, style).edge_extents(),
            TileStyle::Brackets(style) => brackets_shape(0, 0, self.cell, style).edge_extents(),
        }
    }

    pub(super) fn bands(&self) -> (i64, i64) {
        let (extent_x, extent_y) = self.edge_extents();
        (
            band(extent_x, self.cell.width),
            band(extent_y, self.cell.height),
        )
    }

    pub(super) fn tiles(&self, cols: i64, rows: i64) -> Option<Vec<(i64, i64, TileKey)>> {
        let (column_band, row_band) = self.bands();
        if cols < cells_with_middle(column_band) || rows < cells_with_middle(row_band) {
            return None;
        }
        let shape = *self;
        Some(
            (0..rows)
                .flat_map(|row| {
                    (0..cols).map(move |col| {
                        let key = TileKey {
                            shape,
                            column: Band::of(col, cols, column_band),
                            row: Band::of(row, rows, row_band),
                        };
                        (col, row, key)
                    })
                })
                .collect(),
        )
    }

    fn cell_canvas(&self, cols: i64, rows: i64, col: i64, row: i64) -> Canvas {
        let (width, height) = (cols * self.cell.width, rows * self.cell.height);
        match self.style {
            TileStyle::Box(style) => self.cell_of(&box_shape(width, height, style), col, row),
            TileStyle::Brackets(style) => {
                self.cell_of(&brackets_shape(width, height, self.cell, style), col, row)
            }
        }
    }

    fn cell_of(&self, shape: &impl Shape, col: i64, row: i64) -> Canvas {
        let offset = Offset {
            shape,
            x: col * self.cell.width,
            y: row * self.cell.height,
        };
        Canvas::fill(self.cell.width, self.cell.height, &offset)
    }
}

impl TileKey {
    pub(super) fn canvas(&self) -> Canvas {
        let (column_band, row_band) = self.shape.bands();
        self.shape.cell_canvas(
            cells_with_middle(column_band),
            cells_with_middle(row_band),
            self.column.reference_index(column_band),
            self.row.reference_index(row_band),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::terminal::quantized_alpha;
    use crate::style::{palette, BOX_FILL_OPACITY, CELL_HEIGHT, CELL_WIDTH, FOOTER_FILL_OPACITY};
    use crate::view::{Sides, ALL_SIDES, BORDER, NO_SIDES};
    use std::collections::{HashMap, HashSet};

    const CELL: CellSize = CellSize {
        width: CELL_WIDTH,
        height: CELL_HEIGHT,
    };
    const ODD_CELL: CellSize = CellSize {
        width: 11,
        height: 23,
    };
    const CELLS: [CellSize; 2] = [CELL, ODD_CELL];
    const TOP_AND_LEFT: Sides = (true, false, false, true);
    const RIGHT_AND_BOTTOM: Sides = (false, true, true, false);
    const EXTRA_CELLS: i64 = 3;

    fn colours() -> Vec<Option<u8>> {
        std::iter::once(None)
            .chain(
                (0..=u8::MAX)
                    .take_while(|&i| palette(i).is_some())
                    .map(Some),
            )
            .collect()
    }

    fn outlined(colour: Option<u8>, rounded: bool, sides: Sides) -> BoxStyle {
        BoxStyle {
            colour: crate::style::rgb(colour),
            fill: colour,
            fill_alpha: quantized_alpha(colour.map(|_| BOX_FILL_OPACITY)),
            rounded,
            sides,
            border: BORDER,
        }
    }

    fn borderless(colour: Option<u8>) -> BoxStyle {
        BoxStyle {
            colour: crate::style::rgb(None),
            fill: colour,
            fill_alpha: quantized_alpha(Some(FOOTER_FILL_OPACITY)),
            rounded: false,
            sides: NO_SIDES,
            border: 1,
        }
    }

    fn styles() -> Vec<TileStyle> {
        colours()
            .into_iter()
            .flat_map(|colour| {
                [
                    TileStyle::Box(outlined(colour, true, ALL_SIDES)),
                    TileStyle::Box(outlined(colour, false, ALL_SIDES)),
                    TileStyle::Box(outlined(colour, true, TOP_AND_LEFT)),
                    TileStyle::Box(outlined(colour, false, RIGHT_AND_BOTTOM)),
                    TileStyle::Box(borderless(colour)),
                    TileStyle::Brackets(BracketStyle { border: BORDER }),
                ]
            })
            .collect()
    }

    fn whole_sprite(shape: TileShape, cols: i64, rows: i64) -> Canvas {
        let (width, height) = (cols * shape.cell.width, rows * shape.cell.height);
        match shape.style {
            TileStyle::Box(style) => Canvas::fill(width, height, &box_shape(width, height, style)),
            TileStyle::Brackets(style) => Canvas::fill(
                width,
                height,
                &brackets_shape(width, height, shape.cell, style),
            ),
        }
    }

    const CHANNELS: usize = 4;

    fn composed(
        shape: TileShape,
        cols: i64,
        rows: i64,
        rasterized: &mut HashMap<TileKey, Canvas>,
    ) -> Canvas {
        let (width, height) = (cols * shape.cell.width, rows * shape.cell.height);
        let mut pixels = vec![0u8; (width * height) as usize * CHANNELS];
        for (col, row, key) in shape.tiles(cols, rows).expect("a tileable sprite") {
            let tile = rasterized.entry(key).or_insert_with(|| key.canvas());
            for y in 0..tile.height {
                let from = (y * tile.width) as usize * CHANNELS;
                let to = ((row * tile.height + y) * width + col * tile.width) as usize * CHANNELS;
                let span = tile.width as usize * CHANNELS;
                pixels[to..to + span].copy_from_slice(&tile.pixels[from..from + span]);
            }
        }
        Canvas {
            pixels,
            width,
            height,
        }
    }

    #[test]
    fn composing_the_tiles_gives_exactly_the_whole_sprite() {
        let shape = TileShape {
            style: TileStyle::Box(outlined(Some(0), true, ALL_SIDES)),
            cell: ODD_CELL,
        };
        let (column_band, row_band) = shape.bands();
        let (cols, rows) = (
            cells_with_middle(column_band) + 1,
            cells_with_middle(row_band) + 1,
        );
        assert!(
            composed(shape, cols, rows, &mut HashMap::new()).pixels
                == whole_sprite(shape, cols, rows).pixels
        );
    }

    fn rounded_box() -> TileShape {
        TileShape {
            style: TileStyle::Box(outlined(Some(0), true, ALL_SIDES)),
            cell: CELL,
        }
    }

    #[test]
    fn a_sprite_with_room_for_a_middle_band_in_both_axes_is_tiled() {
        let shape = rounded_box();
        let (column_band, row_band) = shape.bands();
        assert!(shape
            .tiles(cells_with_middle(column_band), cells_with_middle(row_band))
            .is_some());
    }

    #[test]
    fn a_sprite_one_column_short_of_a_middle_band_is_not_tiled() {
        let shape = rounded_box();
        let (column_band, row_band) = shape.bands();
        assert!(shape
            .tiles(
                cells_with_middle(column_band) - 1,
                cells_with_middle(row_band)
            )
            .is_none());
    }

    #[test]
    fn a_sprite_one_row_short_of_a_middle_band_is_not_tiled() {
        let shape = rounded_box();
        let (column_band, row_band) = shape.bands();
        assert!(shape
            .tiles(
                cells_with_middle(column_band),
                cells_with_middle(row_band) - 1
            )
            .is_none());
    }

    #[test]
    fn a_band_covers_the_edge_and_one_antialiasing_pixel() {
        for style in styles() {
            for cell in CELLS {
                let shape = TileShape { style, cell };
                let (column_band, row_band) = shape.bands();
                let (extent_x, extent_y) = match style {
                    TileStyle::Box(style) => box_shape(0, 0, style).edge_extents(),
                    TileStyle::Brackets(style) => brackets_shape(0, 0, cell, style).edge_extents(),
                };
                assert!(column_band * cell.width > extent_x);
                assert!((column_band - 1) * cell.width <= extent_x);
                assert!(row_band * cell.height > extent_y);
                assert!((row_band - 1) * cell.height <= extent_y);
            }
        }
    }

    fn tiles_of(shape: TileShape, cols: i64, rows: i64) -> Vec<(i64, i64, TileKey)> {
        shape.tiles(cols, rows).expect("a tileable sprite")
    }

    fn key_set(shape: TileShape, cols: i64, rows: i64) -> HashSet<TileKey> {
        tiles_of(shape, cols, rows)
            .into_iter()
            .map(|(_, _, key)| key)
            .collect()
    }

    fn smallest_tileable(shape: TileShape) -> (i64, i64) {
        let (column_band, row_band) = shape.bands();
        (cells_with_middle(column_band), cells_with_middle(row_band))
    }

    #[test]
    fn tiles_are_in_row_major_order_one_per_cell() {
        let shape = rounded_box();
        let (cols, rows) = smallest_tileable(shape);
        let (cols, rows) = (cols + EXTRA_CELLS, rows + 1);
        let positions: Vec<(i64, i64)> = tiles_of(shape, cols, rows)
            .into_iter()
            .map(|(col, row, _)| (col, row))
            .collect();
        let expected: Vec<(i64, i64)> = (0..rows)
            .flat_map(|row| (0..cols).map(move |col| (col, row)))
            .collect();
        assert_eq!(positions, expected);
    }

    #[test]
    fn edge_tiles_hold_their_offset_from_that_edge_and_the_rest_are_middle() {
        let shape = rounded_box();
        let (column_band, _) = shape.bands();
        let (cols, rows) = smallest_tileable(shape);
        let cols = cols + EXTRA_CELLS;
        let first_row: Vec<Band> = tiles_of(shape, cols, rows)
            .into_iter()
            .filter(|&(_, row, _)| row == 0)
            .map(|(_, _, key)| key.column)
            .collect();
        let expected: Vec<Band> = (0..column_band)
            .map(Band::Start)
            .chain((0..cols - 2 * column_band).map(|_| Band::Middle))
            .chain((0..column_band).rev().map(Band::End))
            .collect();
        assert_eq!(first_row, expected);
    }

    #[test]
    fn sprites_of_different_sizes_in_the_same_style_share_the_same_keys() {
        for style in styles() {
            let shape = TileShape { style, cell: CELL };
            let (cols, rows) = smallest_tileable(shape);
            assert_eq!(
                key_set(shape, cols, rows),
                key_set(shape, cols + EXTRA_CELLS, rows + 1)
            );
        }
    }

    #[test]
    fn tiles_of_different_bands_or_offsets_have_different_keys() {
        let shape = rounded_box();
        let (cols, rows) = smallest_tileable(shape);
        let (column_band, row_band) = shape.bands();
        let distinct = (2 * column_band + 1) * (2 * row_band + 1);
        assert_eq!(key_set(shape, cols, rows).len() as i64, distinct);
    }

    #[test]
    fn tiles_of_different_styles_or_cell_sizes_never_share_a_key() {
        let shapes: Vec<TileShape> = styles()
            .into_iter()
            .flat_map(|style| CELLS.map(|cell| TileShape { style, cell }))
            .collect();
        let mut owner: HashMap<TileKey, TileShape> = HashMap::new();
        for shape in shapes {
            let (cols, rows) = smallest_tileable(shape);
            for key in key_set(shape, cols, rows) {
                assert_eq!(*owner.entry(key).or_insert(shape), shape);
            }
        }
    }

    #[test]
    fn a_tile_is_one_cell_of_pixels() {
        let shape = TileShape {
            style: rounded_box().style,
            cell: ODD_CELL,
        };
        let (cols, rows) = smallest_tileable(shape);
        for (_, _, key) in tiles_of(shape, cols, rows) {
            let tile = key.canvas();
            assert_eq!((tile.width, tile.height), (ODD_CELL.width, ODD_CELL.height));
        }
    }
}
