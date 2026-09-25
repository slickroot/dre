use std::io::{self, Write};

use crate::composer::{self, Area};
use crate::layout::{self, with_cursor, Placement, FOOTER_ROWS};
use crate::palette::{palette, FOREGROUND};
use crate::state::State;

#[cfg(not(target_arch = "wasm32"))]
mod font;
#[cfg(not(target_arch = "wasm32"))]
mod shapes;
mod svg;
#[cfg(not(target_arch = "wasm32"))]
mod terminal;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use font::GlyphCache;
pub use svg::SvgRenderer;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use terminal::{TerminalRenderer, CACHE_LIMIT};

pub trait Renderer {
    fn render(&mut self, state: &State, out: &mut impl Write) -> io::Result<()>;
}

#[allow(dead_code)]
pub(crate) fn editor(state: &State, window: Area) -> Vec<(Area, Vec<Placement<'_>>)> {
    let [body, foot] = composer::stack([None, Some(FOOTER_ROWS)], window);
    vec![
        (
            body,
            centre(
                with_cursor(layout::diagram(state.doc.tree()), state.selected.clone()),
                body,
            ),
        ),
        (foot, shift(layout::footer(foot.cols, foot.rows), foot)),
    ]
}

fn shift(placements: Vec<Placement<'_>>, area: Area) -> Vec<Placement<'_>> {
    offset(placements, area.col, area.row)
}

fn offset(placements: Vec<Placement<'_>>, dx: i64, dy: i64) -> Vec<Placement<'_>> {
    placements
        .into_iter()
        .map(|placement| Placement {
            x: placement.x + dx,
            y: placement.y + dy,
            ..placement
        })
        .collect()
}

fn centre(placements: Vec<Placement<'_>>, area: Area) -> Vec<Placement<'_>> {
    let Some(min_x) = placements.iter().map(|placement| placement.x).min() else {
        return placements;
    };
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
    let horizontal = (area.cols - span).div_euclid(2);
    let vertical = (area.rows - height).div_euclid(2);
    shift(offset(placements, horizontal, vertical), area)
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
const ARROW_OPACITY: f64 = 0.5;

fn colour(colour: Option<u8>) -> (u8, u8, u8) {
    palette(colour.unwrap_or(FOREGROUND)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{node, node_with_children};
    use crate::layout::{PlacementNode, FOOTER_COLOUR};
    use crate::state::{new_state, Mode};

    const AREA: Area = Area {
        col: 3,
        row: 2,
        cols: 20,
        rows: 10,
    };

    const WINDOW: Area = Area {
        col: 1,
        row: 4,
        cols: 80,
        rows: 24,
    };

    fn plain_box() -> PlacementNode<'static> {
        PlacementNode::Box {
            colour: None,
            fill: None,
            rounded: false,
        }
    }

    fn box_at(x: i64, y: i64, width: i64, height: i64) -> Placement<'static> {
        Placement {
            node: plain_box(),
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn shift_moves_every_placement_by_the_areas_corner() {
        let placements = shift(vec![box_at(0, 0, 3, 2), box_at(5, 4, 3, 2)], AREA);
        assert_eq!(
            placements,
            vec![
                box_at(AREA.col, AREA.row, 3, 2),
                box_at(AREA.col + 5, AREA.row + 4, 3, 2),
            ]
        );
    }

    #[test]
    fn centre_puts_the_diagram_in_the_middle_of_the_area() {
        let (width, height) = (6, 4);
        let placed = centre(vec![box_at(0, 0, width, height)], AREA);
        assert_eq!(
            (placed[0].x, placed[0].y),
            (
                AREA.col + (AREA.cols - width).div_euclid(2),
                AREA.row + (AREA.rows - height).div_euclid(2)
            )
        );
    }

    #[test]
    fn centre_measures_the_whole_bounding_box() {
        let placed = centre(vec![box_at(0, 0, 3, 2), box_at(5, 4, 3, 2)], AREA);
        let (horizontal, vertical) = ((AREA.cols - 8).div_euclid(2), (AREA.rows - 6).div_euclid(2));
        assert_eq!(
            placed,
            vec![
                box_at(AREA.col + horizontal, AREA.row + vertical, 3, 2),
                box_at(AREA.col + horizontal + 5, AREA.row + vertical + 4, 3, 2),
            ]
        );
    }

    #[test]
    fn centre_of_nothing_is_nothing() {
        assert!(centre(Vec::new(), AREA).is_empty());
    }

    #[test]
    fn centre_centres_an_overflowing_diagram_before_the_areas_corner() {
        let (span, height) = (AREA.cols + 20, 4);
        let placed = centre(vec![box_at(0, 0, span, height)], AREA);
        assert_eq!(
            (placed[0].x, placed[0].y),
            (
                AREA.col + (AREA.cols - span).div_euclid(2),
                AREA.row + (AREA.rows - height).div_euclid(2)
            )
        );
        assert!(placed[0].x < AREA.col);
    }

    #[test]
    fn centre_cuts_the_extra_column_of_an_odd_overflow_on_the_left() {
        let span = AREA.cols + 3;
        let placed = centre(vec![box_at(0, 0, span, 4)], AREA);
        let cut_on_left = AREA.col - placed[0].x;
        let cut_on_right = placed[0].x + span - (AREA.col + AREA.cols);
        assert_eq!(cut_on_left, cut_on_right + 1);
    }

    fn state(selected: Option<Vec<usize>>) -> State {
        let boxes = vec![node_with_children("root", vec![node("A"), node("B")])];
        new_state(boxes, Mode::Command, selected)
    }

    fn body_of(window: Area) -> Area {
        Area {
            rows: window.rows - FOOTER_ROWS,
            ..window
        }
    }

    fn foot_of(window: Area) -> Area {
        Area {
            row: window.row + window.rows - FOOTER_ROWS,
            rows: FOOTER_ROWS,
            ..window
        }
    }

    #[test]
    fn editor_returns_the_body_then_the_footer() {
        let state = state(None);
        let areas: Vec<Area> = editor(&state, WINDOW)
            .into_iter()
            .map(|(area, _)| area)
            .collect();
        assert_eq!(areas, vec![body_of(WINDOW), foot_of(WINDOW)]);
    }

    #[test]
    fn editor_centres_the_diagram_in_the_body() {
        let state = state(None);
        let screen = editor(&state, WINDOW);
        let body = body_of(WINDOW);
        assert_eq!(screen[0].1, centre(layout::diagram(state.doc.tree()), body));
    }

    #[test]
    fn editor_puts_the_cursor_in_the_body_when_something_is_selected() {
        let selected = Some(vec![0]);
        let state = state(selected.clone());
        let screen = editor(&state, WINDOW);
        assert_eq!(
            screen[0].1,
            centre(
                with_cursor(layout::diagram(state.doc.tree()), selected),
                body_of(WINDOW)
            )
        );
        assert!(screen[0]
            .1
            .iter()
            .any(|placement| matches!(placement.node, PlacementNode::Cursor(_))));
    }

    #[test]
    fn editor_fills_the_footer_with_one_box_as_wide_as_the_window() {
        let state = state(None);
        let screen = editor(&state, WINDOW);
        let foot = foot_of(WINDOW);
        assert_eq!(
            screen[1].1,
            vec![Placement {
                node: PlacementNode::Box {
                    colour: Some(FOOTER_COLOUR),
                    fill: Some(FOOTER_COLOUR),
                    rounded: false,
                },
                x: foot.col,
                y: foot.row,
                width: WINDOW.cols,
                height: FOOTER_ROWS,
            }]
        );
    }

    #[test]
    fn colour_of_plain_is_the_default_foreground() {
        assert_eq!(colour(None), palette(FOREGROUND).unwrap());
    }

    #[test]
    fn colour_of_a_palette_index_is_the_palette_entry() {
        assert_eq!(colour(Some(2)), palette(2).unwrap());
    }
}
