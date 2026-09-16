use crate::state::{Node, PLAIN};

const INDENT: &str = "  ";

pub(crate) fn quote(label: &str) -> String {
    let escaped = label.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn line(node: &Node) -> String {
    let mut line = quote(&node.label);
    if node.colour != PLAIN {
        line.push_str(&format!(" colour={}", node.colour));
    }
    if node.fill != PLAIN {
        line.push_str(&format!(" fill={}", node.fill));
    }
    if node.rounded {
        line.push_str(" rounded");
    }
    line
}

fn write_tree(out: &mut String, node: &Node, depth: usize) {
    out.push_str(&INDENT.repeat(depth));
    out.push_str(&line(node));
    out.push('\n');
    for child in &node.children {
        write_tree(out, child, depth + 1);
    }
}

pub(crate) fn serialize(boxes: &[Node]) -> String {
    let blocks: Vec<String> = boxes
        .iter()
        .map(|node| {
            let mut block = String::new();
            write_tree(&mut block, node, 0);
            block
        })
        .collect();
    blocks.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(label: &str) -> Node {
        Node { label: label.to_string(), ..Default::default() }
    }

    fn node_with_children(label: &str, children: Vec<Node>) -> Node {
        Node { label: label.to_string(), children, ..Default::default() }
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
        assert_eq!(serialize(&[node("Billing")]), "\"Billing\"\n");
    }

    #[test]
    fn labels_are_escaped_when_serialized() {
        assert_eq!(serialize(&[node(r#"say "hi""#)]), "\"say \\\"hi\\\"\"\n");
    }

    #[test]
    fn settings_follow_the_label_in_colour_fill_rounded_order() {
        let boxed = Node { colour: 2, fill: 1, rounded: true, ..node("API") };
        assert_eq!(serialize(&[boxed]), "\"API\" colour=2 fill=1 rounded\n");
    }

    #[test]
    fn settings_at_their_defaults_are_left_out() {
        let colour_only = Node { colour: 3, fill: PLAIN, rounded: false, ..node("a") };
        let fill_only = Node { fill: 0, ..node("b") };
        let rounded_only = Node { rounded: true, ..node("c") };
        assert_eq!(serialize(&[colour_only]), "\"a\" colour=3\n");
        assert_eq!(serialize(&[fill_only]), "\"b\" fill=0\n");
        assert_eq!(serialize(&[rounded_only]), "\"c\" rounded\n");
    }

    #[test]
    fn each_level_of_nesting_adds_two_spaces_of_indentation() {
        let tree = node_with_children(
            "API",
            vec![node("Auth"), node_with_children("Orders", vec![node("Postgres")])],
        );
        assert_eq!(
            serialize(&[tree]),
            "\"API\"\n  \"Auth\"\n  \"Orders\"\n    \"Postgres\"\n"
        );
    }

    #[test]
    fn multiple_top_level_boxes_are_separated_by_a_blank_line() {
        let first = node_with_children("API", vec![node("Auth")]);
        assert_eq!(
            serialize(&[first, node("Billing"), node("Search")]),
            "\"API\"\n  \"Auth\"\n\n\"Billing\"\n\n\"Search\"\n"
        );
    }
}
