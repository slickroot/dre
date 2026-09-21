use crate::dre_format::{FileBox, FileDoc};
use crate::diagram::{Document, Node};

fn file_box(node: &Node) -> FileBox {
    FileBox {
        label: node.label.clone(),
        colour: node.colour,
        fill: if node.filled && node.colour.is_some() { node.colour } else { None },
        rounded: node.rounded,
        children: node.children.iter().map(file_box).collect(),
    }
}

pub(crate) fn from_document(doc: &Document) -> FileDoc {
    FileDoc { boxes: doc.boxes.iter().map(file_box).collect() }
}

fn node(file_box: FileBox) -> Node {
    Node {
        label: file_box.label,
        colour: file_box.colour,
        filled: file_box.fill.is_some(),
        rounded: file_box.rounded,
        children: file_box.children.into_iter().map(node).collect(),
    }
}

pub(crate) fn to_document(doc: FileDoc) -> Document {
    Document { boxes: doc.boxes.into_iter().map(node).collect(), selected: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saving_a_document_maps_labels_colours_fills_rounding_and_nesting_across() {
        let mut doc = Document::default();
        let child = Node { label: "Auth".to_string(), colour: Some(1), filled: true, ..Node::default() };
        doc.boxes = vec![
            Node { label: "API".to_string(), colour: Some(2), rounded: true, children: vec![child], ..Node::default() },
            Node { label: "Billing".to_string(), filled: true, ..Node::default() },
        ];
        let expected = FileDoc {
            boxes: vec![
                FileBox {
                    label: "API".to_string(),
                    colour: Some(2),
                    fill: None,
                    rounded: true,
                    children: vec![FileBox { label: "Auth".to_string(), colour: Some(1), fill: Some(1), rounded: false, children: vec![] }],
                },
                FileBox { label: "Billing".to_string(), colour: None, fill: None, rounded: false, children: vec![] },
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
                FileBox { label: "API".to_string(), colour: None, fill: None, rounded: false, children: vec![] },
                FileBox { label: "Billing".to_string(), colour: None, fill: None, rounded: false, children: vec![] },
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
                    children: vec![FileBox { label: "Auth".to_string(), colour: None, fill: Some(3), rounded: false, children: vec![] }],
                },
                FileBox { label: "Billing".to_string(), colour: Some(4), fill: Some(1), rounded: false, children: vec![] },
            ],
        };
        let child = Node { label: "Auth".to_string(), filled: true, ..Node::default() };
        let expected = vec![
            Node { label: "API".to_string(), colour: Some(2), rounded: true, children: vec![child], ..Node::default() },
            Node { label: "Billing".to_string(), colour: Some(4), filled: true, ..Node::default() },
        ];
        assert_eq!(to_document(doc).boxes, expected);
    }

    #[test]
    fn from_document_writes_fill_equal_to_border_colour_index_when_filled() {
        let mut doc = Document::default();
        doc.boxes = vec![
            Node { label: "A".to_string(), colour: Some(2), filled: true, ..Node::default() },
            Node { label: "B".to_string(), colour: Some(2), filled: false, ..Node::default() },
        ];
        let fd = from_document(&doc);
        assert_eq!(fd.boxes[0].fill, Some(2));
        assert_eq!(fd.boxes[1].fill, None);
    }

    #[test]
    fn from_document_omits_fill_when_filled_but_colourless() {
        let mut doc = Document::default();
        doc.boxes = vec![Node { label: "A".to_string(), colour: None, filled: true, ..Node::default() }];
        let fd = from_document(&doc);
        assert_eq!(fd.boxes[0].fill, None);
    }

    #[test]
    fn filled_colourless_box_round_trips_as_unfilled() {
        let mut doc = Document::default();
        doc.boxes = vec![Node { label: "A".to_string(), colour: None, filled: true, ..Node::default() }];
        let fd = from_document(&doc);
        let reloaded = to_document(fd);
        assert_eq!(reloaded.boxes[0].filled, false);
        assert_eq!(reloaded.boxes[0].label, "A");
        assert_eq!(reloaded.boxes[0].colour, None);
    }

    #[test]
    fn legacy_fill_zero_means_filled() {
        let fd = FileDoc {
            boxes: vec![FileBox { label: "A".to_string(), colour: None, fill: Some(0), rounded: false, children: vec![] }],
        };
        let doc = to_document(fd);
        assert_eq!(doc.boxes[0].filled, true);
    }
}
