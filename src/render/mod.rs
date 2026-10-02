use std::io::{self, Write};

#[cfg(test)]
use crate::style::palette;
use crate::view::Scene;

#[cfg(not(target_arch = "wasm32"))]
mod brackets;
#[cfg(not(target_arch = "wasm32"))]
mod font;
#[cfg(not(target_arch = "wasm32"))]
mod shapes;
mod svg;
#[cfg(not(target_arch = "wasm32"))]
mod terminal;
#[cfg(not(target_arch = "wasm32"))]
mod tiles;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod virtual_terminal;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use font::GlyphCache;
pub use svg::{SvgRenderer, FULL_HD_HEIGHT, FULL_HD_WIDTH};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use terminal::{TerminalRenderer, CACHE_LIMIT};

pub trait Renderer {
    fn render(&mut self, scene: &Scene<'_>, out: &mut impl Write) -> io::Result<()>;
}

const ARROWHEAD_ANGLE_DEG: f64 = 30.0;
fn arrowhead_depth(edge_length: f64) -> f64 {
    edge_length * ARROWHEAD_ANGLE_DEG.to_radians().cos()
}
fn arrowhead_slope(_edge_length: f64) -> f64 {
    ARROWHEAD_ANGLE_DEG.to_radians().tan()
}

const ROUNDED_RADIUS: i64 = 20;
const BRACKET_OFFSET: i64 = 8;
const BRACKET_ARM: i64 = 14;

const LED_DOT_RATIO: f64 = 0.28;
const LED_HALO_ALPHA: f64 = 0.45;
const LED_DIM_ALPHA: f64 = 0.3;

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
const OPAQUE: u8 = 255;
const ARROW_OPACITY: f64 = 0.5;
fn colour(colour: Option<u8>) -> (u8, u8, u8) {
    crate::style::rgb(colour)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{node, node_with_children};
    use crate::layout::tree::diagram;
    use crate::state::{new_state, Mode};
    use crate::style::FOREGROUND;
    use crate::test_support::handle_key;
    use crate::view::{align_right, body, centre, editor, shift, Area, Placement};
    use crate::view::{
        Cursor, Label, PlacementNode, ALL_SIDES, BORDER, BOX_HEIGHT, FOOTER_ROWS, LED_WIDTH,
    };
    use crate::State;

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
            colour: crate::style::rgb(None),
            fill: None,
            opacity: None,
            solid_fill: None,
            rounded: false,
            sides: ALL_SIDES,
            border: BORDER,
            grow: false,
        }
    }

    fn box_at(x: i64, y: i64, width: i64, height: i64) -> Placement<'static> {
        Placement {
            node: plain_box(),
            x,
            y,
            width,
            height,
            depth: 0,
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
        colored_label_at(text, x, y, None)
    }

    fn bold_label_at(text: &str, x: i64, y: i64) -> Placement<'_> {
        Placement {
            node: PlacementNode::Label(Label {
                text: text.into(),
                colour: crate::style::rgb(None),
                bold: true,
            }),
            x,
            y,
            width: text.chars().count() as i64,
            height: 1,
            depth: 1,
        }
    }

    fn colored_label_at(text: &str, x: i64, y: i64, colour: Option<u8>) -> Placement<'_> {
        Placement {
            node: PlacementNode::Label(Label {
                text: text.into(),
                colour: crate::style::rgb(colour),
                bold: false,
            }),
            x,
            y,
            width: text.chars().count() as i64,
            height: 1,
            depth: 1,
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
        let placed = align_right(vec![box_at(0, 0, 10, 3), label_at("plans", 1, 1)], AREA);
        let dx = placed[0].x;
        let dy = placed[0].y;
        assert_eq!(placed[1], label_at("plans", 1 + dx, 1 + dy));
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

    fn brackets_at(x: i64, y: i64, width: i64, height: i64) -> Placement<'static> {
        Placement {
            node: PlacementNode::Brackets { border: 1 },
            x,
            y,
            width,
            height,
            depth: 0,
        }
    }

    fn caret_at(x: i64, y: i64) -> Placement<'static> {
        Placement {
            node: PlacementNode::Caret(crate::view::Caret),
            x,
            y,
            width: 1,
            height: 1,
            depth: 0,
        }
    }

    #[test]
    fn centre_ignores_brackets_and_caret_when_computing_the_bounding_box() {
        let (width, height) = (6, 4);
        let content_only = centre(vec![box_at(0, 0, width, height)], AREA);
        let with_decorations = centre(
            vec![
                box_at(0, 0, width, height),
                brackets_at(-1, -1, width + 2, height + 2),
                caret_at(width + 5, height + 5),
            ],
            AREA,
        );
        assert_eq!(content_only[0], with_decorations[0]);
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
        assert_eq!(
            areas,
            vec![body_of(WINDOW), foot_of(WINDOW), foot_of(WINDOW)]
        );
    }

    #[test]
    fn editor_centres_the_diagram_in_the_body() {
        let state = state(None);
        let screen = editor(&state, WINDOW);
        let body = body_of(WINDOW);
        assert_eq!(
            screen[0].1,
            centre(diagram(state.doc().tree(), None, None), body)
        );
    }

    #[test]
    fn body_is_the_body_half_of_the_editor() {
        let state = state(Some(vec![0]));
        assert_eq!(
            body(&state, body_of(WINDOW))[0].1,
            editor(&state, WINDOW)[0].1
        );
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
                diagram(state.doc().tree(), Some((&[0], 1)), state.selected()),
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
        assert_footer_is_bottom_right(&state, "MOVE", "plans");
    }

    #[test]
    fn editor_ends_with_the_hint_and_dre_in_the_bottom_right_corner_when_there_is_no_path() {
        let state = state(None);
        assert_footer_is_bottom_right(&state, "MOVE", "[no name — press n to name it]");
    }

    const COLUMN_PADDING: i64 = 1;
    const COLUMN_GAP: i64 = COLUMN_PADDING * 2;
    const FILENAME_PREFIX: &str = "\u{2022} ";
    const FILENAME_SUFFIX: &str = " \u{2022}";

    fn padded_filename(name: &str) -> String {
        format!("{FILENAME_PREFIX}{name}{FILENAME_SUFFIX}")
    }

    fn box_at_bottom_right(foot: Area, mode_word: &str, name: &str) -> (i64, i64, i64, i64) {
        let box_width = COLUMN_PADDING
            + LED_WIDTH
            + COLUMN_GAP
            + mode_word.chars().count() as i64
            + COLUMN_PADDING
            + padded_filename(name).chars().count() as i64
            + COLUMN_PADDING
            + FOOTER_SUFFIX.chars().count() as i64
            + COLUMN_PADDING;
        (
            foot.col + foot.cols - box_width,
            foot.row + foot.rows - FOOTER_ROWS,
            box_width,
            FOOTER_ROWS,
        )
    }

    fn led_x(box_x: i64) -> i64 {
        box_x + COLUMN_PADDING
    }

    fn label_x(box_x: i64) -> i64 {
        led_x(box_x) + LED_WIDTH + COLUMN_GAP
    }

    fn filename_x(box_x: i64, mode_word: &str) -> i64 {
        label_x(box_x) + mode_word.chars().count() as i64 + COLUMN_PADDING
    }

    const FOOTER_SUFFIX: &str = "dre";

    fn suffix_x(box_x: i64, mode_word: &str, name: &str) -> i64 {
        filename_x(box_x, mode_word) + padded_filename(name).chars().count() as i64 + COLUMN_PADDING
    }

    fn prompt_cursor_x(typed: &str, displayed: &str, mode_word: &str) -> i64 {
        let foot = foot_of(WINDOW);
        let (box_x, _, _, _) = box_at_bottom_right(foot, mode_word, displayed);
        filename_x(box_x, mode_word)
            + FILENAME_PREFIX.chars().count() as i64
            + typed.chars().count() as i64
    }

    #[test]
    fn the_prompt_shows_a_placeholder_with_the_cursor_on_its_first_character() {
        let state = handle_key(state(None), "n");
        assert_footer_is_bottom_right_with_cursor(
            &state,
            "NAME",
            "type a name",
            prompt_cursor_x("", "type a name", "NAME"),
        );
    }

    #[test]
    fn the_prompt_shows_the_typed_name_with_the_cursor_after_its_last_character() {
        let state = handle_key(handle_key(handle_key(state(None), "n"), "a"), "b");
        assert_footer_is_bottom_right_with_cursor(
            &state,
            "NAME",
            "ab",
            prompt_cursor_x("ab", "ab", "NAME"),
        );
    }

    #[test]
    fn cancelling_the_prompt_restores_the_footer() {
        let state = handle_key(handle_key(state(None), "n"), "\x1b");
        assert_footer_is_bottom_right(&state, "MOVE", "[no name — press n to name it]");
    }

    #[test]
    fn the_quit_prompt_shows_a_placeholder_with_the_cursor_on_its_first_character() {
        let state = handle_key(state(None), "q");
        assert_footer_is_bottom_right_with_cursor(
            &state,
            "NAME",
            "type a name",
            prompt_cursor_x("", "type a name", "NAME"),
        );
    }

    #[test]
    fn the_quit_prompt_shows_the_typed_name_with_the_cursor_after_its_last_character() {
        let state = handle_key(handle_key(handle_key(state(None), "q"), "a"), "b");
        assert_footer_is_bottom_right_with_cursor(
            &state,
            "NAME",
            "ab",
            prompt_cursor_x("ab", "ab", "NAME"),
        );
    }

    fn assert_footer_is_bottom_right_with_cursor(
        state: &State,
        mode_word: &str,
        name: &str,
        x: i64,
    ) {
        let foot = foot_of(WINDOW);
        let (box_x, box_y, _, _) = box_at_bottom_right(foot, mode_word, name);
        let footer = &editor(state, WINDOW)[1].1;
        assert_eq!(footer.len(), 6);
        assert_eq!(
            footer[2],
            bold_label_at(mode_word, label_x(box_x), box_y + BOX_HEIGHT / 2)
        );
        assert_eq!(
            footer[3],
            colored_label_at(
                &padded_filename(name),
                filename_x(box_x, mode_word),
                box_y + BOX_HEIGHT / 2,
                Some(crate::style::DIM),
            )
        );
        assert_eq!(
            footer[4],
            Placement {
                node: PlacementNode::Cursor(Cursor),
                x,
                y: box_y + BOX_HEIGHT / 2,
                width: 1,
                height: 1,
                depth: 1,
            }
        );
        assert_eq!(
            footer[5],
            bold_label_at(
                FOOTER_SUFFIX,
                suffix_x(box_x, mode_word, name),
                box_y + BOX_HEIGHT / 2,
            )
        );
    }

    fn assert_footer_is_bottom_right(state: &State, mode_word: &str, name: &str) {
        let screen = editor(state, WINDOW);
        let foot = foot_of(WINDOW);
        let (box_x, box_y, box_width, box_height) = box_at_bottom_right(foot, mode_word, name);
        let footer = &screen[1].1;
        assert_eq!(footer.len(), 5);
        assert_eq!(
            (footer[0].x, footer[0].y, footer[0].width, footer[0].height),
            (box_x, box_y, box_width, box_height)
        );
        assert_eq!(
            (footer[1].x, footer[1].y, footer[1].width, footer[1].height),
            (led_x(box_x), box_y + BOX_HEIGHT / 2, 2, 1)
        );
        assert_eq!(
            footer[2],
            bold_label_at(mode_word, label_x(box_x), box_y + BOX_HEIGHT / 2)
        );
        assert_eq!(
            footer[3],
            colored_label_at(
                &padded_filename(name),
                filename_x(box_x, mode_word),
                box_y + BOX_HEIGHT / 2,
                Some(crate::style::DIM),
            )
        );
        assert_eq!(
            footer[4],
            bold_label_at(
                FOOTER_SUFFIX,
                suffix_x(box_x, mode_word, name),
                box_y + BOX_HEIGHT / 2,
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

    #[test]
    fn colour_of_dim_is_the_dim_palette_entry() {
        assert_eq!(
            colour(Some(crate::style::DIM)),
            palette(crate::style::DIM).unwrap()
        );
    }
}
