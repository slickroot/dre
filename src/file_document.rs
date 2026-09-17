use crate::dre_format::{FileBox, FileDoc};
use crate::state::{Node, State};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
