use crate::layout::PlacementNode;
use crate::render::{colour, CELL_HEIGHT, CELL_WIDTH};

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
        svg.push_str("</svg>");
        svg
    }
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
        assert!(!svg.contains("fill"));
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

        assert!(!svg.contains("fill"));
    }
}