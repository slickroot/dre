use std::collections::VecDeque;

const INDENT: &str = "  ";

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct FileDoc {
    pub(crate) boxes: Vec<FileBox>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FileBox {
    pub(crate) label: String,
    pub(crate) colour: Option<u8>,
    pub(crate) fill: Option<u8>,
    pub(crate) rounded: bool,
    pub(crate) children: Vec<FileBox>,
}

pub(crate) fn quote(label: &str) -> String {
    let escaped = label.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn line(node: &FileBox) -> String {
    let mut line = quote(&node.label);
    if let Some(colour) = node.colour {
        line.push_str(&format!(" colour={colour}"));
    }
    if let Some(fill) = node.fill {
        line.push_str(&format!(" fill={fill}"));
    }
    if node.rounded {
        line.push_str(" rounded");
    }
    line
}

struct Groups<'a> {
    next: usize,
    pending: VecDeque<(usize, &'a FileBox)>,
}

impl<'a> Groups<'a> {
    fn reference(&mut self, node: &'a FileBox) -> usize {
        let n = self.next;
        self.next += 1;
        self.pending.push_back((n, node));
        n
    }
}

fn write_tree<'a>(out: &mut String, node: &'a FileBox, depth: usize, groups: &mut Groups<'a>) {
    out.push_str(&INDENT.repeat(depth));
    if depth == 2 && !node.children.is_empty() {
        out.push('@');
        out.push_str(&groups.reference(node).to_string());
        out.push('\n');
        return;
    }
    out.push_str(&line(node));
    out.push('\n');
    for child in &node.children {
        write_tree(out, child, depth + 1, groups);
    }
}

pub(crate) fn write(doc: &FileDoc) -> String {
    let mut groups = Groups { next: 1, pending: VecDeque::new() };
    let mut blocks: Vec<String> = Vec::new();
    for node in &doc.boxes {
        let mut block = String::new();
        write_tree(&mut block, node, 0, &mut groups);
        blocks.push(block);
    }
    while let Some((n, node)) = groups.pending.pop_front() {
        let mut block = format!("@{n} {}\n", line(node));
        for child in &node.children {
            write_tree(&mut block, child, 1, &mut groups);
        }
        blocks.push(block);
    }
    blocks.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(label: &str) -> FileBox {
        FileBox { label: label.to_string(), colour: None, fill: None, rounded: false, children: Vec::new() }
    }

    fn node_with_children(label: &str, children: Vec<FileBox>) -> FileBox {
        FileBox { children, ..node(label) }
    }

    fn doc(boxes: Vec<FileBox>) -> FileDoc {
        FileDoc { boxes }
    }

    #[test]
    fn quote_wraps_the_label_in_double_quotes() {
        assert_eq!(quote("Auth"), "\"Auth\"");
    }

    #[test]
    fn quote_escapes_backslashes_and_double_quotes() {
        assert_eq!(quote(r#"a\b"c"#), r#""a\\b\"c""#);
    }

    #[test]
    fn a_plain_box_is_written_as_its_quoted_label_with_a_trailing_newline() {
        assert_eq!(write(&doc(vec![node("Billing")])), "\"Billing\"\n");
    }

    #[test]
    fn labels_are_escaped_when_serialized() {
        assert_eq!(write(&doc(vec![node(r#"say "hi""#)])), "\"say \\\"hi\\\"\"\n");
    }

    #[test]
    fn settings_follow_the_label_in_colour_fill_rounded_order() {
        let boxed = FileBox { colour: Some(2), fill: Some(1), rounded: true, ..node("API") };
        assert_eq!(write(&doc(vec![boxed])), "\"API\" colour=2 fill=1 rounded\n");
    }

    #[test]
    fn settings_at_their_defaults_are_left_out() {
        let colour_only = FileBox { colour: Some(3), fill: None, rounded: false, ..node("a") };
        let fill_only = FileBox { fill: Some(0), ..node("b") };
        let rounded_only = FileBox { rounded: true, ..node("c") };
        assert_eq!(write(&doc(vec![colour_only])), "\"a\" colour=3\n");
        assert_eq!(write(&doc(vec![fill_only])), "\"b\" fill=0\n");
        assert_eq!(write(&doc(vec![rounded_only])), "\"c\" rounded\n");
    }

    #[test]
    fn each_level_of_nesting_adds_two_spaces_of_indentation() {
        let tree = node_with_children(
            "API",
            vec![node("Auth"), node_with_children("Orders", vec![node("Postgres")])],
        );
        assert_eq!(
            write(&doc(vec![tree])),
            "\"API\"\n  \"Auth\"\n  \"Orders\"\n    \"Postgres\"\n"
        );
    }

    #[test]
    fn multiple_top_level_boxes_are_separated_by_a_blank_line() {
        let first = node_with_children("API", vec![node("Auth")]);
        assert_eq!(
            write(&doc(vec![first, node("Billing"), node("Search")])),
            "\"API\"\n  \"Auth\"\n\n\"Billing\"\n\n\"Search\"\n"
        );
    }

    #[test]
    fn a_leaf_at_depth_two_stays_inline() {
        let tree = node_with_children("A", vec![node_with_children("B", vec![node("C")])]);
        assert_eq!(write(&doc(vec![tree])), "\"A\"\n  \"B\"\n    \"C\"\n");
    }

    #[test]
    fn a_node_at_depth_two_with_children_becomes_a_group_reference_and_block() {
        let db = FileBox { rounded: true, ..node_with_children("DB", vec![node("Replica")]) };
        let tree = node_with_children("A", vec![node_with_children("B", vec![db])]);
        assert_eq!(
            write(&doc(vec![tree])),
            "\"A\"\n  \"B\"\n    @1\n\n@1 \"DB\" rounded\n  \"Replica\"\n"
        );
    }

    #[test]
    fn groups_nest_inside_groups_and_blocks_follow_first_reference_order() {
        let inner = node_with_children("Inner", vec![node("Leaf")]);
        let deep = node_with_children("X", vec![node_with_children("Y", vec![inner])]);
        let first = node_with_children("First", vec![deep]);
        let second = node_with_children("Second", vec![node("S")]);
        let tree = node_with_children("Root", vec![node_with_children("Mid", vec![first, second])]);
        assert_eq!(
            write(&doc(vec![tree])),
            "\"Root\"\n  \"Mid\"\n    @1\n    @2\n\n\
             @1 \"First\"\n  \"X\"\n    @3\n\n\
             @2 \"Second\"\n  \"S\"\n\n\
             @3 \"Y\"\n  \"Inner\"\n    \"Leaf\"\n"
        );
    }

    #[test]
    fn group_references_are_numbered_by_first_reference_order_regardless_of_label() {
        let group = |label: &str| node_with_children(label, vec![node("x")]);
        let tree = node_with_children(
            "A",
            vec![node_with_children("B", vec![group("DB"), group("db"), group("Db!")])],
        );
        assert_eq!(
            write(&doc(vec![tree])),
            "\"A\"\n  \"B\"\n    @1\n    @2\n    @3\n\n\
             @1 \"DB\"\n  \"x\"\n\n@2 \"db\"\n  \"x\"\n\n@3 \"Db!\"\n  \"x\"\n"
        );
    }

    #[test]
    fn the_spec_format_example_round_trips_to_the_exact_text() {
        let postgres = node_with_children(
            "Postgres",
            vec![node_with_children("Replica", vec![node("Backup")])],
        );
        let orders = FileBox { fill: Some(1), ..node_with_children("Orders", vec![postgres]) };
        let api = FileBox {
            colour: Some(2),
            rounded: true,
            ..node_with_children("API gateway", vec![node("Auth"), orders])
        };
        assert_eq!(
            write(&doc(vec![api, node("Billing")])),
            "\"API gateway\" colour=2 rounded\n  \"Auth\"\n  \"Orders\" fill=1\n    @1\n\n\
             \"Billing\"\n\n\
             @1 \"Postgres\"\n  \"Replica\"\n    \"Backup\"\n"
        );
    }
}
