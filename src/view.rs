use crate::composer;
use crate::layout::{self, with_caret, with_glow, FOOTER_ROWS};
use crate::state::{Mode, State};

pub use crate::composer::Area;
pub use crate::layout::Placement;
pub use crate::layout::PlacementNode;

pub type Scene<'a> = Vec<(Area, Vec<Placement<'a>>)>;

pub fn editor(state: &State, window: Area) -> Scene<'_> {
    let [body_area, foot] = composer::stack([None, Some(FOOTER_ROWS)], window);
    let footer = align_right(layout::footer(&state.footer()), foot);
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
                with_caret(layout::diagram(state.doc().tree(), editing), editing_caret),
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
    use crate::layout::{PlacementNode, FOOTER_ROWS};
    use crate::state::{new_state, Mode};

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
