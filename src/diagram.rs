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

impl Node {
    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) fn colour(&self) -> Option<u8> {
        self.colour
    }

    pub(crate) fn filled(&self) -> bool {
        self.filled
    }

    pub(crate) fn rounded(&self) -> bool {
        self.rounded
    }
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

#[allow(dead_code)]
pub(crate) enum Scope {
    Box,
    Siblings,
}

impl Document {
    pub(crate) fn tree(&self) -> &Tree<Node> {
        &self.root
    }

    #[allow(dead_code)]
    pub(crate) fn set_label(&mut self, path: &[usize], label: String) {
        self.root.value_mut(path).label = label;
    }

    #[allow(dead_code)]
    pub(crate) fn set_colour(&mut self, path: &[usize], colour: Option<u8>, scope: Scope) {
        self.set(path, scope, |node| node.colour = colour);
    }

    #[allow(dead_code)]
    pub(crate) fn set_fill(&mut self, path: &[usize], filled: bool, scope: Scope) {
        self.set(path, scope, |node| node.filled = filled);
    }

    #[allow(dead_code)]
    pub(crate) fn set_rounded(&mut self, path: &[usize], rounded: bool, scope: Scope) {
        self.set(path, scope, |node| node.rounded = rounded);
    }

    #[allow(dead_code)]
    pub(crate) fn insert(&mut self, parent: &[usize], subtree: &Tree<Node>) -> Vec<usize> {
        self.root.push(parent, subtree.clone())
    }

    #[allow(dead_code)]
    pub(crate) fn remove(&mut self, path: &[usize]) -> Tree<Node> {
        self.root.remove(path)
    }

    #[allow(dead_code)]
    fn set(&mut self, path: &[usize], scope: Scope, write: impl Fn(&mut Node)) {
        let targets: Vec<Vec<usize>> = match scope {
            Scope::Box => vec![path.to_vec()],
            Scope::Siblings => children(&self.root, parent_of(path)).collect(),
        };
        for target in targets {
            write(self.root.value_mut(&target));
        }
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
impl Node {
    pub(crate) fn with_colour(self, colour: Option<u8>) -> Node {
        Node { colour, ..self }
    }

    pub(crate) fn with_fill(self, filled: bool) -> Node {
        Node { filled, ..self }
    }

    pub(crate) fn with_rounded(self, rounded: bool) -> Node {
        Node { rounded, ..self }
    }
}

#[cfg(test)]
impl Document {
    pub(crate) fn with_boxes(boxes: Vec<Tree<Node>>) -> Document {
        Document {
            root: Tree::root(boxes),
        }
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

    fn row() -> Document {
        Document::with_boxes(vec![
            node_with_children("a", vec![node("c"), node("d")]),
            node("b"),
        ])
    }

    fn row_with_child_of_a(subtree: Tree<Node>) -> Document {
        Document::with_boxes(vec![
            node_with_children("a", vec![node("c"), node("d"), subtree]),
            node("b"),
        ])
    }

    #[test]
    fn set_label_writes_the_label_of_the_box() {
        let mut doc = row();
        doc.set_label(&[0, 1], "e".to_string());
        assert_eq!(doc.tree().value(&[0, 1]).label(), "e");
    }

    #[test]
    fn set_label_leaves_other_boxes_alone() {
        let mut doc = row();
        doc.set_label(&[0, 1], "e".to_string());
        assert_eq!(doc.tree().value(&[0, 0]).label(), "c");
    }

    #[test]
    fn set_colour_on_a_box_writes_only_that_box() {
        let mut doc = row();
        doc.set_colour(&[0], Some(3), Scope::Box);
        assert_eq!(doc.tree().value(&[0]).colour(), Some(3));
        assert_eq!(doc.tree().value(&[1]).colour(), None);
    }

    #[test]
    fn set_colour_on_siblings_writes_every_child_of_the_parent() {
        let mut doc = row();
        doc.set_colour(&[0, 0], Some(3), Scope::Siblings);
        assert_eq!(doc.tree().value(&[0, 0]).colour(), Some(3));
        assert_eq!(doc.tree().value(&[0, 1]).colour(), Some(3));
        assert_eq!(doc.tree().value(&[0]).colour(), None);
    }

    #[test]
    fn set_colour_writes_no_colour() {
        let mut doc = Document::with_boxes(vec![Tree::leaf(labelled("a").with_colour(Some(2)))]);
        doc.set_colour(&[0], None, Scope::Box);
        assert_eq!(doc.tree().value(&[0]).colour(), None);
    }

    #[test]
    fn set_colour_leaves_the_fill_alone() {
        let mut doc = Document::with_boxes(vec![Tree::leaf(
            labelled("a").with_colour(Some(2)).with_fill(true),
        )]);
        doc.set_colour(&[0], None, Scope::Box);
        assert!(doc.tree().value(&[0]).filled());
    }

    #[test]
    fn set_fill_on_a_box_writes_only_that_box() {
        let mut doc = row();
        doc.set_fill(&[1], true, Scope::Box);
        assert!(doc.tree().value(&[1]).filled());
        assert!(!doc.tree().value(&[0]).filled());
    }

    #[test]
    fn set_fill_writes_the_value_even_without_a_colour() {
        let mut doc = row();
        doc.set_fill(&[0], true, Scope::Box);
        assert!(doc.tree().value(&[0]).filled());
    }

    #[test]
    fn set_fill_on_siblings_writes_every_top_level_box() {
        let mut doc = row();
        doc.set_fill(&[1], true, Scope::Siblings);
        assert!(doc.tree().value(&[0]).filled());
        assert!(doc.tree().value(&[1]).filled());
        assert!(!doc.tree().value(&[0, 0]).filled());
    }

    #[test]
    fn set_fill_writes_false() {
        let mut doc = Document::with_boxes(vec![Tree::leaf(labelled("a").with_fill(true))]);
        doc.set_fill(&[0], false, Scope::Box);
        assert!(!doc.tree().value(&[0]).filled());
    }

    #[test]
    fn set_rounded_on_a_box_writes_only_that_box() {
        let mut doc = row();
        doc.set_rounded(&[0, 1], true, Scope::Box);
        assert!(doc.tree().value(&[0, 1]).rounded());
        assert!(!doc.tree().value(&[0, 0]).rounded());
    }

    #[test]
    fn set_rounded_on_siblings_writes_every_child_of_the_parent() {
        let mut doc = row();
        doc.set_rounded(&[0, 1], true, Scope::Siblings);
        assert!(doc.tree().value(&[0, 0]).rounded());
        assert!(doc.tree().value(&[0, 1]).rounded());
    }

    #[test]
    fn set_rounded_writes_false() {
        let mut doc = Document::with_boxes(vec![Tree::leaf(labelled("a").with_rounded(true))]);
        doc.set_rounded(&[0], false, Scope::Box);
        assert!(!doc.tree().value(&[0]).rounded());
    }

    #[test]
    fn insert_adds_the_subtree_as_the_last_child_and_returns_its_path() {
        let mut doc = row();
        let subtree = node_with_children("e", vec![node("f")]);
        let path = doc.insert(&[0], &subtree);
        assert_eq!(path, vec![0, 2]);
        assert_eq!(doc.tree(), row_with_child_of_a(subtree).tree());
    }

    #[test]
    fn insert_at_the_root_adds_a_top_level_box() {
        let mut doc = row();
        let path = doc.insert(&[], &node("e"));
        assert_eq!(path, vec![2]);
        assert_eq!(doc.tree().value(&path).label(), "e");
    }

    #[test]
    fn insert_into_an_empty_document_adds_the_first_box() {
        let mut doc = Document::default();
        let path = doc.insert(&[], &node("a"));
        assert_eq!(doc, Document::with_boxes(vec![node("a")]));
        assert_eq!(path, vec![0]);
    }

    #[test]
    fn remove_returns_the_detached_subtree() {
        let mut doc = row();
        assert_eq!(
            doc.remove(&[0]),
            node_with_children("a", vec![node("c"), node("d")])
        );
    }

    #[test]
    fn remove_takes_the_box_out_of_the_document() {
        let mut doc = row();
        doc.remove(&[0, 0]);
        assert_eq!(
            doc,
            Document::with_boxes(vec![node_with_children("a", vec![node("d")]), node("b")])
        );
    }
}
