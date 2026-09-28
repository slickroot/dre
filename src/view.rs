use std::borrow::Cow;

use crate::composer;
use crate::layout::tree;
use crate::state::{FooterMode, FooterModel, Mode, State};
use crate::style::{self, FOOTER_FILL_OPACITY, FOREGROUND};

pub use crate::composer::Area;
pub type Sides = (bool, bool, bool, bool);
pub const ALL_SIDES: Sides = (true, true, true, true);
pub const NO_SIDES: Sides = (false, false, false, false);

pub(crate) const BOX_HEIGHT: i64 = 3;
pub(crate) const BORDER: i64 = 4;
pub(crate) const SIDE_PADDING: i64 = 2;
pub(crate) const GAP_WIDTH: i64 = 8;
pub(crate) const FOOTER_ROWS: i64 = BOX_HEIGHT;
pub(crate) const LED_WIDTH: i64 = 2;
pub(crate) const LED_LABEL_GAP: i64 = 1;
pub(crate) const GLOW_MARGIN: i64 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct Label<'a> {
    pub text: Cow<'a, str>,
    pub colour: Option<u8>,
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
    Glow {
        colour: Option<u8>,
        rounded: bool,
    },
    Cursor(Cursor),
    Led {
        colour: u8,
        lit: bool,
    },
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
const FOOTER_SUFFIX: &str = " • dre";
const MOVE: &str = "MOVE";
const WRITE: &str = "WRITE";
const NAME: &str = "NAME";

struct Column {
    node: PlacementNode<'static>,
    width: i64,
    padding: i64,
}

fn stack_columns(columns: Vec<Column>, y: i64) -> (Vec<Placement<'static>>, i64) {
    let mut placements = Vec::with_capacity(columns.len());
    let mut x = 0;
    for column in columns {
        x += column.padding;
        placements.push(Placement {
            node: column.node,
            x,
            y,
            width: column.width,
            height: 1,
        });
        x += column.width + column.padding;
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

pub(crate) fn footer(model: &FooterModel) -> Vec<Placement<'static>> {
    let mode_word = footer_mode_word(model.mode);
    let columns = vec![
        Column {
            node: PlacementNode::Led {
                colour: match model.mode {
                    FooterMode::Move => style::LIME,
                    FooterMode::Write => style::VIOLET,
                    FooterMode::Naming => style::AMBER,
                },
                lit: !matches!(model.mode, FooterMode::Move),
            },
            width: LED_WIDTH,
            padding: SIDE_PADDING,
        },
        Column {
            node: PlacementNode::Label(Label {
                text: Cow::Borrowed(mode_word),
                colour: None,
            }),
            width: interior(mode_word),
            padding: LED_LABEL_GAP - SIDE_PADDING,
        },
    ];
    let (mut placements, word_end) = stack_columns(columns, BOX_HEIGHT / 2);
    let filename = footer_filename(model);
    let filename_x = word_end + SIDE_PADDING;
    let filename_width = interior(filename);
    placements.push(Placement {
        node: PlacementNode::Label(Label {
            text: Cow::Owned(filename.to_string()),
            colour: Some(style::DIM),
        }),
        x: filename_x,
        y: BOX_HEIGHT / 2,
        width: filename_width,
        height: 1,
    });
    let suffix_x = filename_x + filename_width;
    let suffix_width = interior(FOOTER_SUFFIX);
    placements.push(Placement {
        node: PlacementNode::Label(Label {
            text: Cow::Borrowed(FOOTER_SUFFIX),
            colour: None,
        }),
        x: suffix_x,
        y: BOX_HEIGHT / 2,
        width: suffix_width,
        height: 1,
    });
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
            width: suffix_x + suffix_width + SIDE_PADDING,
            height: BOX_HEIGHT,
        },
    );
    if let Some(offset) = model.cursor {
        placements.push(Placement {
            node: PlacementNode::Cursor(Cursor),
            x: filename_x + offset as i64,
            y: BOX_HEIGHT / 2,
            width: 1,
            height: 1,
        });
    }
    placements
}

pub type Scene<'a> = Vec<(Area, Vec<Placement<'a>>)>;

pub fn editor(state: &State, window: Area) -> Scene<'_> {
    let [body_area, foot] = composer::stack([None, Some(FOOTER_ROWS)], window);
    let footer = align_right(footer(&state.footer()), foot);
    let body_scene = body(state, body_area);
    vec![
        (body_area, body_scene.into_iter().next().unwrap().1),
        (foot, footer),
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

pub(crate) fn align_right(placements: Vec<Placement<'_>>, area: Area) -> Vec<Placement<'_>> {
    let Some(bounds) = placements.first() else {
        return placements;
    };
    let dx = area.col + area.cols - bounds.width;
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
    fn editor_returns_body_then_footer() {
        let state = state(None);
        let scene = editor(&state, WINDOW);
        assert_eq!(scene.len(), 2);
        assert_eq!(scene[0].0.rows, WINDOW.rows - FOOTER_ROWS);
        assert_eq!(scene[1].0.row, scene[0].0.row + scene[0].0.rows);
        assert!(scene[1]
            .1
            .iter()
            .any(|placement| matches!(placement.node, PlacementNode::Led { .. })));
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
}
