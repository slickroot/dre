use std::io::{self, Write};

use crate::composer::{self, Area};
use crate::layout::{self, with_caret, Caret, Placement, PlacementNode, FOOTER_ROWS};
use crate::palette::{palette, FOREGROUND};
use crate::state::{Mode, State};

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
    let mut footer = align_right(layout::footer(&state.footer()), foot);
    if let Mode::NamePrompt { name, .. } = state.mode() {
        footer.push(Placement {
            node: PlacementNode::Caret(Caret),
            x: footer[1].x + name.chars().count() as i64,
            y: footer[1].y,
            width: 1,
            height: 1,
        });
    }
    vec![(body, self::body(state, body)), (foot, footer)]
}

pub(crate) fn body(state: &State, area: Area) -> Vec<Placement<'_>> {
    let editing_caret: Option<(Vec<usize>, usize)> = match state.mode() {
        Mode::Insert { cursor } => state.selected().map(|path| (path.to_vec(), *cursor)),
        _ => None,
    };
    let editing = editing_caret.as_ref().map(|(path, _)| path.as_slice());
    centre(
        with_caret(
            layout::diagram(state.doc().tree(), editing, state.selected()),
            editing_caret,
        ),
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
    let Some(bounds) = placements.first() else {
        return placements;
    };
    let dx = area.col + area.cols - bounds.width;
    let dy = area.row + area.rows - bounds.height;
    offset(placements, dx, dy)
}

const ARROWHEAD_ANGLE_DEG: f64 = 30.0;
fn arrowhead_depth(edge_length: f64) -> f64 {
    edge_length * ARROWHEAD_ANGLE_DEG.to_radians().cos()
}
fn arrowhead_slope(_edge_length: f64) -> f64 {
    ARROWHEAD_ANGLE_DEG.to_radians().tan()
}

const CELL_WIDTH: i64 = 8;
const CELL_HEIGHT: i64 = 16;

const ROUNDED_RADIUS: i64 = 20;

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
const OPAQUE: u8 = 255;
const ARROW_OPACITY: f64 = 0.5;
pub(crate) const BOX_FILL_OPACITY: f64 = 0.3;
pub(crate) const FOOTER_FILL_OPACITY: f64 = 0.12;

fn colour(colour: Option<u8>) -> (u8, u8, u8) {
    palette(colour.unwrap_or(FOREGROUND)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{node, node_with_children};
    use crate::layout::{Label, PlacementNode, ALL_SIDES, BORDER, SIDE_PADDING};
    use crate::state::{new_state, Mode};
    use crate::test_support::handle_key;

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
            opacity: None,
            rounded: false,
            sides: ALL_SIDES,
            border: BORDER,
            selected: false,
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
            node: PlacementNode::Label(Label {
                text: text.into(),
                path: vec![],
            }),
            x,
            y,
            width: text.chars().count() as i64,
            height: 1,
        }
    }

    #[test]
    fn align_right_meets_the_areas_right_and_bottom_edge() {
        let placed = align_right(vec![label_at("plans", 0, 0)], AREA);
        assert_eq!(placed[0].x + placed[0].width, AREA.col + AREA.cols);
        assert_eq!(placed[0].y + placed[0].height, AREA.row + AREA.rows);
    }

    #[test]
    fn align_right_shifts_every_placement_by_the_same_delta() {
        let placed = align_right(
            vec![
                box_at(0, 0, 10, 3),
                label_at("plans", SIDE_PADDING / 2, SIDE_PADDING / 2),
            ],
            AREA,
        );
        let dx = placed[0].x;
        let dy = placed[0].y;
        assert_eq!(
            placed[1],
            label_at("plans", SIDE_PADDING / 2 + dx, SIDE_PADDING / 2 + dy)
        );
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
        assert_eq!(
            screen[0].1,
            centre(layout::diagram(state.doc().tree(), None, None), body)
        );
    }

    #[test]
    fn body_is_the_body_half_of_the_editor() {
        let state = state(Some(vec![0]));
        assert_eq!(body(&state, body_of(WINDOW)), editor(&state, WINDOW)[0].1);
    }

    #[test]
    fn command_mode_shows_no_caret_even_when_something_is_selected() {
        let state = state(Some(vec![0]));
        let screen = editor(&state, WINDOW);
        assert!(!screen[0]
            .1
            .iter()
            .any(|placement| matches!(placement.node, PlacementNode::Caret(_))));
    }

    #[test]
    fn insert_mode_puts_the_caret_in_the_body_at_the_typed_index() {
        let boxes = vec![node_with_children("root", vec![node("A"), node("B")])];
        let state = new_state(boxes, Mode::Insert { cursor: 1 }, Some(vec![0]));
        let screen = editor(&state, WINDOW);
        assert_eq!(
            screen[0].1,
            centre(
                with_caret(
                    layout::diagram(state.doc().tree(), Some(&[0]), state.selected()),
                    Some((vec![0], 1)),
                ),
                body_of(WINDOW)
            )
        );
        assert!(screen[0]
            .1
            .iter()
            .any(|placement| matches!(placement.node, PlacementNode::Caret(_))));
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
    fn editor_ends_with_the_hint_and_dre_in_the_bottom_right_corner_when_there_is_no_path() {
        let state = state(None);
        assert_footer_is_bottom_right(&state, "[no name — press n to name it] \u{2022} dre");
    }

    fn box_at_bottom_right(foot: Area, text: &str) -> (i64, i64, i64, i64) {
        let text_width = text.chars().count() as i64;
        let box_width = text_width + SIDE_PADDING * 2;
        (
            foot.col + foot.cols - box_width,
            foot.row + foot.rows - FOOTER_ROWS,
            box_width,
            FOOTER_ROWS,
        )
    }

    fn prompt_cursor_x(name: &str, text: &str) -> i64 {
        let foot = foot_of(WINDOW);
        let (box_x, _, box_width, _) = box_at_bottom_right(foot, text);
        box_x + layout::centre(box_width, text) + name.chars().count() as i64
    }

    #[test]
    fn the_prompt_shows_a_placeholder_with_the_cursor_on_its_first_character() {
        let state = handle_key(state(None), "n");
        let text = "type a name \u{2022} dre";
        assert_footer_is_bottom_right_with_cursor(&state, text, prompt_cursor_x("", text));
    }

    #[test]
    fn the_prompt_shows_the_typed_name_with_the_cursor_after_its_last_character() {
        let state = handle_key(handle_key(handle_key(state(None), "n"), "a"), "b");
        let text = "ab \u{2022} dre";
        assert_footer_is_bottom_right_with_cursor(&state, text, prompt_cursor_x("ab", text));
    }

    #[test]
    fn cancelling_the_prompt_restores_the_footer() {
        let state = handle_key(handle_key(state(None), "n"), "\x1b");
        assert_footer_is_bottom_right(&state, "[no name — press n to name it] \u{2022} dre");
    }

    #[test]
    fn the_quit_prompt_shows_a_placeholder_with_the_cursor_on_its_first_character() {
        let state = handle_key(state(None), "q");
        let text = "type a name \u{2022} dre";
        assert_footer_is_bottom_right_with_cursor(&state, text, prompt_cursor_x("", text));
    }

    #[test]
    fn the_quit_prompt_shows_the_typed_name_with_the_cursor_after_its_last_character() {
        let state = handle_key(handle_key(handle_key(state(None), "q"), "a"), "b");
        let text = "ab \u{2022} dre";
        assert_footer_is_bottom_right_with_cursor(&state, text, prompt_cursor_x("ab", text));
    }

    fn assert_footer_is_bottom_right_with_cursor(state: &State, text: &str, x: i64) {
        let foot = foot_of(WINDOW);
        let (box_x, box_y, box_width, _) = box_at_bottom_right(foot, text);
        let footer = &editor(state, WINDOW)[1].1;
        assert_eq!(footer.len(), 3);
        assert_eq!(
            footer[1],
            label_at(
                text,
                box_x + layout::centre(box_width, text),
                box_y + layout::BOX_HEIGHT / 2
            )
        );
        assert_eq!(
            footer[2],
            Placement {
                node: PlacementNode::Caret(Caret),
                x,
                y: box_y + layout::BOX_HEIGHT / 2,
                width: 1,
                height: 1
            }
        );
    }

    fn assert_footer_is_bottom_right(state: &State, text: &str) {
        let screen = editor(state, WINDOW);
        let foot = foot_of(WINDOW);
        let (box_x, box_y, box_width, box_height) = box_at_bottom_right(foot, text);
        let footer = &screen[1].1;
        assert_eq!(footer.len(), 2);
        assert_eq!(footer[0].node, layout::footer(text)[0].node);
        assert_eq!(
            (footer[0].x, footer[0].y, footer[0].width, footer[0].height),
            (box_x, box_y, box_width, box_height)
        );
        assert_eq!(
            footer[1],
            label_at(
                text,
                box_x + layout::centre(box_width, text),
                box_y + layout::BOX_HEIGHT / 2
            )
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
