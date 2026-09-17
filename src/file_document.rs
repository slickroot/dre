use crate::dre_format::{FileBox, FileDoc};
use crate::state::{Document, Node, Path, State};

fn file_box(node: &Node) -> FileBox {
    FileBox {
        label: node.label.clone(),
        colour: node.colour,
        fill: node.fill,
        rounded: node.rounded,
        children: node.children.iter().map(file_box).collect(),
    }
}

pub(crate) fn from_state(state: &State) -> FileDoc {
    FileDoc { boxes: state.doc.boxes.iter().map(file_box).collect() }
}

fn node(file_box: FileBox) -> Node {
    Node {
        label: file_box.label,
        colour: file_box.colour,
        fill: file_box.fill,
        rounded: file_box.rounded,
        children: file_box.children.into_iter().map(node).collect(),
    }
}

pub(crate) fn to_state(doc: FileDoc) -> State {
    let boxes: Vec<Node> = doc.boxes.into_iter().map(node).collect();
    let selected = if boxes.is_empty() { None } else { Some(Path { ancestors: vec![], index: 0 }) };
    let mut state = State::default();
    state.doc = Document { boxes, selected };
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Mode;

    #[test]
    fn saving_a_state_maps_labels_colours_fills_rounding_and_nesting_across() {
        let mut state = State::default();
        let child = Node { label: "Auth".to_string(), fill: Some(3), ..Node::default() };
        state.doc.boxes = vec![
            Node { label: "API".to_string(), colour: Some(2), rounded: true, children: vec![child], ..Node::default() },
            Node { label: "Billing".to_string(), ..Node::default() },
        ];
        let expected = FileDoc {
            boxes: vec![
                FileBox {
                    label: "API".to_string(),
                    colour: Some(2),
                    fill: None,
                    rounded: true,
                    children: vec![FileBox { label: "Auth".to_string(), colour: None, fill: Some(3), rounded: false, children: vec![] }],
                },
                FileBox { label: "Billing".to_string(), colour: None, fill: None, rounded: false, children: vec![] },
            ],
        };
        assert_eq!(from_state(&state), expected);
    }

    #[test]
    fn opening_an_empty_file_doc_gives_no_boxes_and_nothing_selected() {
        let state = to_state(FileDoc { boxes: vec![] });
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
    }

    #[test]
    fn opening_a_file_doc_with_boxes_selects_the_first_top_level_box() {
        let doc = FileDoc {
            boxes: vec![
                FileBox { label: "API".to_string(), colour: None, fill: None, rounded: false, children: vec![] },
                FileBox { label: "Billing".to_string(), colour: None, fill: None, rounded: false, children: vec![] },
            ],
        };
        assert_eq!(to_state(doc).doc.selected, Some(Path { ancestors: vec![], index: 0 }));
    }

    #[test]
    fn opening_a_file_doc_maps_labels_colours_fills_rounding_and_nesting_into_state() {
        let doc = FileDoc {
            boxes: vec![
                FileBox {
                    label: "API".to_string(),
                    colour: Some(2),
                    fill: None,
                    rounded: true,
                    children: vec![FileBox { label: "Auth".to_string(), colour: None, fill: Some(3), rounded: false, children: vec![] }],
                },
                FileBox { label: "Billing".to_string(), colour: Some(4), fill: Some(1), rounded: false, children: vec![] },
            ],
        };
        let child = Node { label: "Auth".to_string(), fill: Some(3), ..Node::default() };
        let expected = vec![
            Node { label: "API".to_string(), colour: Some(2), rounded: true, children: vec![child], ..Node::default() },
            Node { label: "Billing".to_string(), colour: Some(4), fill: Some(1), ..Node::default() },
        ];
        assert_eq!(to_state(doc).doc.boxes, expected);
    }

    #[test]
    fn opening_a_file_doc_keeps_command_mode_and_no_save_target() {
        let state = to_state(FileDoc { boxes: vec![] });
        assert_eq!(state.mode, Mode::Command);
        assert_eq!(state.save_to, None);
        assert!(state.running);
    }
}
