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

pub(crate) fn editor(state: &State, window: Area) -> Vec<(Area, Vec<Placement<'_>>)> {
    let [body, foot] = composer::stack([None, Some(FOOTER_ROWS)], window);
    vec![
        (body, self::body(state, body)),
        (foot, align_right(layout::footer(state.footer()), foot)),
    ]
}

pub(crate) fn body(state: &State, area: Area) -> Vec<Placement<'_>> {
    centre(
        with_cursor(layout::diagram(state.doc.tree()), state.selected.clone()),
        area,
    )
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

fn align_right(placements: Vec<Placement<'_>>, area: Area) -> Vec<Placement<'_>> {
    placements
        .into_iter()
        .map(|placement| Placement {
            x: area.col + area.cols - placement.width,
            y: area.row,
            ..placement
        })
        .collect()
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
    use crate::layout::{Label, PlacementNode};
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

    fn label_at(text: &str, x: i64, y: i64) -> Placement<'_> {
        Placement {
            node: PlacementNode::Label(Label { text, path: vec![] }),
            x,
            y,
            width: text.chars().count() as i64,
            height: 1,
        }
    }

    #[test]
    fn align_right_meets_the_areas_right_edge_on_the_areas_row() {
        let placed = align_right(vec![label_at("plans", 0, 0)], AREA);
        assert_eq!(placed[0].x + placed[0].width, AREA.col + AREA.cols);
        assert_eq!(placed[0].y, AREA.row);
    }

    #[test]
    fn align_right_keeps_the_width_and_height_of_a_placement() {
        let placed = align_right(vec![label_at("plans", 0, 0)], AREA);
        assert_eq!((placed[0].width, placed[0].height), (5, 1));
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
    fn body_is_the_body_half_of_the_editor() {
        let state = state(Some(vec![0]));
        assert_eq!(body(&state, body_of(WINDOW)), editor(&state, WINDOW)[0].1);
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

    fn state_saved_to(path: &str) -> State {
        let mut state = state(None);
        state.set_save_to(Some(path.to_string()));
        state
    }

    #[test]
    fn editor_ends_with_the_name_and_dre_in_the_bottom_right_corner_when_there_is_a_path() {
        let state = state_saved_to("docs/plans.dre");
        assert_footer_is_bottom_right(&state, "plans \u{2022} dre");
    }

    #[test]
    fn editor_ends_with_no_name_and_dre_in_the_bottom_right_corner_when_there_is_no_path() {
        let state = state(None);
        assert_footer_is_bottom_right(&state, "[no name] \u{2022} dre");
    }

    fn assert_footer_is_bottom_right(state: &State, text: &str) {
        let screen = editor(state, WINDOW);
        let foot = foot_of(WINDOW);
        let width = text.chars().count() as i64;
        assert_eq!(
            screen[1].1,
            vec![label_at(text, foot.col + foot.cols - width, foot.row)]
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
