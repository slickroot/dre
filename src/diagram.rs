#[cfg(not(target_arch = "wasm32"))]
use crate::dre_format::{FileBox, FileDoc};

use types::Tree;

#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct Node {
    pub(crate) label: String,
    pub(crate) colour: Option<u8>,
    pub(crate) filled: bool,
    pub(crate) rounded: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub(crate) root: Tree<Node>,
}

impl Default for Document {
    fn default() -> Self {
        Document {
            root: Tree::root(Vec::new()),
        }
    }
}

impl Document {
    pub(crate) fn tree(&self) -> &Tree<Node> {
        &self.root
    }
}

pub(crate) fn parent_of(path: &[usize]) -> &[usize] {
    &path[..path.len() - 1]
}

pub(crate) fn children<'a, T>(
    tree: &'a Tree<T>,
    parent: &'a [usize],
) -> impl Iterator<Item = Vec<usize>> + 'a {
    (0..)
        .map(move |index| [parent, &[index]].concat())
        .take_while(|path| tree.contains(path))
}

#[cfg(not(target_arch = "wasm32"))]
fn file_box(tree: &Tree<Node>, path: &[usize]) -> FileBox {
    let node = tree.value(path);
    FileBox {
        label: node.label.clone(),
        colour: node.colour,
        fill: if node.filled && node.colour.is_some() {
            node.colour
        } else {
            None
        },
        rounded: node.rounded,
        children: children(tree, path)
            .map(|child| file_box(tree, &child))
            .collect(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn from_document(doc: &Document) -> FileDoc {
    FileDoc {
        boxes: children(doc.tree(), &[])
            .map(|path| file_box(doc.tree(), &path))
            .collect(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn node_from_file_box(file_box: FileBox) -> Tree<Node> {
    Tree::new(
        Node {
            label: file_box.label,
            colour: file_box.colour,
            filled: file_box.fill.is_some(),
            rounded: file_box.rounded,
        },
        file_box
            .children
            .into_iter()
            .map(node_from_file_box)
            .collect(),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn to_document(doc: FileDoc) -> Document {
    Document {
        root: Tree::root(doc.boxes.into_iter().map(node_from_file_box).collect()),
    }
}

#[cfg(test)]
pub(crate) fn labelled(label: &str) -> Node {
    Node {
        label: label.to_string(),
        ..Default::default()
    }
}

#[cfg(test)]
pub(crate) fn node(label: &str) -> Tree<Node> {
    Tree::leaf(labelled(label))
}

#[cfg(test)]
pub(crate) fn node_with_children(label: &str, children: Vec<Tree<Node>>) -> Tree<Node> {
    Tree::new(labelled(label), children)
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
        assert_eq!(Tree::leaf(Node::default()), node(""));
    }

    #[test]
    fn boxes_with_different_labels_are_not_equal() {
        assert_ne!(node("a"), node("b"));
    }

    #[test]
    fn boxes_default_to_no_children() {
        let tree = Tree::root(vec![node("a")]);
        assert_eq!(children(&tree, &[0]).count(), 0);
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
        assert!(!Tree::root(vec![node("a")]).value(&[0]).rounded);
    }

    #[test]
    fn a_default_document_has_no_boxes() {
        assert_eq!(Document::default().tree().walk().count(), 0);
    }

    #[test]
    fn children_of_the_root_are_the_top_level_boxes() {
        let tree = Tree::root(vec![node_with_children("a", vec![node("c")]), node("b")]);
        assert_eq!(
            children(&tree, &[]).collect::<Vec<_>>(),
            vec![vec![0], vec![1]]
        );
    }

    #[test]
    fn children_of_a_box_are_its_child_paths_in_order() {
        let tree = Tree::root(vec![node_with_children(
            "a",
            vec![node_with_children("b", vec![node("d")]), node("c")],
        )]);
        assert_eq!(
            children(&tree, &[0]).collect::<Vec<_>>(),
            vec![vec![0, 0], vec![0, 1]]
        );
    }

    #[test]
    fn saving_a_document_maps_labels_colours_fills_rounding_and_nesting_across() {
        let child = Tree::leaf(Node {
            label: "Auth".to_string(),
            colour: Some(1),
            filled: true,
            ..Node::default()
        });
        let doc = Document {
            root: Tree::root(vec![
                Tree::new(
                    Node {
                        label: "API".to_string(),
                        colour: Some(2),
                        rounded: true,
                        ..Node::default()
                    },
                    vec![child],
                ),
                Tree::leaf(Node {
                    label: "Billing".to_string(),
                    filled: true,
                    ..Node::default()
                }),
            ]),
        };
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
    fn opening_an_empty_file_doc_gives_no_boxes() {
        let doc = to_document(FileDoc { boxes: vec![] });
        assert_eq!(doc, Document::default());
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
        let child = Tree::leaf(Node {
            label: "Auth".to_string(),
            filled: true,
            ..Node::default()
        });
        let expected = Tree::root(vec![
            Tree::new(
                Node {
                    label: "API".to_string(),
                    colour: Some(2),
                    rounded: true,
                    ..Node::default()
                },
                vec![child],
            ),
            Tree::leaf(Node {
                label: "Billing".to_string(),
                colour: Some(4),
                filled: true,
                ..Node::default()
            }),
        ]);
        assert_eq!(*to_document(doc).tree(), expected);
    }

    #[test]
    fn from_document_writes_fill_equal_to_border_colour_index_when_filled() {
        let doc = Document {
            root: Tree::root(vec![
                Tree::leaf(Node {
                    label: "A".to_string(),
                    colour: Some(2),
                    filled: true,
                    ..Node::default()
                }),
                Tree::leaf(Node {
                    label: "B".to_string(),
                    colour: Some(2),
                    filled: false,
                    ..Node::default()
                }),
            ]),
        };
        let fd = from_document(&doc);
        assert_eq!(fd.boxes[0].fill, Some(2));
        assert_eq!(fd.boxes[1].fill, None);
    }

    #[test]
    fn from_document_omits_fill_when_filled_but_colourless() {
        let doc = Document {
            root: Tree::root(vec![Tree::leaf(Node {
                label: "A".to_string(),
                colour: None,
                filled: true,
                ..Node::default()
            })]),
        };
        let fd = from_document(&doc);
        assert_eq!(fd.boxes[0].fill, None);
    }

    #[test]
    fn filled_colourless_box_round_trips_as_unfilled() {
        let doc = Document {
            root: Tree::root(vec![Tree::leaf(Node {
                label: "A".to_string(),
                colour: None,
                filled: true,
                ..Node::default()
            })]),
        };
        let fd = from_document(&doc);
        let reloaded = to_document(fd);
        let reloaded = reloaded.tree().value(&[0]);
        assert!(!reloaded.filled);
        assert_eq!(reloaded.label, "A");
        assert_eq!(reloaded.colour, None);
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
        assert!(doc.tree().value(&[0]).filled);
    }
}
