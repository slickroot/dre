#[cfg(not(target_arch = "wasm32"))]
use crate::dre_format::{FileBox, FileDoc};

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

pub(crate) fn append(siblings: &mut Vec<Node>, node: Node) -> usize {
    siblings.push(node);
    siblings.len() - 1
}

pub(crate) fn remove(boxes: &mut Vec<Node>, path: &Path) -> Node {
    children_at(boxes, &path.ancestors).remove(path.index)
}

#[cfg(not(target_arch = "wasm32"))]
fn file_box(node: &Node) -> FileBox {
    FileBox {
        label: node.label.clone(),
        colour: node.colour,
        fill: if node.filled && node.colour.is_some() {
            node.colour
        } else {
            None
        },
        rounded: node.rounded,
        children: node.children.iter().map(file_box).collect(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn from_document(doc: &Document) -> FileDoc {
    FileDoc {
        boxes: doc.boxes.iter().map(file_box).collect(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn node_from_file_box(file_box: FileBox) -> Node {
    Node {
        label: file_box.label,
        colour: file_box.colour,
        filled: file_box.fill.is_some(),
        rounded: file_box.rounded,
        children: file_box
            .children
            .into_iter()
            .map(node_from_file_box)
            .collect(),
        hint: false,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn to_document(doc: FileDoc) -> Document {
    Document {
        boxes: doc.boxes.into_iter().map(node_from_file_box).collect(),
        selected: None,
    }
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
    fn remove_returns_the_node_with_its_descendants() {
        let mut boxes = vec![
            node_with_children("a", vec![node_with_children("b", vec![node("c")])]),
            node("d"),
        ];
        let removed = remove(&mut boxes, &path(&[], 0));
        assert_eq!(
            removed,
            node_with_children("a", vec![node_with_children("b", vec![node("c")])])
        );
        assert_eq!(boxes, vec![node("d")]);
    }

    #[test]
    fn remove_leaves_other_top_level_boxes_untouched() {
        let mut boxes = vec![
            node_with_children("a", vec![node("b"), node("c")]),
            node_with_children("d", vec![node("e")]),
        ];
        let removed = remove(&mut boxes, &path(&[0], 0));
        assert_eq!(removed, node("b"));
        assert_eq!(
            boxes,
            vec![
                node_with_children("a", vec![node("c")]),
                node_with_children("d", vec![node("e")]),
            ]
        );
    }

    #[test]
    fn saving_a_document_maps_labels_colours_fills_rounding_and_nesting_across() {
        let mut doc = Document::default();
        let child = Node {
            label: "Auth".to_string(),
            colour: Some(1),
            filled: true,
            ..Node::default()
        };
        doc.boxes = vec![
            Node {
                label: "API".to_string(),
                colour: Some(2),
                rounded: true,
                children: vec![child],
                ..Node::default()
            },
            Node {
                label: "Billing".to_string(),
                filled: true,
                ..Node::default()
            },
        ];
        let expected = FileDoc {
            boxes: vec![
                FileBox {
                    label: "API".to_string(),
                    colour: Some(2),
                    fill: None,
                    rounded: true,
                    children: vec![FileBox {
                        label: "Auth".to_string(),
                        colour: Some(1),
                        fill: Some(1),
                        rounded: false,
                        children: vec![],
                    }],
                },
                FileBox {
                    label: "Billing".to_string(),
                    colour: None,
                    fill: None,
                    rounded: false,
                    children: vec![],
                },
            ],
        };
        assert_eq!(from_document(&doc), expected);
    }

    #[test]
    fn opening_an_empty_file_doc_gives_no_boxes_and_nothing_selected() {
        let doc = to_document(FileDoc { boxes: vec![] });
        assert!(doc.boxes.is_empty());
        assert_eq!(doc.selected, None);
    }

    #[test]
    fn opening_a_file_doc_with_boxes_selects_nothing() {
        let doc = FileDoc {
            boxes: vec![
                FileBox {
                    label: "API".to_string(),
                    colour: None,
                    fill: None,
                    rounded: false,
                    children: vec![],
                },
                FileBox {
                    label: "Billing".to_string(),
                    colour: None,
                    fill: None,
                    rounded: false,
                    children: vec![],
                },
            ],
        };
        assert_eq!(to_document(doc).selected, None);
    }

    #[test]
    fn opening_a_file_doc_maps_labels_colours_fills_rounding_and_nesting_into_the_document() {
        let doc = FileDoc {
            boxes: vec![
                FileBox {
                    label: "API".to_string(),
                    colour: Some(2),
                    fill: None,
                    rounded: true,
                    children: vec![FileBox {
                        label: "Auth".to_string(),
                        colour: None,
                        fill: Some(3),
                        rounded: false,
                        children: vec![],
                    }],
                },
                FileBox {
                    label: "Billing".to_string(),
                    colour: Some(4),
                    fill: Some(1),
                    rounded: false,
                    children: vec![],
                },
            ],
        };
        let child = Node {
            label: "Auth".to_string(),
            filled: true,
            ..Node::default()
        };
        let expected = vec![
            Node {
                label: "API".to_string(),
                colour: Some(2),
                rounded: true,
                children: vec![child],
                ..Node::default()
            },
            Node {
                label: "Billing".to_string(),
                colour: Some(4),
                filled: true,
                ..Node::default()
            },
        ];
        assert_eq!(to_document(doc).boxes, expected);
    }

    #[test]
    fn from_document_writes_fill_equal_to_border_colour_index_when_filled() {
        let doc = Document {
            boxes: vec![
                Node {
                    label: "A".to_string(),
                    colour: Some(2),
                    filled: true,
                    ..Node::default()
                },
                Node {
                    label: "B".to_string(),
                    colour: Some(2),
                    filled: false,
                    ..Node::default()
                },
            ],
            ..Document::default()
        };
        let fd = from_document(&doc);
        assert_eq!(fd.boxes[0].fill, Some(2));
        assert_eq!(fd.boxes[1].fill, None);
    }

    #[test]
    fn from_document_omits_fill_when_filled_but_colourless() {
        let doc = Document {
            boxes: vec![Node {
                label: "A".to_string(),
                colour: None,
                filled: true,
                ..Node::default()
            }],
            ..Document::default()
        };
        let fd = from_document(&doc);
        assert_eq!(fd.boxes[0].fill, None);
    }

    #[test]
    fn filled_colourless_box_round_trips_as_unfilled() {
        let doc = Document {
            boxes: vec![Node {
                label: "A".to_string(),
                colour: None,
                filled: true,
                ..Node::default()
            }],
            ..Document::default()
        };
        let fd = from_document(&doc);
        let reloaded = to_document(fd);
        assert!(!reloaded.boxes[0].filled);
        assert_eq!(reloaded.boxes[0].label, "A");
        assert_eq!(reloaded.boxes[0].colour, None);
    }

    #[test]
    fn legacy_fill_zero_means_filled() {
        let fd = FileDoc {
            boxes: vec![FileBox {
                label: "A".to_string(),
                colour: None,
                fill: Some(0),
                rounded: false,
                children: vec![],
            }],
        };
        let doc = to_document(fd);
        assert!(doc.boxes[0].filled);
    }
}
