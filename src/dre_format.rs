use std::collections::{HashSet, VecDeque};

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

pub(crate) fn slug(label: &str) -> String {
    let mut slug = String::new();
    for c in label.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() { "box".to_string() } else { slug.to_string() }
}

struct Groups<'a> {
    used: HashSet<String>,
    pending: VecDeque<(String, &'a Node)>,
}

impl<'a> Groups<'a> {
    fn reference(&mut self, node: &'a Node) -> String {
        let base = slug(&node.label);
        let mut name = base.clone();
        let mut n = 2;
        while self.used.contains(&name) {
            name = format!("{base}-{n}");
            n += 1;
        }
        self.used.insert(name.clone());
        self.pending.push_back((name.clone(), node));
        name
    }
}

fn write_tree<'a>(out: &mut String, node: &'a Node, depth: usize, groups: &mut Groups<'a>) {
    out.push_str(&INDENT.repeat(depth));
    if depth == 2 && !node.children.is_empty() {
        out.push('@');
        out.push_str(&groups.reference(node));
        out.push('\n');
        return;
    }
    out.push_str(&line(node));
    out.push('\n');
    for child in &node.children {
        write_tree(out, child, depth + 1, groups);
    }
}

pub(crate) fn serialize(boxes: &[Node]) -> String {
    let mut groups = Groups { used: HashSet::new(), pending: VecDeque::new() };
    let mut blocks: Vec<String> = Vec::new();
    for node in boxes {
        let mut block = String::new();
        write_tree(&mut block, node, 0, &mut groups);
        blocks.push(block);
    }
    while let Some((name, node)) = groups.pending.pop_front() {
        let mut block = format!("@{name} {}\n", line(node));
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

    #[test]
    fn slug_lowercases_ascii_letters_and_keeps_digits() {
        assert_eq!(slug("Postgres14"), "postgres14");
    }

    #[test]
    fn slug_turns_runs_of_other_characters_into_a_single_dash() {
        assert_eq!(slug("API  gateway/v2"), "api-gateway-v2");
        assert_eq!(slug("café bar"), "caf-bar");
    }

    #[test]
    fn slug_trims_dashes_from_both_ends() {
        assert_eq!(slug("  (Auth) "), "auth");
    }

    #[test]
    fn slug_falls_back_to_box_when_empty() {
        assert_eq!(slug(""), "box");
        assert_eq!(slug("!!"), "box");
    }

    #[test]
    fn a_leaf_at_depth_two_stays_inline() {
        let tree = node_with_children("A", vec![node_with_children("B", vec![node("C")])]);
        assert_eq!(serialize(&[tree]), "\"A\"\n  \"B\"\n    \"C\"\n");
    }

    #[test]
    fn a_node_at_depth_two_with_children_becomes_a_group_reference_and_block() {
        let db = Node { rounded: true, ..node_with_children("DB", vec![node("Replica")]) };
        let tree = node_with_children("A", vec![node_with_children("B", vec![db])]);
        assert_eq!(
            serialize(&[tree]),
            "\"A\"\n  \"B\"\n    @db\n\n@db \"DB\" rounded\n  \"Replica\"\n"
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
            serialize(&[tree]),
            "\"Root\"\n  \"Mid\"\n    @first\n    @second\n\n\
             @first \"First\"\n  \"X\"\n    @y\n\n\
             @second \"Second\"\n  \"S\"\n\n\
             @y \"Y\"\n  \"Inner\"\n    \"Leaf\"\n"
        );
    }

    #[test]
    fn repeated_slugs_get_numbered_suffixes_in_order() {
        let group = |label: &str| node_with_children(label, vec![node("x")]);
        let tree = node_with_children(
            "A",
            vec![node_with_children("B", vec![group("DB"), group("db"), group("Db!")])],
        );
        assert_eq!(
            serialize(&[tree]),
            "\"A\"\n  \"B\"\n    @db\n    @db-2\n    @db-3\n\n\
             @db \"DB\"\n  \"x\"\n\n@db-2 \"db\"\n  \"x\"\n\n@db-3 \"Db!\"\n  \"x\"\n"
        );
    }

    #[test]
    fn the_spec_format_example_round_trips_to_the_exact_text() {
        let postgres = node_with_children(
            "Postgres",
            vec![node_with_children("Replica", vec![node("Backup")])],
        );
        let orders = Node { fill: 1, ..node_with_children("Orders", vec![postgres]) };
        let api = Node {
            colour: 2,
            rounded: true,
            ..node_with_children("API gateway", vec![node("Auth"), orders])
        };
        assert_eq!(
            serialize(&[api, node("Billing")]),
            "\"API gateway\" colour=2 rounded\n  \"Auth\"\n  \"Orders\" fill=1\n    @postgres\n\n\
             \"Billing\"\n\n\
             @postgres \"Postgres\"\n  \"Replica\"\n    \"Backup\"\n"
        );
    }
}
