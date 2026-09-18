use crate::layout::PlacementNode;
use crate::render::{colour, CELL_HEIGHT, CELL_WIDTH};

const TEXT_COLOUR: (u8, u8, u8) = (33, 33, 33);

#[allow(dead_code)]
pub(crate) struct SvgRenderer {}

#[allow(dead_code)]
impl SvgRenderer {
    pub(crate) fn render(&self, placements: &[crate::layout::Placement]) -> String {
        let (min_x, min_y, span_x, span_y) = view_box(placements);
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{min_x} {min_y} {span_x} {span_y}\">"
        );
        for placement in placements {
            if let PlacementNode::Node(node) = &placement.node {
                svg.push_str(&rect(placement, node));
            }
        }
        for placement in placements {
            if let PlacementNode::Arrow(_) = &placement.node {
            }
        }
        for placement in placements {
            if let PlacementNode::Label(label) = &placement.node {
                svg.push_str(&label_text(placement, label));
            }
        }
        svg.push_str("</svg>");
        svg
    }
}

fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[allow(dead_code)]
fn view_box(placements: &[crate::layout::Placement]) -> (i64, i64, i64, i64) {
    let mut min_x = 0;
    let mut min_y = 0;
    let mut max_x = 0;
    let mut max_y = 0;
    for placement in placements {
        min_x = min_x.min(placement.x);
        min_y = min_y.min(placement.y);
        max_x = max_x.max(placement.x + placement.width);
        max_y = max_y.max(placement.y + placement.height);
    }
    (
        min_x * CELL_WIDTH - CELL_HEIGHT,
        min_y * CELL_HEIGHT - CELL_HEIGHT,
        (max_x - min_x) * CELL_WIDTH + 2 * CELL_HEIGHT,
        (max_y - min_y) * CELL_HEIGHT + 2 * CELL_HEIGHT,
    )
}

#[allow(dead_code)]
fn rect(placement: &crate::layout::Placement, node: &crate::state::Node) -> String {
    use crate::render::{BORDER, OPAQUE, ROUNDED_RADIUS};
    use std::fmt::Write as _;

    let (r, g, b) = colour(node.colour);
    let mut rect = format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" stroke=\"rgb({r},{g},{b})\" stroke-width=\"{BORDER}\"",
        placement.x * CELL_WIDTH,
        placement.y * CELL_HEIGHT,
        placement.width * CELL_WIDTH,
        placement.height * CELL_HEIGHT,
    );
    if node.rounded {
        write!(rect, " rx=\"{ROUNDED_RADIUS}\"").unwrap();
    }
    if node.filled && node.colour.is_some() {
        let (fr, fg, fb) = crate::render::PALETTE[node.colour.unwrap() as usize];
        let opacity = crate::render::FILL_ALPHA as f64 / OPAQUE as f64;
        write!(rect, " fill=\"rgb({fr},{fg},{fb})\" fill-opacity=\"{opacity}\"").unwrap();
    }
    rect.push_str("/>");
    rect
}

fn label_text(
    placement: &crate::layout::Placement,
    label: &crate::layout::Label,
) -> String {
    let (r, g, b) = TEXT_COLOUR;
    let chars = label.text.chars().count() as i64;
    format!(
        "<text font-family=\"monospace\" font-size=\"{CELL_HEIGHT}\" text-anchor=\"start\" x=\"{}\" y=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\" fill=\"rgb({r},{g},{b})\">{}</text>",
        placement.x * CELL_WIDTH,
        placement.y * CELL_HEIGHT,
        chars * CELL_WIDTH,
        escape(label.text),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::BOX_HEIGHT;
    use crate::render::colour;
    use crate::render::BORDER;
    use crate::render::CELL_HEIGHT;
    use crate::render::CELL_WIDTH;
    use crate::render::FILL_ALPHA;
    use crate::render::OPAQUE;
    use crate::render::PALETTE;
    use crate::render::PLAIN_COLOUR;
    use crate::render::ROUNDED_RADIUS;
    use crate::state::Node;

    fn node(label: &str) -> Node {
        Node { label: label.to_string(), ..Default::default() }
    }

    fn boxed(label: &str, colour: Option<u8>, filled: bool, rounded: bool) -> Node {
        Node { label: label.to_string(), colour, filled, rounded, children: vec![] }
    }

    fn rgb(colour: (u8, u8, u8)) -> String {
        format!("rgb({},{},{})", colour.0, colour.1, colour.2)
    }

    fn view_box(min_x: i64, min_y: i64, span_x: i64, span_y: i64) -> String {
        format!("{min_x} {min_y} {span_x} {span_y}")
    }

    fn fill_opacity() -> String {
        format!("{}", FILL_ALPHA as f64 / OPAQUE as f64)
    }

    #[test]
    fn a_plain_leaf_box_renders_with_margins_and_no_fill_or_rounding() {
        let nodes = vec![node("hi")];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer {}.render(&placements);

        let box_width = crate::layout::width(&nodes[0]);
        let expected = view_box(
            -CELL_HEIGHT,
            -CELL_HEIGHT,
            box_width * CELL_WIDTH + 2 * CELL_HEIGHT,
            BOX_HEIGHT * CELL_HEIGHT + 2 * CELL_HEIGHT,
        );
        assert!(svg.contains(&format!("viewBox=\"{expected}\"")));
        assert!(svg.contains(&format!(
            "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>",
            box_width * CELL_WIDTH,
            BOX_HEIGHT * CELL_HEIGHT,
            rgb(PLAIN_COLOUR),
            BORDER,
        )));
        assert!(svg.contains(&format!("stroke=\"{}\"", rgb(colour(None)))));
        assert!(!svg.contains("rx"));
        let rect = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag")
            .split('<')
            .find(|element| element.starts_with("rect "))
            .expect("a plain leaf box renders a rect");
        assert!(!rect.contains("fill"));
    }

    #[test]
    fn a_coloured_filled_rounded_box_renders_stroke_fill_and_rx() {
        let nodes = vec![boxed("hi", Some(1), true, true)];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer {}.render(&placements);

        assert!(svg.contains(&format!("stroke=\"{}\"", rgb(colour(Some(1))))));
        assert!(svg.contains(&format!("stroke-width=\"{BORDER}\"")));
        assert!(svg.contains(&format!("rx=\"{ROUNDED_RADIUS}\"")));
        assert!(svg.contains(&format!("fill=\"{}\"", rgb(PALETTE[1]))));
        assert!(svg.contains(&format!("fill-opacity=\"{}\"", fill_opacity())));
    }

    #[test]
    fn empty_placements_render_a_document_with_the_empty_bbox_view_box() {
        let svg = SvgRenderer {}.render(&[]);

        let expected = view_box(
            -CELL_HEIGHT,
            -CELL_HEIGHT,
            2 * CELL_HEIGHT,
            2 * CELL_HEIGHT,
        );
        assert!(svg.contains(&format!("viewBox=\"{expected}\"")));
        assert!(!svg.contains("<rect"));
    }

    #[test]
    fn a_fill_is_only_emitted_for_a_filled_coloured_box() {
        let nodes = vec![
            boxed("plain", None, true, false),
            boxed("colour", Some(1), false, false),
        ];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer {}.render(&placements);

        for element in svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag")
            .split('<')
            .filter(|element| element.starts_with("rect "))
        {
            assert!(!element.contains("fill"));
        }
    }

    fn label_placement<'a>(text: &'a str, x: i64, y: i64) -> crate::layout::Placement<'a> {
        crate::layout::Placement {
            node: crate::layout::PlacementNode::Label(crate::layout::Label {
                text,
                path: crate::state::Path { ancestors: vec![], index: 0 },
            }),
            x,
            y,
            width: text.chars().count() as i64,
            height: 1,
        }
    }

    #[test]
    fn a_single_label_over_a_box_renders_one_text_element() {
        let node = node("hi");
        let label_x = 2;
        let label_y = 1;
        let placements = vec![
            crate::layout::Placement {
                node: crate::layout::PlacementNode::Node(&node),
                x: 0,
                y: 0,
                width: 4,
                height: 3,
            },
            label_placement("hi", label_x, label_y),
        ];

        let svg = SvgRenderer {}.render(&placements);

        let (r, g, b) = TEXT_COLOUR;
        assert!(svg.contains(&format!(
            "<text font-family=\"monospace\" font-size=\"{CELL_HEIGHT}\" text-anchor=\"start\" x=\"{}\" y=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\" fill=\"rgb({r},{g},{b})\"",
            label_x * CELL_WIDTH,
            label_y * CELL_HEIGHT,
            2 * CELL_WIDTH,
        )));
        assert!(svg.contains(">hi</text>"));
    }

    #[test]
    fn boxes_are_drawn_before_labels() {
        let node = node("hi");
        let placements = vec![
            crate::layout::Placement {
                node: crate::layout::PlacementNode::Node(&node),
                x: 0,
                y: 0,
                width: 4,
                height: 3,
            },
            label_placement("hi", 2, 1),
        ];

        let svg = SvgRenderer {}.render(&placements);

        let rect = svg.find("<rect").expect("a box placement draws a rect");
        let text = svg.find("<text").expect("a label placement draws a text");
        assert!(rect < text);
    }

    #[test]
    fn a_label_with_xml_special_characters_escapes_them_in_the_text_content() {
        let node = node("hi");
        let placements = vec![
            crate::layout::Placement {
                node: crate::layout::PlacementNode::Node(&node),
                x: 0,
                y: 0,
                width: 4,
                height: 3,
            },
            label_placement("a<b>&c", 2, 1),
        ];

        let svg = SvgRenderer {}.render(&placements);

        assert!(svg.contains(">a&lt;b&gt;&amp;c</text>"));
        assert!(!svg.contains("a<b>&c"));
    }
}