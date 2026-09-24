#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct Node {
    pub(crate) label: String,
    pub(crate) colour: Option<u8>,
    pub(crate) filled: bool,
    pub(crate) rounded: bool,
    pub(crate) children: Vec<Node>,
    pub(crate) hint: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Path {
    pub(crate) ancestors: Vec<usize>,
    pub(crate) index: usize,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Document {
    pub(crate) boxes: Vec<Node>,
    pub(crate) selected: Option<Path>,
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

const PALETTE: [(&str, (u8, u8, u8)); 7] = [
    ("lime", (0xC6, 0xFF, 0x00)),
    ("mint", (0x39, 0xFF, 0xB0)),
    ("violet", (0xB3, 0x88, 0xFF)),
    ("pink", (0xFF, 0x3D, 0xF5)),
    ("amber", (0xFF, 0xB0, 0x20)),
    ("foreground", (0xE8, 0xEA, 0xED)),
    ("background", (0x0A, 0x0B, 0x0D)),
];

pub(crate) const FOREGROUND: u8 = 5;
pub(crate) const BACKGROUND: u8 = 6;

pub(crate) fn palette(index: u8) -> Option<(u8, u8, u8)> {
    PALETTE.get(index as usize).map(|&(_, rgb)| rgb)
}

pub(crate) fn append(siblings: &mut Vec<Node>, node: Node) -> usize {
    siblings.push(node);
    siblings.len() - 1
}

pub(crate) fn remove(boxes: &mut Vec<Node>, path: &Path) -> Option<Path> {
    let siblings = children_at(boxes, &path.ancestors);
    siblings.remove(path.index);
    if path.index < siblings.len() {
        return Some(path.clone());
    }
    if path.index > 0 {
        return Some(Path {
            ancestors: path.ancestors.clone(),
            index: path.index - 1,
        });
    }
    let (&index, ancestors) = path.ancestors.split_last()?;
    Some(Path {
        ancestors: ancestors.to_vec(),
        index,
    })
}

#[cfg(test)]
pub(crate) fn node(label: &str) -> Node {
    Node {
        label: label.to_string(),
        ..Default::default()
    }
}

#[cfg(test)]
pub(crate) fn node_with_children(label: &str, children: Vec<Node>) -> Node {
    Node {
        label: label.to_string(),
        children,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_has_a_colour_at_index_zero() {
        assert!(palette(0).is_some());
    }

    #[test]
    fn palette_has_the_foreground_colour_at_its_index() {
        assert_eq!(palette(FOREGROUND), Some((232, 234, 237)));
    }

    #[test]
    fn palette_has_the_background_colour_at_its_index() {
        assert_eq!(palette(BACKGROUND), Some((10, 11, 13)));
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
        assert!(!Node::default().filled);
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
        assert!(!node("a").rounded);
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
        assert_eq!(
            *children_at(&mut boxes, &[0]),
            vec![node_with_children("b", vec![node("c"), node("d")])]
        );
        assert_eq!(
            *children_at(&mut boxes, &[0, 0]),
            vec![node("c"), node("d")]
        );
    }

    #[test]
    fn at_a_single_index_returns_the_top_level_box() {
        let mut boxes = vec![node("a"), node("b")];
        assert_eq!(
            *at(
                &mut boxes,
                &Path {
                    ancestors: vec![],
                    index: 1
                }
            ),
            node("b")
        );
    }

    #[test]
    fn at_a_longer_path_walks_into_children() {
        let mut boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        assert_eq!(
            *at(
                &mut boxes,
                &Path {
                    ancestors: vec![0],
                    index: 1
                }
            ),
            node("d")
        );
    }

    #[test]
    fn at_a_deep_path_walks_multiple_levels() {
        let mut boxes = vec![node_with_children(
            "a",
            vec![node_with_children("b", vec![node("c")])],
        )];
        assert_eq!(
            *at(
                &mut boxes,
                &Path {
                    ancestors: vec![0, 0],
                    index: 0
                }
            ),
            node("c")
        );
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
        assert_eq!(
            boxes,
            vec![node_with_children("a", vec![node("c"), node("d")])]
        );
        assert_eq!(index, 1);
    }

    fn path(ancestors: &[usize], index: usize) -> Path {
        Path {
            ancestors: ancestors.to_vec(),
            index,
        }
    }

    #[test]
    fn remove_selects_the_next_sibling_at_the_same_path() {
        let mut boxes = vec![node("a"), node("b"), node("c")];
        let selected = remove(&mut boxes, &path(&[], 1));
        assert_eq!(boxes, vec![node("a"), node("c")]);
        assert_eq!(selected, Some(path(&[], 1)));
    }

    #[test]
    fn remove_the_last_sibling_selects_the_previous_sibling() {
        let mut boxes = vec![node("a"), node("b"), node("c")];
        let selected = remove(&mut boxes, &path(&[], 2));
        assert_eq!(boxes, vec![node("a"), node("b")]);
        assert_eq!(selected, Some(path(&[], 1)));
    }

    #[test]
    fn remove_an_only_child_selects_the_parent() {
        let mut boxes = vec![
            node("a"),
            node_with_children("b", vec![node_with_children("c", vec![node("d")])]),
        ];
        let selected = remove(&mut boxes, &path(&[1, 0], 0));
        assert_eq!(
            boxes,
            vec![node("a"), node_with_children("b", vec![node("c")])]
        );
        assert_eq!(selected, Some(path(&[1], 0)));
    }

    #[test]
    fn remove_the_only_top_level_box_selects_nothing() {
        let mut boxes = vec![node("a")];
        let selected = remove(&mut boxes, &path(&[], 0));
        assert_eq!(boxes, vec![]);
        assert_eq!(selected, None);
    }

    #[test]
    fn remove_drops_the_descendants_with_the_box() {
        let mut boxes = vec![
            node_with_children("a", vec![node_with_children("b", vec![node("c")])]),
            node("d"),
        ];
        remove(&mut boxes, &path(&[], 0));
        assert_eq!(boxes, vec![node("d")]);
    }

    #[test]
    fn remove_leaves_other_top_level_boxes_untouched() {
        let mut boxes = vec![
            node_with_children("a", vec![node("b"), node("c")]),
            node_with_children("d", vec![node("e")]),
        ];
        remove(&mut boxes, &path(&[0], 0));
        assert_eq!(
            boxes,
            vec![
                node_with_children("a", vec![node("c")]),
                node_with_children("d", vec![node("e")]),
            ]
        );
    }
}
