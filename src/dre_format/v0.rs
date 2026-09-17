use std::collections::{HashMap, HashSet, VecDeque};

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

pub(crate) fn read(text: &str) -> Option<FileDoc> {
    let doc = parse(text)?;
    (write(&doc) == text).then_some(doc)
}

enum Content {
    Header(String, Option<u8>, Option<u8>, bool),
    Ref(usize),
}

enum Draft {
    Box { label: String, colour: Option<u8>, fill: Option<u8>, rounded: bool, children: Vec<Draft> },
    Ref(usize),
}

struct ParsedBlock {
    defines: Option<usize>,
    root: Draft,
}

fn parse_quoted(s: &str) -> Option<(String, &str)> {
    let mut chars = s.char_indices();
    match chars.next() {
        Some((_, '"')) => {}
        _ => return None,
    }
    let mut result = String::new();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Some((result, &s[i + 1..])),
            '\\' => match chars.next() {
                Some((_, '\\')) => result.push('\\'),
                Some((_, '"')) => result.push('"'),
                _ => return None,
            },
            other => result.push(other),
        }
    }
    None
}

fn parse_header_body(s: &str) -> Option<(String, Option<u8>, Option<u8>, bool)> {
    let (label, rest) = parse_quoted(s)?;
    let mut colour = None;
    let mut fill = None;
    let mut rounded = false;
    if rest.is_empty() {
        return Some((label, colour, fill, rounded));
    }
    let rest = rest.strip_prefix(' ')?;
    if rest.is_empty() {
        return None;
    }
    for token in rest.split(' ') {
        if token.is_empty() {
            return None;
        }
        if token == "rounded" {
            if rounded {
                return None;
            }
            rounded = true;
        } else if let Some(v) = token.strip_prefix("colour=") {
            if colour.is_some() {
                return None;
            }
            colour = Some(v.parse::<u8>().ok()?);
        } else if let Some(v) = token.strip_prefix("fill=") {
            if fill.is_some() {
                return None;
            }
            fill = Some(v.parse::<u8>().ok()?);
        } else {
            return None;
        }
    }
    Some((label, colour, fill, rounded))
}

fn parse_first_line(s: &str) -> Option<(Option<usize>, String, Option<u8>, Option<u8>, bool)> {
    if let Some(rest) = s.strip_prefix('@') {
        let digit_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        if digit_end == 0 {
            return None;
        }
        let n: usize = rest[..digit_end].parse().ok()?;
        let after = rest[digit_end..].strip_prefix(' ')?;
        let (label, colour, fill, rounded) = parse_header_body(after)?;
        Some((Some(n), label, colour, fill, rounded))
    } else {
        let (label, colour, fill, rounded) = parse_header_body(s)?;
        Some((None, label, colour, fill, rounded))
    }
}

fn parse_child_line(s: &str) -> Option<Content> {
    if let Some(rest) = s.strip_prefix('@') {
        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
            let n: usize = rest.parse().ok()?;
            return Some(Content::Ref(n));
        }
        return None;
    }
    let (label, colour, fill, rounded) = parse_header_body(s)?;
    Some(Content::Header(label, colour, fill, rounded))
}

fn line_depth_content(s: &str) -> Option<(usize, Content)> {
    let indent = s.chars().take_while(|c| *c == ' ').count();
    if indent % 2 != 0 {
        return None;
    }
    let depth = indent / 2;
    let content_str = &s[indent..];
    if content_str.is_empty() {
        return None;
    }
    let content = parse_child_line(content_str)?;
    Some((depth, content))
}

fn build_children(lines: &[&str], idx: &mut usize, depth: usize) -> Option<Vec<Draft>> {
    let mut children = Vec::new();
    while *idx < lines.len() {
        let (d, content) = line_depth_content(lines[*idx])?;
        if d < depth {
            break;
        }
        if d > depth {
            return None;
        }
        *idx += 1;
        match content {
            Content::Ref(n) => children.push(Draft::Ref(n)),
            Content::Header(label, colour, fill, rounded) => {
                let sub_children = build_children(lines, idx, depth + 1)?;
                children.push(Draft::Box { label, colour, fill, rounded, children: sub_children });
            }
        }
    }
    Some(children)
}

fn parse_block(raw_lines: &[&str]) -> Option<ParsedBlock> {
    let first = raw_lines[0];
    if first.starts_with(' ') {
        return None;
    }
    let (defines, label, colour, fill, rounded) = parse_first_line(first)?;
    let mut idx = 1;
    let children = build_children(raw_lines, &mut idx, 1)?;
    if idx != raw_lines.len() {
        return None;
    }
    Some(ParsedBlock { defines, root: Draft::Box { label, colour, fill, rounded, children } })
}

fn resolve_draft(
    d: &Draft,
    defs: &HashMap<usize, &Draft>,
    resolving: &mut HashSet<usize>,
    cache: &mut HashMap<usize, FileBox>,
) -> Option<FileBox> {
    match d {
        Draft::Box { label, colour, fill, rounded, children } => {
            let mut out_children = Vec::new();
            for c in children {
                out_children.push(resolve_draft(c, defs, resolving, cache)?);
            }
            Some(FileBox {
                label: label.clone(),
                colour: *colour,
                fill: *fill,
                rounded: *rounded,
                children: out_children,
            })
        }
        Draft::Ref(n) => {
            if let Some(cached) = cache.get(n) {
                return Some(cached.clone());
            }
            if resolving.contains(n) {
                return None; // reference cycle
            }
            let target = *defs.get(n)?; // missing block
            resolving.insert(*n);
            let resolved = resolve_draft(target, defs, resolving, cache)?;
            resolving.remove(n);
            cache.insert(*n, resolved.clone());
            Some(resolved)
        }
    }
}

fn parse(text: &str) -> Option<FileDoc> {
    if text.is_empty() {
        return Some(FileDoc::default());
    }
    if !text.ends_with('\n') {
        return None;
    }
    let body = &text[..text.len() - 1];
    let block_strs: Vec<&str> = body.split("\n\n").collect();
    let mut blocks = Vec::new();
    for b in &block_strs {
        if b.is_empty() {
            return None;
        }
        let lines: Vec<&str> = b.split('\n').collect();
        if lines.iter().any(|l| l.is_empty()) {
            return None;
        }
        blocks.push(parse_block(&lines)?);
    }

    let mut defs: HashMap<usize, &Draft> = HashMap::new();
    for pb in &blocks {
        if let Some(n) = pb.defines {
            if defs.insert(n, &pb.root).is_some() {
                return None; // duplicate block
            }
        }
    }

    let mut resolving: HashSet<usize> = HashSet::new();
    let mut cache: HashMap<usize, FileBox> = HashMap::new();
    let mut top_boxes = Vec::new();
    for pb in &blocks {
        if pb.defines.is_none() {
            top_boxes.push(resolve_draft(&pb.root, &defs, &mut resolving, &mut cache)?);
        }
    }
    Some(FileDoc { boxes: top_boxes })
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

    fn spec_example_doc() -> FileDoc {
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
        doc(vec![api, node("Billing")])
    }

    #[test]
    fn read_returns_none_for_an_empty_string() {
        assert_eq!(read(""), Some(doc(vec![])));
    }

    #[test]
    fn read_round_trips_a_plain_box() {
        let d = doc(vec![node("Billing")]);
        assert_eq!(read(&write(&d)), Some(d));
    }

    #[test]
    fn read_round_trips_escaped_labels() {
        let d = doc(vec![node(r#"say "hi""#)]);
        assert_eq!(read(&write(&d)), Some(d));
    }

    #[test]
    fn read_round_trips_settings() {
        let boxed = FileBox { colour: Some(2), fill: Some(1), rounded: true, ..node("API") };
        let d = doc(vec![boxed]);
        assert_eq!(read(&write(&d)), Some(d));
    }

    #[test]
    fn read_round_trips_nested_indentation() {
        let tree = node_with_children(
            "API",
            vec![node("Auth"), node_with_children("Orders", vec![node("Postgres")])],
        );
        let d = doc(vec![tree]);
        assert_eq!(read(&write(&d)), Some(d));
    }

    #[test]
    fn read_round_trips_multiple_top_level_boxes() {
        let first = node_with_children("API", vec![node("Auth")]);
        let d = doc(vec![first, node("Billing"), node("Search")]);
        assert_eq!(read(&write(&d)), Some(d));
    }

    #[test]
    fn read_round_trips_grouped_depth_two_references() {
        let db = FileBox { rounded: true, ..node_with_children("DB", vec![node("Replica")]) };
        let tree = node_with_children("A", vec![node_with_children("B", vec![db])]);
        let d = doc(vec![tree]);
        assert_eq!(read(&write(&d)), Some(d));
    }

    #[test]
    fn read_round_trips_nested_groups_inside_groups() {
        let inner = node_with_children("Inner", vec![node("Leaf")]);
        let deep = node_with_children("X", vec![node_with_children("Y", vec![inner])]);
        let first = node_with_children("First", vec![deep]);
        let second = node_with_children("Second", vec![node("S")]);
        let tree = node_with_children("Root", vec![node_with_children("Mid", vec![first, second])]);
        let d = doc(vec![tree]);
        assert_eq!(read(&write(&d)), Some(d));
    }

    #[test]
    fn the_spec_example_text_reads_back_into_the_expected_tree() {
        let d = spec_example_doc();
        assert_eq!(read(&write(&d)), Some(d));
    }

    #[test]
    fn read_rejects_settings_out_of_order() {
        assert_eq!(read("\"API\" fill=1 colour=2\n"), None);
    }

    #[test]
    fn read_rejects_a_reference_to_a_missing_block() {
        assert_eq!(read("\"A\"\n  \"B\"\n    @1\n"), None);
    }

    #[test]
    fn read_rejects_a_duplicate_block() {
        let text = "\"A\"\n  \"B\"\n    @1\n\n@1 \"DB\"\n  \"Replica\"\n\n@1 \"DB2\"\n  \"Replica\"\n";
        assert_eq!(read(text), None);
    }

    #[test]
    fn read_rejects_text_missing_a_trailing_newline() {
        assert_eq!(read("\"Billing\""), None);
    }

    #[test]
    fn read_rejects_a_self_referencing_group() {
        // @1's own subtree (at depth 2) refers back to @1, forming a cycle.
        let text = "\"A\"\n  \"B\"\n    @1\n\n@1 \"DB\"\n  \"C\"\n    @1\n";
        assert_eq!(read(text), None);
    }
}
