#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Node {
    pub(crate) label: String,
    pub(crate) colour: Option<u8>,
    pub(crate) filled: bool,
    pub(crate) rounded: bool,
    pub(crate) children: Vec<Node>,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            label: String::new(),
            colour: None,
            filled: false,
            rounded: false,
            children: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Path {
    pub(crate) ancestors: Vec<usize>,
    pub(crate) index: usize,
}

pub(crate) fn children_at<'a>(boxes: &'a mut Vec<Node>, ancestors: &[usize]) -> &'a mut Vec<Node> {
    let mut children = boxes;
    for &index in ancestors {
        children = &mut children[index].children;
    }
    children
}

pub(crate) fn at<'a>(boxes: &'a mut Vec<Node>, path: &Path) -> &'a mut Node {
    &mut children_at(boxes, &path.ancestors)[path.index]
}

const PALETTE: [(u8, u8, u8); 5] = [
    (255, 190, 11),
    (251, 86, 7),
    (255, 0, 110),
    (131, 56, 236),
    (58, 134, 255),
];

pub(crate) fn palette(index: u8) -> Option<(u8, u8, u8)> {
    PALETTE.get(index as usize).copied()
}

pub(crate) fn append(siblings: &mut Vec<Node>, node: Node) -> usize {
    siblings.push(node);
    siblings.len() - 1
}

#[cfg(test)]
pub(crate) fn node(label: &str) -> Node {
    Node { label: label.to_string(), ..Default::default() }
}

#[cfg(test)]
pub(crate) fn node_with_children(label: &str, children: Vec<Node>) -> Node {
    Node { label: label.to_string(), children, ..Default::default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_has_a_colour_at_index_zero() {
        assert!(palette(0).is_some());
    }

    #[test]
    fn palette_has_no_colour_past_its_last_index() {
        let first_missing = (0..=u8::MAX).find(|&i| palette(i).is_none()).unwrap();
        assert!(first_missing > 0);
        assert_eq!(palette(first_missing), None);
        assert!((first_missing..=u8::MAX).all(|i| palette(i).is_none()));
    }

    #[test]
    fn boxes_are_equal() {
        assert_eq!(Node::default(), Node::default());
    }

    #[test]
    fn boxes_default_to_the_plain_colour() {
        assert_eq!(Node::default().colour, None);
    }

    #[test]
    fn boxes_default_to_the_plain_fill() {
        assert_eq!(Node::default().filled, false);
    }

    #[test]
    fn boxes_default_to_an_empty_label() {
        assert_eq!(Node::default(), node(""));
    }

    #[test]
    fn boxes_with_different_labels_are_not_equal() {
        assert_ne!(node("a"), node("b"));
    }

    #[test]
    fn boxes_default_to_no_children() {
        assert_eq!(Node::default().children, Vec::<Node>::new());
    }

    #[test]
    fn boxes_with_different_children_are_not_equal() {
        assert_ne!(
            node_with_children("", vec![node("a")]),
            node_with_children("", vec![node("b")])
        );
    }

    #[test]
    fn new_box_starts_with_square_corners() {
        assert_eq!(node("a").rounded, false);
    }

    #[test]
    fn children_at_no_ancestors_returns_the_top_level_boxes() {
        let mut boxes = vec![node("a"), node("b")];
        assert_eq!(*children_at(&mut boxes, &[]), vec![node("a"), node("b")]);
    }

    #[test]
    fn children_at_ancestors_returns_the_addressed_nodes_children() {
        let mut boxes = vec![node_with_children(
            "a",
            vec![node_with_children("b", vec![node("c"), node("d")])],
        )];
        assert_eq!(*children_at(&mut boxes, &[0]), vec![node_with_children("b", vec![node("c"), node("d")])]);
        assert_eq!(*children_at(&mut boxes, &[0, 0]), vec![node("c"), node("d")]);
    }

    #[test]
    fn at_a_single_index_returns_the_top_level_box() {
        let mut boxes = vec![node("a"), node("b")];
        assert_eq!(*at(&mut boxes, &Path { ancestors: vec![], index: 1 }), node("b"));
    }

    #[test]
    fn at_a_longer_path_walks_into_children() {
        let mut boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        assert_eq!(*at(&mut boxes, &Path { ancestors: vec![0], index: 1 }), node("d"));
    }

    #[test]
    fn at_a_deep_path_walks_multiple_levels() {
        let mut boxes = vec![node_with_children(
            "a",
            vec![node_with_children("b", vec![node("c")])],
        )];
        assert_eq!(*at(&mut boxes, &Path { ancestors: vec![0, 0], index: 0 }), node("c"));
    }

    #[test]
    fn append_on_an_empty_list_pushes_the_node_and_returns_its_index() {
        let mut boxes = Vec::new();
        let index = append(&mut boxes, node("a"));
        assert_eq!(boxes, vec![node("a")]);
        assert_eq!(index, 0);
    }

    #[test]
    fn append_pushes_after_existing_nodes_and_returns_its_index() {
        let mut boxes = vec![node("a")];
        let index = append(&mut boxes, node("b"));
        assert_eq!(boxes, vec![node("a"), node("b")]);
        assert_eq!(index, 1);
    }

    #[test]
    fn append_on_a_nodes_children_appends_a_child() {
        let mut boxes = vec![node_with_children("a", vec![node("c")])];
        let index = append(&mut boxes[0].children, node("d"));
        assert_eq!(boxes, vec![node_with_children("a", vec![node("c"), node("d")])]);
        assert_eq!(index, 1);
    }
}
