use crate::layout::PlacementNode;
use crate::render::{arrowhead_depth, arrowhead_slope, colour, CELL_HEIGHT, CELL_WIDTH};

const ARROW_STROKE: i64 = 2;
const ARROW_JOIN_OVERLAP: i64 = ARROW_STROKE / 2;
const INK: (u8, u8, u8) = (0, 0, 0);

pub(crate) struct SvgRenderer {}

impl SvgRenderer {
    pub(crate) fn render(&self, placements: &[crate::layout::Placement]) -> String {
        let (min_x, min_y, span_x, span_y) = view_box(placements);
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{min_x} {min_y} {span_x} {span_y}\">"
        );
        svg.push_str(&style_block());
        if placements.iter().any(|placement| matches!(placement.node, PlacementNode::Arrow(_))) {
            svg.push_str(&marker_defs());
        }
        for placement in placements {
            if let PlacementNode::Arrow(arrow) = &placement.node {
                svg.push_str(&arrow_paths(placement, arrow));
            }
        }
        for placement in placements {
            if let PlacementNode::Node(node) = &placement.node {
                svg.push_str(&rect(placement, node));
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

fn style_block() -> String {
    let (r, g, b) = INK;
    format!(
        "<style>svg {{ --ink: rgb({r},{g},{b}) }}@media (prefers-color-scheme: dark) {{ svg {{ --ink: rgb(255,255,255) }} }}</style>"
    )
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

fn marker_defs() -> String {
    let depth = arrowhead_depth();
    let slope = arrowhead_slope();
    let arm = depth * slope;
    let box_width = depth.ceil() as i64;
    let box_height = (arm * 2.0).ceil() as i64;
    let tip_x = box_width as f64;
    let tip_y = box_height as f64 / 2.0;
    let base_x = box_width as f64 - depth;
    format!(
        "<defs><marker id=\"arrowhead\" orient=\"auto\" markerUnits=\"userSpaceOnUse\" markerWidth=\"{box_width}\" markerHeight=\"{box_height}\" refX=\"{tip_x}\" refY=\"{tip_y}\" viewBox=\"0 0 {box_width} {box_height}\"><path d=\"M {tip_x} {tip_y} L {base_x} {} M {tip_x} {tip_y} L {base_x} {}\" stroke=\"var(--ink)\" stroke-width=\"{}\" fill=\"none\"/></marker></defs>",
        tip_y - arm,
        tip_y + arm,
        ARROW_STROKE,
    )
}

fn arrow_paths(
    placement: &crate::layout::Placement,
    arrow: &crate::layout::Arrow,
) -> String {
    let left = placement.x * CELL_WIDTH;
    let right = placement.x * CELL_WIDTH + placement.width * CELL_WIDTH - 1;
    let trunk_x = placement.x * CELL_WIDTH + (placement.width * CELL_WIDTH) / 2;
    let shaft_row = (placement.y + arrow.shaft) * CELL_HEIGHT + CELL_HEIGHT / 2;
    let stop_rows: Vec<i64> = arrow
        .stops
        .iter()
        .map(|stop| (placement.y + stop) * CELL_HEIGHT + CELL_HEIGHT / 2)
        .collect();
    let trunk_top = *stop_rows.iter().min().expect("an arrow always has at least one stop");
    let trunk_bottom = *stop_rows.iter().max().expect("an arrow always has at least one stop");
    let mut paths = format!(
        "<path d=\"M {left} {shaft_row} L {} {shaft_row}\" stroke=\"var(--ink)\" stroke-width=\"{}\" fill=\"none\"/>",
        trunk_x + ARROW_JOIN_OVERLAP,
        ARROW_STROKE
    );
    paths.push_str(&format!(
        "<path d=\"M {trunk_x} {trunk_top} L {trunk_x} {trunk_bottom}\" stroke=\"var(--ink)\" stroke-width=\"{}\" fill=\"none\"/>",
        ARROW_STROKE
    ));
    for row in stop_rows {
        paths.push_str(&format!(
            "<path d=\"M {} {row} L {right} {row}\" marker-end=\"url(#arrowhead)\" stroke=\"var(--ink)\" stroke-width=\"{}\" fill=\"none\"/>",
            trunk_x - ARROW_JOIN_OVERLAP,
            ARROW_STROKE
        ));
    }
    paths
}

fn rect(placement: &crate::layout::Placement, node: &crate::state::Node) -> String {
    use crate::render::{BORDER, OPAQUE, ROUNDED_RADIUS};
    use std::fmt::Write as _;

    let stroke = match node.colour {
        None => "var(--ink)".to_string(),
        Some(_) => {
            let (r, g, b) = colour(node.colour);
            format!("rgb({r},{g},{b})")
        }
    };
    let mut rect = format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" stroke=\"{stroke}\" stroke-width=\"{}\"",
        placement.x * CELL_WIDTH,
        placement.y * CELL_HEIGHT,
        placement.width * CELL_WIDTH,
        placement.height * CELL_HEIGHT,
        BORDER / 2,
    );
    if node.rounded {
        write!(rect, " rx=\"{ROUNDED_RADIUS}\"").unwrap();
    }
    if node.filled && node.colour.is_some() {
        let (fr, fg, fb) = crate::render::PALETTE[node.colour.unwrap() as usize];
        let opacity = crate::render::FILL_ALPHA as f64 / OPAQUE as f64;
        write!(rect, " fill=\"rgb({fr},{fg},{fb})\" fill-opacity=\"{opacity}\"").unwrap();
    } else {
        rect.push_str(" fill=\"none\"");
    }
    rect.push_str("/>");
    rect
}

fn label_text(
    placement: &crate::layout::Placement,
    label: &crate::layout::Label,
) -> String {
    let chars = label.text.chars().count() as i64;
    format!(
        "<text font-family=\"monospace\" font-size=\"{CELL_HEIGHT}\" text-anchor=\"start\" dominant-baseline=\"central\" x=\"{}\" y=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\" fill=\"var(--ink)\">{}</text>",
        placement.x * CELL_WIDTH,
        placement.y * CELL_HEIGHT + CELL_HEIGHT / 2,
        chars * CELL_WIDTH,
        escape(label.text),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::BOX_HEIGHT;
    use crate::render::arrowhead_depth;
    use crate::render::arrowhead_slope;
    use crate::render::colour;
    use crate::render::BORDER;
    use crate::render::CELL_HEIGHT;
    use crate::render::CELL_WIDTH;
    use crate::render::FILL_ALPHA;
    use crate::render::OPAQUE;
    use crate::render::PALETTE;
    use crate::render::ROUNDED_RADIUS;
    use crate::state::Node;

    fn node(label: &str) -> Node {
        Node { label: label.to_string(), ..Default::default() }
    }

    fn boxed(label: &str, colour: Option<u8>, filled: bool, rounded: bool) -> Node {
        Node { label: label.to_string(), colour, filled, rounded, children: vec![] }
    }

    fn node_with_children(label: &str, children: Vec<Node>) -> Node {
        Node { label: label.to_string(), children, ..Default::default() }
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
    fn a_plain_leaf_box_renders_with_margins_a_transparent_fill_and_no_rounding() {
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
            "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" stroke=\"var(--ink)\" stroke-width=\"{}\" fill=\"none\"/>",
            box_width * CELL_WIDTH,
            BOX_HEIGHT * CELL_HEIGHT,
            BORDER / 2,
        )));
        assert!(svg.contains("stroke=\"var(--ink)\""));
        assert!(!svg.contains("rx"));
        let rect = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag")
            .split('<')
            .find(|element| element.starts_with("rect "))
            .expect("a plain leaf box renders a rect");
        assert!(rect.contains("fill=\"none\""));
    }

    #[test]
    fn a_coloured_filled_rounded_box_renders_stroke_fill_and_rx() {
        let nodes = vec![boxed("hi", Some(1), true, true)];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer {}.render(&placements);

        assert!(svg.contains(&format!("stroke=\"{}\"", rgb(colour(Some(1))))));
        assert!(svg.contains(&format!("stroke-width=\"{}\"", BORDER / 2)));
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
    fn every_box_declares_a_fill_and_only_a_filled_coloured_box_gets_a_colour_fill() {
        let nodes = vec![
            boxed("plain", None, false, false),
            boxed("plain_filled", None, true, false),
            boxed("colour", Some(2), false, false),
            boxed("colour_filled", Some(1), true, false),
        ];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer {}.render(&placements);

        let rects: Vec<&str> = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag")
            .split('<')
            .filter(|element| element.starts_with("rect "))
            .collect();

        assert_eq!(rects.len(), 4);

        for rect in &rects {
            assert!(rect.contains("fill="));
        }

        let (pr, pg, pb) = PALETTE[1];
        let palette_fill = format!("fill=\"rgb({pr},{pg},{pb})\"");
        let colour_filled_count = rects.iter().filter(|r| r.contains(&palette_fill)).count();
        assert_eq!(colour_filled_count, 1);

        let none_fill_count = rects.iter().filter(|r| r.contains("fill=\"none\"")).count();
        assert_eq!(none_fill_count, 3);
    }

    #[test]
    fn colourless_boxes_are_ink_while_coloured_boxes_keep_palette_colours() {
        let nodes = vec![
            boxed("plain", None, false, false),
            boxed("colour", Some(2), false, false),
            node_with_children("root", vec![node("A")]),
        ];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer {}.render(&placements);

        let ink_stroke = "stroke=\"var(--ink)\"".to_string();
        let coloured_stroke = format!("stroke=\"{}\"", rgb(PALETTE[2]));

        let rects: Vec<&str> = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag")
            .split('<')
            .filter(|element| element.starts_with("rect "))
            .collect();

        assert!(rects.iter().all(|rect| rect.contains(&ink_stroke) || rect.contains(&coloured_stroke)));
        assert_eq!(rects.iter().filter(|rect| rect.contains(&coloured_stroke)).count(), 1);
        assert_eq!(
            rects.iter().filter(|rect| rect.contains(&ink_stroke)).count(),
            rects.len() - 1
        );
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
    fn a_label_over_a_coloured_box_is_vertically_centred_on_the_box_midline() {
        let node = boxed("hi", Some(1), false, false);
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

        assert!(svg.contains(&format!(
            "<text font-family=\"monospace\" font-size=\"{CELL_HEIGHT}\" text-anchor=\"start\" dominant-baseline=\"central\" x=\"{}\" y=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\" fill=\"var(--ink)\"",
            label_x * CELL_WIDTH,
            label_y * CELL_HEIGHT + CELL_HEIGHT / 2,
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

    #[test]
    fn an_arrow_with_two_stops_renders_a_defs_marker_before_boxes_and_labels() {
        let parent = node_with_children("parent", vec![node("a"), node("b")]);
        let nodes = vec![parent];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer {}.render(&placements);

        let defs = svg.find("<defs>").expect("arrows emit a defs block");
        let rect = svg.find("<rect").expect("the parent box draws a rect");
        let arm = svg
            .find("marker-end=\"url(#arrowhead)\"")
            .expect("each stop arm references the arrowhead");
        let text = svg.find("<text").expect("labels draw text");
        assert!(defs < rect, "defs are emitted before the first rect");
        assert!(defs < arm, "defs are emitted before the stop arms");
        assert!(arm < rect, "stop arms are drawn before the boxes so box borders sit on top");
        assert!(arm < text, "arrow paths are drawn before labels");
        assert!(svg.contains(
            "<marker id=\"arrowhead\" orient=\"auto\" markerUnits=\"userSpaceOnUse\""
        ));

        let arrow_placement = placements
            .iter()
            .find(|placement| matches!(placement.node, PlacementNode::Arrow(_)))
            .expect("a parent with children yields an arrow placement");
        let arrow = match &arrow_placement.node {
            PlacementNode::Arrow(arrow) => arrow,
            _ => unreachable!("the arrow placement wraps an Arrow"),
        };

        let left = arrow_placement.x * CELL_WIDTH;
        let right = arrow_placement.x * CELL_WIDTH + arrow_placement.width * CELL_WIDTH - 1;
        let trunk_x = arrow_placement.x * CELL_WIDTH + (arrow_placement.width * CELL_WIDTH) / 2;
        let shaft_row = (arrow_placement.y + arrow.shaft) * CELL_HEIGHT + CELL_HEIGHT / 2;
        let stop_rows: Vec<i64> = arrow
            .stops
            .iter()
            .map(|stop| (arrow_placement.y + stop) * CELL_HEIGHT + CELL_HEIGHT / 2)
            .collect();
        let trunk_top = *stop_rows.iter().min().expect("arrows have at least one stop");
        let trunk_bottom = *stop_rows.iter().max().expect("arrows have at least one stop");
        let ink = "var(--ink)";

        assert_eq!(stop_rows.len(), 2);
        assert!(svg.contains(&format!(
            "<path d=\"M {left} {shaft_row} L {} {shaft_row}\" stroke=\"{ink}\" stroke-width=\"{}\" fill=\"none\"/>",
            trunk_x + ARROW_JOIN_OVERLAP,
            ARROW_STROKE
        )));
        assert!(svg.contains(&format!(
            "<path d=\"M {trunk_x} {trunk_top} L {trunk_x} {trunk_bottom}\" stroke=\"{ink}\" stroke-width=\"{}\" fill=\"none\"/>",
            ARROW_STROKE
        )));
        for row in stop_rows {
            assert!(svg.contains(&format!(
                "<path d=\"M {} {row} L {right} {row}\" marker-end=\"url(#arrowhead)\" stroke=\"{ink}\" stroke-width=\"{}\" fill=\"none\"/>",
                trunk_x - ARROW_JOIN_OVERLAP,
                ARROW_STROKE
            )));
        }
    }

    #[test]
    fn the_join_overlap_seams_each_horizontal_stroke_into_the_trunk_for_a_flush_elbow() {
        let parent = node_with_children("parent", vec![node("a"), node("b")]);
        let nodes = vec![parent];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer {}.render(&placements);

        let arrow_placement = placements
            .iter()
            .find(|placement| matches!(placement.node, PlacementNode::Arrow(_)))
            .expect("a parent with children yields an arrow placement");
        let arrow = match &arrow_placement.node {
            PlacementNode::Arrow(arrow) => arrow,
            _ => unreachable!("the arrow placement wraps an Arrow"),
        };

        let left = arrow_placement.x * CELL_WIDTH;
        let right = arrow_placement.x * CELL_WIDTH + arrow_placement.width * CELL_WIDTH - 1;
        let trunk_x = arrow_placement.x * CELL_WIDTH + (arrow_placement.width * CELL_WIDTH) / 2;
        let shaft_row = (arrow_placement.y + arrow.shaft) * CELL_HEIGHT + CELL_HEIGHT / 2;
        let stop_rows: Vec<i64> = arrow
            .stops
            .iter()
            .map(|stop| (arrow_placement.y + stop) * CELL_HEIGHT + CELL_HEIGHT / 2)
            .collect();
        let trunk_top = *stop_rows.iter().min().expect("arrows have at least one stop");
        let trunk_bottom = *stop_rows.iter().max().expect("arrows have at least one stop");
        let ink = "var(--ink)";

        assert!(svg.contains(&format!(
            "<path d=\"M {left} {shaft_row} L {} {shaft_row}\" stroke=\"{ink}\" stroke-width=\"{}\" fill=\"none\"/>",
            trunk_x + ARROW_JOIN_OVERLAP,
            ARROW_STROKE
        )));
        assert!(svg.contains(&format!(
            "<path d=\"M {trunk_x} {trunk_top} L {trunk_x} {trunk_bottom}\" stroke=\"{ink}\" stroke-width=\"{}\" fill=\"none\"/>",
            ARROW_STROKE
        )));
        for row in stop_rows {
            assert!(svg.contains(&format!(
                "<path d=\"M {} {row} L {right} {row}\" marker-end=\"url(#arrowhead)\" stroke=\"{ink}\" stroke-width=\"{}\" fill=\"none\"/>",
                trunk_x - ARROW_JOIN_OVERLAP,
                ARROW_STROKE
            )));
        }
    }

    #[test]
    fn the_arrowhead_marker_geometry_is_computed_from_the_mirrored_constants() {
        let parent = node_with_children("parent", vec![node("a"), node("b")]);
        let nodes = vec![parent];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer {}.render(&placements);

        let depth = arrowhead_depth();
        let slope = arrowhead_slope();
        let arm = depth * slope;
        let box_width = depth.ceil() as i64;
        let box_height = (arm * 2.0).ceil() as i64;
        let tip_x = box_width as f64;
        let tip_y = box_height as f64 / 2.0;
        let base_x = box_width as f64 - depth;

        assert!(svg.contains(&format!(
            "<marker id=\"arrowhead\" orient=\"auto\" markerUnits=\"userSpaceOnUse\" markerWidth=\"{box_width}\" markerHeight=\"{box_height}\" refX=\"{tip_x}\" refY=\"{tip_y}\" viewBox=\"0 0 {box_width} {box_height}\">"
        )));
        assert!(svg.contains(&format!(
            "d=\"M {tip_x} {tip_y} L {base_x} {} M {tip_x} {tip_y} L {base_x} {}\"",
            tip_y - arm,
            tip_y + arm
        )));
        assert!(svg.contains("stroke=\"var(--ink)\""));
        assert!(svg.contains(&format!("stroke-width=\"{}\"", ARROW_STROKE)));
        assert!(svg.contains("fill=\"none\""));
        let marker = svg
            .split("</defs>")
            .next()
            .expect("arrows emit a defs block")
            .split('<')
            .find(|element| element.starts_with("path "))
            .expect("the marker contains a path");
        assert!(!marker.contains('Z'));
    }

    #[test]
    fn a_chart_without_arrows_has_no_defs_or_arrowhead() {
        let nodes = vec![node("hi")];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer {}.render(&placements);

        assert!(!svg.contains("<defs>"));
        assert!(!svg.contains("marker-end"));
        assert!(!svg.contains("arrowhead"));
    }

    fn expected_style() -> String {
        format!(
            "<style>svg {{ --ink: {} }}@media (prefers-color-scheme: dark) {{ svg {{ --ink: rgb(255,255,255) }} }}</style>",
            rgb(INK)
        )
    }

    #[test]
    fn the_style_block_follows_the_svg_tag_on_every_document() {
        let svg = SvgRenderer {}.render(&[]);

        let opening = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{}\">",
            view_box(
                -CELL_HEIGHT,
                -CELL_HEIGHT,
                2 * CELL_HEIGHT,
                2 * CELL_HEIGHT,
            )
        );
        assert!(svg.contains("<style>"));
        assert!(svg.contains("</style>"));
        assert!(svg.starts_with(&format!("{opening}{}", expected_style())));
    }

    #[test]
    fn the_style_light_default_is_built_from_the_ink_constant() {
        let svg = SvgRenderer {}.render(&[]);

        assert!(svg.contains(&format!("svg {{ --ink: {} }}", rgb(INK))));
    }

    #[test]
    fn the_style_dark_override_uses_a_media_query_with_white_ink() {
        let svg = SvgRenderer {}.render(&[]);

        assert!(svg.contains(
            "@media (prefers-color-scheme: dark) { svg { --ink: rgb(255,255,255) } }"
        ));
    }

    #[test]
    fn the_style_block_precedes_defs_and_immediately_follows_the_svg_tag() {
        let parent = node_with_children("parent", vec![node("a"), node("b")]);
        let nodes = vec![parent];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer {}.render(&placements);

        let body = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag");
        let elements: Vec<&str> = body.split('<').collect();
        assert!(elements[1].starts_with("svg "), "the document opens with the svg tag");
        assert!(
            elements[2].starts_with("style>"),
            "the style block immediately follows the opening svg tag"
        );
        let style = svg.find("<style>").expect("the document emits a style block");
        let defs = svg.find("<defs>").expect("arrows emit a defs block");
        let rect = svg.find("<rect").expect("a box draws a rect");
        assert!(style < defs, "the style block precedes any defs");
        assert!(style < rect, "the style block precedes the boxes");
    }

    fn expected_marker() -> String {
        let depth = arrowhead_depth();
        let slope = arrowhead_slope();
        let arm = depth * slope;
        let box_width = depth.ceil() as i64;
        let box_height = (arm * 2.0).ceil() as i64;
        let tip_x = box_width as f64;
        let tip_y = box_height as f64 / 2.0;
        let base_x = box_width as f64 - depth;
        format!(
            "<defs><marker id=\"arrowhead\" orient=\"auto\" markerUnits=\"userSpaceOnUse\" markerWidth=\"{box_width}\" markerHeight=\"{box_height}\" refX=\"{tip_x}\" refY=\"{tip_y}\" viewBox=\"0 0 {box_width} {box_height}\"><path d=\"M {tip_x} {tip_y} L {base_x} {} M {tip_x} {tip_y} L {base_x} {}\" stroke=\"var(--ink)\" stroke-width=\"{}\" fill=\"none\"/></marker></defs>",
            tip_y - arm,
            tip_y + arm,
            ARROW_STROKE
        )
    }

    fn rect_at(
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        colour_index: Option<u8>,
        filled: bool,
        rounded: bool,
    ) -> String {
        use std::fmt::Write as _;

        let stroke = match colour_index {
            None => "var(--ink)".to_string(),
            Some(_) => rgb(colour(colour_index)),
        };
        let mut rect = format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" stroke=\"{stroke}\" stroke-width=\"{}\"",
            x * CELL_WIDTH,
            y * CELL_HEIGHT,
            width * CELL_WIDTH,
            height * CELL_HEIGHT,
            BORDER / 2,
        );
        if rounded {
            write!(rect, " rx=\"{ROUNDED_RADIUS}\"").unwrap();
        }
        if filled && colour_index.is_some() {
            let (fr, fg, fb) = PALETTE[colour_index.unwrap() as usize];
            write!(
                rect,
                " fill=\"rgb({fr},{fg},{fb})\" fill-opacity=\"{}\"",
                fill_opacity()
            )
            .unwrap();
        } else {
            rect.push_str(" fill=\"none\"");
        }
        rect.push_str("/>");
        rect
    }

    fn label_at(x: i64, y: i64, text: &str) -> String {
        let chars = text.chars().count() as i64;
        format!(
            "<text font-family=\"monospace\" font-size=\"{CELL_HEIGHT}\" text-anchor=\"start\" dominant-baseline=\"central\" x=\"{}\" y=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\" fill=\"var(--ink)\">{text}</text>",
            x * CELL_WIDTH,
            y * CELL_HEIGHT + CELL_HEIGHT / 2,
            chars * CELL_WIDTH,
        )
    }

    fn arrow_at(x: i64, y: i64, width: i64, shaft: i64, stops: &[i64]) -> String {
        let left = x * CELL_WIDTH;
        let right = x * CELL_WIDTH + width * CELL_WIDTH - 1;
        let trunk_x = x * CELL_WIDTH + (width * CELL_WIDTH) / 2;
        let shaft_row = (y + shaft) * CELL_HEIGHT + CELL_HEIGHT / 2;
        let stop_rows: Vec<i64> = stops
            .iter()
            .map(|&stop| (y + stop) * CELL_HEIGHT + CELL_HEIGHT / 2)
            .collect();
        let trunk_top = *stop_rows.iter().min().expect("an arrow has stops");
        let trunk_bottom = *stop_rows.iter().max().expect("an arrow has stops");
        let mut paths = format!(
            "<path d=\"M {left} {shaft_row} L {} {shaft_row}\" stroke=\"var(--ink)\" stroke-width=\"{}\" fill=\"none\"/>",
            trunk_x + ARROW_JOIN_OVERLAP,
            ARROW_STROKE
        );
        paths.push_str(&format!(
            "<path d=\"M {trunk_x} {trunk_top} L {trunk_x} {trunk_bottom}\" stroke=\"var(--ink)\" stroke-width=\"{}\" fill=\"none\"/>",
            ARROW_STROKE
        ));
        for row in stop_rows {
            paths.push_str(&format!(
                "<path d=\"M {} {row} L {right} {row}\" marker-end=\"url(#arrowhead)\" stroke=\"var(--ink)\" stroke-width=\"{}\" fill=\"none\"/>",
                trunk_x - ARROW_JOIN_OVERLAP,
                ARROW_STROKE
            ));
        }
        paths
    }

    #[test]
    fn renders_the_spec_example_diagram_as_a_whole_document() {
        let nodes = vec![
            node("start"),
            boxed("greet", Some(1), true, true),
            boxed("warn", Some(3), false, false),
            node_with_children("root", vec![node("A"), node("B"), node("C")]),
        ];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer {}.render(&placements);

        let expected = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{}\">{}{}{}{}{}</svg>",
            view_box(
                -CELL_HEIGHT,
                -CELL_HEIGHT,
                18 * CELL_WIDTH + 2 * CELL_HEIGHT,
                33 * CELL_HEIGHT + 2 * CELL_HEIGHT,
            ),
            expected_style(),
            expected_marker(),
            arrow_at(7, 19, 8, 6, &[0, 6, 12]),
            [
                rect_at(0, 0, 7, 3, None, false, false),
                rect_at(0, 6, 7, 3, Some(1), true, true),
                rect_at(0, 12, 7, 3, Some(3), false, false),
                rect_at(0, 24, 7, 3, None, false, false),
                rect_at(15, 18, 3, 3, None, false, false),
                rect_at(15, 24, 3, 3, None, false, false),
                rect_at(15, 30, 3, 3, None, false, false),
            ]
            .concat(),
            [
                label_at(1, 1, "start"),
                label_at(1, 7, "greet"),
                label_at(2, 13, "warn"),
                label_at(2, 25, "root"),
                label_at(16, 19, "A"),
                label_at(16, 25, "B"),
                label_at(16, 31, "C"),
            ]
            .concat(),
        );

        assert_eq!(svg, expected);
    }
}