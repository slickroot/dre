use crate::dre_format::v0::{FileBox, FileDoc};
use crate::state::{Document, Node, State, PLAIN};

fn node_to_file_box(node: &Node) -> FileBox {
    FileBox {
        label: node.label.clone(),
        colour: (node.colour != PLAIN).then_some(node.colour as u8),
        fill: (node.fill != PLAIN).then_some(node.fill as u8),
        rounded: node.rounded,
        children: node.children.iter().map(node_to_file_box).collect(),
    }
}

fn file_box_to_node(file_box: FileBox) -> Node {
    Node {
        label: file_box.label,
        colour: file_box.colour.map_or(PLAIN, |n| n as i64),
        fill: file_box.fill.map_or(PLAIN, |n| n as i64),
        rounded: file_box.rounded,
        children: file_box.children.into_iter().map(file_box_to_node).collect(),
    }
}

pub(crate) fn to_state(doc: FileDoc) -> State {
    let boxes: Vec<Node> = doc.boxes.into_iter().map(file_box_to_node).collect();
    let selected = if boxes.is_empty() { vec![] } else { vec![0] };
    State { doc: Document { boxes, selected }, ..State::default() }
}

pub(crate) fn from_state(state: &State) -> FileDoc {
    FileDoc { boxes: state.doc.boxes.iter().map(node_to_file_box).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_file_doc_gives_no_boxes_and_no_selection() {
        let state = to_state(FileDoc { boxes: vec![] });
        assert_eq!(state.doc.boxes, vec![]);
        assert_eq!(state.doc.selected, Vec::<usize>::new());
    }

    #[test]
    fn a_non_empty_file_doc_selects_the_first_box() {
        let state = to_state(FileDoc {
            boxes: vec![FileBox {
                label: "a".to_string(),
                colour: None,
                fill: None,
                rounded: false,
                children: vec![],
            }],
        });
        assert_eq!(state.doc.selected, vec![0]);
    }

    #[test]
    fn colour_and_fill_none_map_to_plain() {
        let state = to_state(FileDoc {
            boxes: vec![FileBox {
                label: "a".to_string(),
                colour: None,
                fill: None,
                rounded: false,
                children: vec![],
            }],
        });
        assert_eq!(state.doc.boxes[0].colour, PLAIN);
        assert_eq!(state.doc.boxes[0].fill, PLAIN);
    }

    #[test]
    fn colour_and_fill_some_map_to_the_value() {
        let state = to_state(FileDoc {
            boxes: vec![FileBox {
                label: "a".to_string(),
                colour: Some(3),
                fill: Some(2),
                rounded: false,
                children: vec![],
            }],
        });
        assert_eq!(state.doc.boxes[0].colour, 3);
        assert_eq!(state.doc.boxes[0].fill, 2);
    }

    #[test]
    fn from_state_maps_plain_back_to_none() {
        let state = to_state(FileDoc {
            boxes: vec![FileBox {
                label: "a".to_string(),
                colour: None,
                fill: None,
                rounded: false,
                children: vec![],
            }],
        });
        let doc = from_state(&state);
        assert_eq!(doc.boxes[0].colour, None);
        assert_eq!(doc.boxes[0].fill, None);
    }

    #[test]
    fn from_state_maps_values_back_to_some() {
        let state = to_state(FileDoc {
            boxes: vec![FileBox {
                label: "a".to_string(),
                colour: Some(3),
                fill: Some(2),
                rounded: false,
                children: vec![],
            }],
        });
        let doc = from_state(&state);
        assert_eq!(doc.boxes[0].colour, Some(3));
        assert_eq!(doc.boxes[0].fill, Some(2));
    }

    #[test]
    fn label_nesting_and_rounded_survive_the_round_trip() {
        let original = FileDoc {
            boxes: vec![FileBox {
                label: "parent".to_string(),
                colour: None,
                fill: None,
                rounded: true,
                children: vec![FileBox {
                    label: "child".to_string(),
                    colour: Some(1),
                    fill: None,
                    rounded: false,
                    children: vec![],
                }],
            }],
        };
        let state = to_state(original.clone());
        let round_tripped = from_state(&state);
        assert_eq!(round_tripped, original);
    }
}
