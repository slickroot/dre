use std::borrow::Cow;

use crate::view::{
    self, Area, Label, Placement, PlacementNode, Scene, ALL_SIDES, BORDER, BOX_HEIGHT,
};

use super::state::FlexState;

pub(crate) fn scene(state: &FlexState, window: Area) -> Scene<'_> {
    let text = state.text.as_str();
    let width = view::interior(text) + 2;
    let placements = vec![
        Placement {
            node: PlacementNode::Box {
                colour: None,
                fill: None,
                opacity: None,
                rounded: false,
                sides: ALL_SIDES,
                border: BORDER,
            },
            x: 0,
            y: 0,
            width,
            height: BOX_HEIGHT,
        },
        Placement {
            node: PlacementNode::Label(Label {
                text: Cow::Borrowed(text),
                colour: None,
                bold: false,
            }),
            x: view::label_centre(width, text),
            y: BOX_HEIGHT / 2,
            width: view::interior(text),
            height: 1,
        },
    ];
    vec![(window, view::centre(placements, window))]
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Area = Area {
        col: 0,
        row: 0,
        cols: 80,
        rows: 24,
    };

    fn with_text(text: &str) -> FlexState {
        FlexState {
            text: text.to_string(),
            ..FlexState::default()
        }
    }

    fn placements(state: &FlexState) -> Vec<Placement<'_>> {
        let [(area, placements)] = <[_; 1]>::try_from(scene(state, WINDOW)).unwrap();
        assert_eq!(area, WINDOW);
        placements
    }

    fn the_box<'a>(placements: &'a [Placement<'a>]) -> &'a Placement<'a> {
        placements
            .iter()
            .find(|placement| matches!(placement.node, PlacementNode::Box { .. }))
            .unwrap()
    }

    fn the_label<'a>(placements: &'a [Placement<'a>]) -> &'a Placement<'a> {
        placements
            .iter()
            .find(|placement| matches!(placement.node, PlacementNode::Label(_)))
            .unwrap()
    }

    #[test]
    fn an_empty_box_is_three_cells_wide() {
        let state = with_text("");
        let placements = placements(&state);
        let the_box = the_box(&placements);
        assert_eq!(the_box.width, 3);
        assert_eq!(the_box.height, BOX_HEIGHT);
    }

    #[test]
    fn the_label_sits_inside_the_box() {
        let state = with_text("");
        let placements = placements(&state);
        let the_box = the_box(&placements);
        let label = the_label(&placements);
        assert!(label.x > the_box.x);
        assert!(label.x + label.width < the_box.x + the_box.width);
        assert!(label.y > the_box.y);
        assert!(label.y + label.height < the_box.y + the_box.height);
    }

    #[test]
    fn only_the_box_and_its_label_are_placed() {
        let state = with_text("");
        let placements = placements(&state);
        let boxes = placements
            .iter()
            .filter(|placement| matches!(placement.node, PlacementNode::Box { .. }))
            .count();
        let labels = placements
            .iter()
            .filter(|placement| matches!(placement.node, PlacementNode::Label(_)))
            .count();
        assert_eq!((boxes, labels, placements.len()), (1, 1, 2));
    }

    #[test]
    fn the_box_grows_to_fit_the_text() {
        let state = with_text("Hello");
        let placements = placements(&state);
        assert_eq!(the_box(&placements).width, 7);
    }

    #[test]
    fn the_label_shows_the_text_inside_the_box() {
        let state = with_text("Hello");
        let placements = placements(&state);
        let the_box = the_box(&placements);
        let label = the_label(&placements);
        let PlacementNode::Label(Label { text, .. }) = &label.node else {
            unreachable!()
        };
        assert_eq!(text, &state.text);
        assert!(label.x > the_box.x);
        assert!(label.x + label.width < the_box.x + the_box.width);
    }
}
