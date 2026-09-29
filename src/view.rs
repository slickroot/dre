use std::borrow::Cow;

use crate::composer;
use crate::layout::tree;
use crate::state::{CommandStatus, FooterMode, FooterModel, Mode, State};
use crate::style::{self, FOOTER_FILL_OPACITY, FOREGROUND};

pub use crate::composer::Area;
pub type Sides = (bool, bool, bool, bool);
pub const ALL_SIDES: Sides = (true, true, true, true);
pub const NO_SIDES: Sides = (false, false, false, false);

pub(crate) const BOX_HEIGHT: i64 = 3;
pub(crate) const BORDER: i64 = 4;
pub(crate) const GAP_WIDTH: i64 = 8;
pub(crate) const FOOTER_ROWS: i64 = BOX_HEIGHT;
pub(crate) const LED_WIDTH: i64 = 2;
pub(crate) const BRACKET_MARGIN: i64 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct Label<'a> {
    pub text: Cow<'a, str>,
    pub colour: Option<u8>,
    pub bold: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arrow {
    pub stops: Vec<i64>,
    pub shaft: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Caret;

#[derive(Debug, Clone, PartialEq)]
pub struct Cursor;

#[derive(Debug, Clone, PartialEq)]
pub enum PlacementNode<'a> {
    Box {
        colour: Option<u8>,
        fill: Option<u8>,
        opacity: Option<f64>,
        rounded: bool,
        sides: Sides,
        border: i64,
    },
    Label(Label<'a>),
    Arrow(Arrow),
    Caret(Caret),
    Brackets {
        border: i64,
    },
    Cursor(Cursor),
    Led {
        colour: u8,
        lit: bool,
    },
}

impl<'a> PlacementNode<'a> {
    pub(crate) fn is_decoration(&self) -> bool {
        match self {
            PlacementNode::Brackets { .. } | PlacementNode::Caret(_) => true,
            PlacementNode::Box { .. }
            | PlacementNode::Label(_)
            | PlacementNode::Arrow(_)
            | PlacementNode::Cursor(_)
            | PlacementNode::Led { .. } => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Placement<'a> {
    pub node: PlacementNode<'a>,
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

impl From<&Placement<'_>> for Geometry {
    fn from(placement: &Placement<'_>) -> Self {
        Geometry {
            x: placement.x,
            y: placement.y,
            width: placement.width,
            height: placement.height,
        }
    }
}

impl Placement<'_> {
    pub fn geometry(&self) -> Geometry {
        self.into()
    }
}

pub(crate) fn interior(label: &str) -> i64 {
    (label.chars().count() as i64).max(1)
}

pub(crate) fn label_centre(width: i64, label: &str) -> i64 {
    let leftover = width - 2 - interior(label);
    1 + leftover - leftover.div_euclid(2)
}

const NO_NAME: &str = "[no name — press n to name it]";
const PLACEHOLDER: &str = "type a name";
const FOOTER_SUFFIX: &str = "dre";
const MOVE: &str = "MOVE";
const WRITE: &str = "WRITE";
const NAME: &str = "NAME";

struct Column {
    node: PlacementNode<'static>,
    width: i64,
    padding: u8,
    cursor: Option<i64>,
}

fn stack_columns(columns: Vec<Column>, y: i64) -> (Vec<Placement<'static>>, i64) {
    let mut placements = Vec::with_capacity(columns.len());
    let mut x = 0;
    for column in columns {
        x += column.padding as i64;
        placements.push(Placement {
            node: column.node,
            x,
            y,
            width: column.width,
            height: 1,
        });
        if let Some(offset) = column.cursor {
            placements.push(Placement {
                node: PlacementNode::Cursor(Cursor),
                x: x + offset,
                y,
                width: 1,
                height: 1,
            });
        }
        x += column.width + column.padding as i64;
    }
    (placements, x)
}

fn footer_mode_word(mode: FooterMode) -> &'static str {
    match mode {
        FooterMode::Move => MOVE,
        FooterMode::Write => WRITE,
        FooterMode::Naming => NAME,
    }
}

fn footer_filename(model: &FooterModel) -> &str {
    let placeholder = match model.mode {
        FooterMode::Naming => PLACEHOLDER,
        FooterMode::Move | FooterMode::Write => NO_NAME,
    };
    model.filename.as_deref().unwrap_or(placeholder)
}

const FILENAME_PREFIX: &str = "• ";
const FILENAME_SUFFIX: &str = " •";

pub(crate) fn footer(model: &FooterModel) -> Vec<Placement<'static>> {
    let mode_word = footer_mode_word(model.mode);
    let filename = footer_filename(model);
    let padded_filename = format!("{FILENAME_PREFIX}{filename}{FILENAME_SUFFIX}");
    let columns = vec![
        Column {
            node: PlacementNode::Led {
                colour: match model.mode {
                    FooterMode::Move => style::LIME,
                    FooterMode::Write => style::VIOLET,
                    FooterMode::Naming => style::AMBER,
                },
                lit: match model.mode {
                    FooterMode::Move => model.flash,
                    FooterMode::Write | FooterMode::Naming => true,
                },
            },
            width: LED_WIDTH,
            padding: 1,
            cursor: None,
        },
        Column {
            node: PlacementNode::Label(Label {
                text: Cow::Borrowed(mode_word),
                colour: None,
                bold: true,
            }),
            width: interior(mode_word),
            padding: 1,
            cursor: None,
        },
        Column {
            node: PlacementNode::Label(Label {
                text: Cow::Owned(padded_filename.clone()),
                colour: Some(style::DIM),
                bold: false,
            }),
            width: interior(&padded_filename),
            padding: 0,
            cursor: model
                .cursor
                .map(|c| c as i64 + FILENAME_PREFIX.chars().count() as i64),
        },
        Column {
            node: PlacementNode::Label(Label {
                text: Cow::Borrowed(FOOTER_SUFFIX),
                colour: None,
                bold: true,
            }),
            width: interior(FOOTER_SUFFIX),
            padding: 1,
            cursor: None,
        },
    ];
    let (mut placements, width) = stack_columns(columns, BOX_HEIGHT / 2);
    placements.insert(
        0,
        Placement {
            node: PlacementNode::Box {
                colour: None,
                fill: Some(FOREGROUND),
                opacity: Some(FOOTER_FILL_OPACITY),
                rounded: false,
                sides: if model.mode == FooterMode::Naming {
                    ALL_SIDES
                } else {
                    NO_SIDES
                },
                border: 1,
            },
            x: 0,
            y: 0,
            width,
            height: BOX_HEIGHT,
        },
    );
    placements
}

pub(crate) fn command_status(status: &Option<CommandStatus>) -> Vec<Placement<'static>> {
    let Some(status) = status else {
        return vec![];
    };
    let text = format!(
        "[\"{}\" {} {}ms]",
        status.key, status.name, status.duration_ms
    );
    vec![Placement {
        width: text.chars().count() as i64,
        height: 1,
        x: 0,
        y: 0,
        node: PlacementNode::Label(Label {
            text: Cow::Owned(text),
            colour: None,
            bold: false,
        }),
    }]
}

pub type Scene<'a> = Vec<(Area, Vec<Placement<'a>>)>;

pub fn editor(state: &State, window: Area) -> Scene<'_> {
    let [body_area, foot] = composer::stack([None, Some(FOOTER_ROWS)], window);
    let footer = align_right(footer(&state.footer()), foot);
    let body_scene = body(state, body_area);
    vec![
        (body_area, body_scene.into_iter().next().unwrap().1),
        (foot, footer),
        (
            foot,
            align_center(command_status(&state.command_status()), foot),
        ),
    ]
}

pub fn body(state: &State, area: Area) -> Scene<'_> {
    let editing = match state.mode() {
        Mode::Insert { cursor } => state.selected().map(|path| (path, *cursor)),
        _ => None,
    };
    vec![(
        area,
        centre(
            tree::diagram(state.doc().tree(), editing, state.selected()),
            area,
        ),
    )]
}

pub(crate) fn shift(placements: Vec<Placement<'_>>, area: Area) -> Vec<Placement<'_>> {
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

pub(crate) fn centre(placements: Vec<Placement<'_>>, area: Area) -> Vec<Placement<'_>> {
    let content = || {
        placements
            .iter()
            .filter(|placement| !placement.node.is_decoration())
    };
    let Some(min_x) = content().map(|placement| placement.x).min() else {
        return placements;
    };
    let span = content()
        .map(|placement| placement.x + placement.width)
        .max()
        .unwrap()
        - min_x;
    let height = content()
        .map(|placement| placement.y + placement.height)
        .max()
        .unwrap();
    let horizontal = (area.cols - span).div_euclid(2);
    let vertical = (area.rows - height).div_euclid(2);
    shift(offset(placements, horizontal, vertical), area)
}

pub(crate) fn align_right(placements: Vec<Placement<'_>>, area: Area) -> Vec<Placement<'_>> {
    let Some(bounds) = placements.first() else {
        return placements;
    };
    let dx = area.col + area.cols - bounds.width;
    let dy = area.row + area.rows - bounds.height;
    offset(placements, dx, dy)
}

pub(crate) fn align_center(placements: Vec<Placement<'_>>, area: Area) -> Vec<Placement<'_>> {
    let Some(bounds) = placements.first() else {
        return placements;
    };
    let dx = area.col + (area.cols - bounds.width) / 2;
    let dy = area.row + area.rows - bounds.height;
    offset(placements, dx, dy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{node, node_with_children};
    use crate::state::{new_state, Mode};
    use crate::view::{PlacementNode, FOOTER_ROWS};

    const WINDOW: Area = Area {
        col: 1,
        row: 4,
        cols: 80,
        rows: 24,
    };

    fn state(selected: Option<Vec<usize>>) -> State {
        new_state(
            vec![node_with_children("root", vec![node("A"), node("B")])],
            Mode::Command,
            selected,
        )
    }

    #[test]
    fn body_returns_one_entry_for_the_requested_area() {
        let state = state(None);
        let scene = body(&state, WINDOW);
        assert_eq!(scene.len(), 1);
        assert_eq!(scene[0].0, WINDOW);
    }

    #[test]
    fn editor_returns_body_then_footer_then_command_status() {
        let state = state(None);
        let scene = editor(&state, WINDOW);
        assert_eq!(scene.len(), 3);
        assert_eq!(scene[0].0.rows, WINDOW.rows - FOOTER_ROWS);
        assert_eq!(scene[1].0.row, scene[0].0.row + scene[0].0.rows);
        assert!(scene[1]
            .1
            .iter()
            .any(|placement| matches!(placement.node, PlacementNode::Led { .. })));
        assert_eq!(scene[2].0, scene[1].0);
        assert!(scene[2].1.is_empty());
    }

    #[test]
    fn editor_body_matches_body_for_the_matching_area() {
        let state = state(Some(vec![0]));
        let scene = editor(&state, WINDOW);
        assert_eq!(scene[0].1, body(&state, scene[0].0)[0].1);
    }

    #[test]
    fn insert_mode_selection_keeps_the_caret_in_the_body_scene() {
        let state = new_state(
            vec![node_with_children("root", vec![node("A"), node("B")])],
            Mode::Insert { cursor: 1 },
            Some(vec![0]),
        );
        assert!(body(&state, WINDOW)[0]
            .1
            .iter()
            .any(|placement| matches!(placement.node, PlacementNode::Caret(_))));
    }

    fn assert_selected_box_is_bracketed(mode: Mode) {
        let state = new_state(
            vec![node_with_children("root", vec![node("A"), node("B")])],
            mode,
            Some(vec![0]),
        );
        assert!(body(&state, WINDOW)[0]
            .1
            .iter()
            .any(|placement| matches!(placement.node, PlacementNode::Brackets { .. })));
    }

    #[test]
    fn command_mode_selection_is_bracketed() {
        assert_selected_box_is_bracketed(Mode::Command);
    }

    #[test]
    fn insert_mode_selection_is_bracketed() {
        assert_selected_box_is_bracketed(Mode::Insert { cursor: 1 });
    }

    #[test]
    fn name_prompt_mode_selection_is_bracketed() {
        assert_selected_box_is_bracketed(Mode::NamePrompt {
            name: String::new(),
            quits: false,
        });
    }

    fn led_column(padding: u8, cursor: Option<i64>) -> Column {
        Column {
            node: PlacementNode::Led {
                colour: style::LIME,
                lit: true,
            },
            width: LED_WIDTH,
            padding,
            cursor,
        }
    }

    #[test]
    fn stack_columns_emits_a_cursor_placement_right_after_its_column_when_set() {
        let columns = vec![led_column(1, None), led_column(1, Some(2))];
        let (placements, _) = stack_columns(columns, 0);
        let column_placement = &placements[1];
        assert!(matches!(column_placement.node, PlacementNode::Led { .. }));
        let cursor_placement = &placements[2];
        assert!(matches!(cursor_placement.node, PlacementNode::Cursor(_)));
        assert_eq!(cursor_placement.x, column_placement.x + 2);
        assert_eq!(cursor_placement.y, column_placement.y);
    }

    #[test]
    fn stack_columns_emits_no_cursor_placement_when_column_cursor_is_none() {
        let columns = vec![led_column(1, None)];
        let (placements, _) = stack_columns(columns, 0);
        assert!(!placements
            .iter()
            .any(|placement| matches!(placement.node, PlacementNode::Cursor(_))));
    }

    #[test]
    fn stack_columns_applies_padding_symmetrically_and_additively_between_columns() {
        let first_padding: u8 = 2;
        let second_padding: u8 = 3;
        let columns = vec![
            led_column(first_padding, None),
            led_column(second_padding, None),
        ];
        let (placements, _) = stack_columns(columns, 0);
        let first = &placements[0];
        let second = &placements[1];
        let gap = second.x - (first.x + first.width);
        assert_eq!(gap, first_padding as i64 + second_padding as i64);
    }

    fn footer_model(mode: FooterMode, flash: bool) -> FooterModel {
        FooterModel {
            mode,
            filename: None,
            cursor: None,
            flash,
        }
    }

    fn led_lit(model: &FooterModel) -> bool {
        footer(model)
            .into_iter()
            .find_map(|placement| match placement.node {
                PlacementNode::Led { lit, .. } => Some(lit),
                _ => None,
            })
            .expect("footer placements should contain a Led node")
    }

    #[test]
    fn footer_led_is_unlit_in_move_mode_when_not_flashing() {
        let model = footer_model(FooterMode::Move, false);
        assert!(!led_lit(&model));
    }

    #[test]
    fn footer_led_is_lit_in_move_mode_when_flashing() {
        let model = footer_model(FooterMode::Move, true);
        assert!(led_lit(&model));
    }

    #[test]
    fn footer_led_is_lit_in_write_mode_regardless_of_flash() {
        let model = footer_model(FooterMode::Write, false);
        assert!(led_lit(&model));
    }

    #[test]
    fn footer_led_is_lit_in_naming_mode_regardless_of_flash() {
        let model = footer_model(FooterMode::Naming, false);
        assert!(led_lit(&model));
    }

    fn label_bold(model: &FooterModel, text: &str) -> bool {
        footer(model)
            .into_iter()
            .find_map(|placement| match placement.node {
                PlacementNode::Label(label) if label.text == text => Some(label.bold),
                _ => None,
            })
            .unwrap_or_else(|| {
                panic!("footer placements should contain a Label with text {text:?}")
            })
    }

    #[test]
    fn footer_mode_word_is_bold_in_every_mode() {
        for mode in [FooterMode::Move, FooterMode::Write, FooterMode::Naming] {
            let model = footer_model(mode, false);
            let mode_word = footer_mode_word(mode);
            assert!(label_bold(&model, mode_word));
        }
    }

    #[test]
    fn footer_suffix_is_bold_in_every_mode() {
        for mode in [FooterMode::Move, FooterMode::Write, FooterMode::Naming] {
            let model = footer_model(mode, false);
            assert!(label_bold(&model, FOOTER_SUFFIX));
        }
    }

    #[test]
    fn footer_filename_is_not_bold_in_every_mode() {
        for mode in [FooterMode::Move, FooterMode::Write, FooterMode::Naming] {
            let model = footer_model(mode, false);
            let filename = footer_filename(&model);
            let padded_filename = format!("{FILENAME_PREFIX}{filename}{FILENAME_SUFFIX}");
            assert!(!label_bold(&model, &padded_filename));
        }
    }

    #[test]
    fn command_status_is_empty_when_none() {
        assert_eq!(command_status(&None), vec![]);
    }

    #[test]
    fn command_status_renders_the_key_name_and_duration() {
        let status = Some(CommandStatus {
            key: "b".to_string(),
            name: "Add a child box",
            duration_ms: 20,
        });
        let placements = command_status(&status);
        assert_eq!(placements.len(), 1);
        let expected = format!("[\"{}\" {} {}ms]", "b", "Add a child box", 20);
        match &placements[0].node {
            PlacementNode::Label(label) => assert_eq!(label.text, expected),
            other => panic!("expected a Label placement, got {other:?}"),
        }
    }

    #[test]
    fn align_center_returns_an_empty_vec_for_an_empty_vec() {
        let area = Area {
            col: 0,
            row: 0,
            cols: 20,
            rows: 3,
        };
        assert_eq!(align_center(vec![], area), vec![]);
    }

    #[test]
    fn align_center_centers_a_placement_horizontally_within_the_area() {
        let area = Area {
            col: 1,
            row: 4,
            cols: 20,
            rows: 3,
        };
        let placements = vec![Placement {
            node: PlacementNode::Label(Label {
                text: Cow::Borrowed("hi"),
                colour: None,
                bold: false,
            }),
            x: 0,
            y: 0,
            width: 6,
            height: 1,
        }];
        let result = align_center(placements, area);
        assert_eq!(result[0].x, area.col + (area.cols - 6) / 2);
    }

    #[test]
    fn editor_places_command_status_centered_on_the_footer_row_when_present() {
        let mut state = state(None);
        state = crate::state::set_command_status(
            state,
            "b".to_string(),
            "Add a child box",
            std::time::Duration::from_millis(20),
        );
        let scene = editor(&state, WINDOW);
        let (area, placements) = &scene[2];
        assert_eq!(*area, scene[1].0);
        assert_eq!(placements.len(), 1);
        match &placements[0].node {
            PlacementNode::Label(label) => {
                assert_eq!(label.text, "[\"b\" Add a child box 20ms]")
            }
            other => panic!("expected a Label placement, got {other:?}"),
        }
    }
}
