use std::borrow::Cow;

use crate::composer;
use crate::layout::tree;
use crate::palette::FOREGROUND;
use crate::render::FOOTER_FILL_OPACITY;
use crate::state::{FooterView, Mode, State};

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
    pub path: Vec<usize>,
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

pub(crate) fn footer(view: &FooterView) -> Vec<Placement<'static>> {
    let text = view.text.as_str();
    let text_width = text.chars().count() as i64;
    let inset_box_width = interior(text) + SIDE_PADDING * 2;
    let box_width = inset_box_width + LED_WIDTH + LED_LABEL_GAP;
    let corner_box = PlacementNode::Box {
        colour: None,
        fill: Some(FOREGROUND),
        opacity: Some(FOOTER_FILL_OPACITY),
        rounded: false,
        sides: if view.bordered { ALL_SIDES } else { NO_SIDES },
        border: 1,
    };
    let led = PlacementNode::Led {
        colour: view.led_colour,
        lit: view.lit,
    };
    let label = PlacementNode::Label(Label {
        text: Cow::Owned(text.to_string()),
        path: vec![],
    });
    let led_x = label_centre(inset_box_width, text);
    let label_x = led_x + LED_WIDTH + LED_LABEL_GAP;
    let placements = vec![
        Placement {
            node: corner_box,
            x: 0,
            y: 0,
            width: box_width,
            height: BOX_HEIGHT,
        },
        Placement {
            node: led,
            x: led_x,
            y: BOX_HEIGHT / 2,
            width: LED_WIDTH,
            height: 1,
        },
        Placement {
            node: label,
            x: label_x,
            y: BOX_HEIGHT / 2,
            width: text_width,
            height: 1,
        },
    ];
    with_cursor(placements, view.cursor.map(|_| Vec::new()), view.cursor)
}

pub(crate) fn with_glow<'a>(
    placements: Vec<Placement<'a>>,
    selected: Option<&[usize]>,
) -> Vec<Placement<'a>> {
    let Some(selected) = selected else {
        return placements;
    };
    let Some(label) = placements
        .iter()
        .find(|p| matches!(&p.node, PlacementNode::Label(label) if label.path == selected))
    else {
        return placements;
    };
    let Some(glow) = placements
        .iter()
        .find_map(|placement| match &placement.node {
            PlacementNode::Box {
                colour, rounded, ..
            } if placement.x <= label.x
                && label.x < placement.x + placement.width
                && placement.y <= label.y
                && label.y < placement.y + placement.height =>
            {
                Some(Placement {
                    node: PlacementNode::Glow {
                        colour: *colour,
                        rounded: *rounded,
                    },
                    x: placement.x - GLOW_MARGIN,
                    y: placement.y - GLOW_MARGIN,
                    width: placement.width + 2 * GLOW_MARGIN,
                    height: placement.height + 2 * GLOW_MARGIN,
                })
            }
            _ => None,
        })
    else {
        return placements;
    };
    let mut result = Vec::with_capacity(placements.len() + 1);
    result.push(glow);
    result.extend(placements);
    result
}

pub(crate) fn with_cursor<'a>(
    placements: Vec<Placement<'a>>,
    path: Option<Vec<usize>>,
    cursor: Option<usize>,
) -> Vec<Placement<'a>> {
    let (Some(path), Some(index)) = (path, cursor) else {
        return placements;
    };
    for placement in &placements {
        if let PlacementNode::Label(label) = &placement.node {
            if label.path == path {
                let mut result = placements.clone();
                result.push(Placement {
                    node: PlacementNode::Cursor(Cursor),
                    x: placement.x + index as i64,
                    y: placement.y,
                    width: 1,
                    height: 1,
                });
                return result;
            }
        }
    }
    placements
}

pub(crate) fn with_caret<'a>(
    placements: Vec<Placement<'a>>,
    editing: Option<(Vec<usize>, usize)>,
) -> Vec<Placement<'a>> {
    let Some((path, index)) = editing else {
        return placements;
    };
    for placement in &placements {
        if let PlacementNode::Label(label) = &placement.node {
            if label.path == path {
                let mut result = placements.clone();
                result.push(Placement {
                    node: PlacementNode::Caret(Caret),
                    x: placement.x + index as i64,
                    y: placement.y,
                    width: 1,
                    height: 1,
                });
                return result;
            }
        }
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
    let editing_caret: Option<(Vec<usize>, usize)> = match state.mode() {
        Mode::Insert { cursor } => state.selected().map(|path| (path.to_vec(), *cursor)),
        _ => None,
    };
    let editing = editing_caret.as_ref().map(|(path, _)| path.as_slice());
    vec![(
        area,
        with_glow(
            centre(
                with_caret(tree::diagram(state.doc().tree(), editing), editing_caret),
                area,
            ),
            state.selected(),
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
