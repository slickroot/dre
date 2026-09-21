use std::io::{self, Write};

use crate::layout::{layout, with_cursor, PlacementNode};
use super::{arrowhead_depth, arrowhead_slope, colour, Renderer, CELL_HEIGHT, CELL_WIDTH};
use crate::diagram::Document;

const ARROW_STROKE: i64 = 2;
const ARROW_JOIN_OVERLAP: i64 = ARROW_STROKE / 2;
const INK: (u8, u8, u8) = (0, 0, 0);
const MONOSPACE_ADVANCE_RATIO: f64 = 0.6;

fn label_font_size() -> f64 {
    (CELL_WIDTH as f64 / MONOSPACE_ADVANCE_RATIO * 100.0).round() / 100.0
}

#[derive(Default)]
pub struct SvgRenderer {
    canvas: Option<(i64, i64)>,
    extent: Option<(i64, i64)>,
}

impl SvgRenderer {
    pub fn with_canvas(columns: i64, rows: i64) -> Self {
        SvgRenderer { canvas: Some((columns, rows)), extent: None }
    }

    pub fn centered_on(self, width: i64, height: i64) -> Self {
        SvgRenderer { extent: Some((width, height)), ..self }
    }

    fn centering_offset(&self) -> (i64, i64) {
        match (self.canvas, self.extent) {
            (Some((columns, rows)), Some((width, height))) => (
                ((columns - width) / 2).max(0),
                ((rows - height) / 2).max(0),
            ),
            _ => (0, 0),
        }
    }

    fn draw(&self, placements: &[crate::layout::Placement]) -> String {
        let (offset_x, offset_y) = self.centering_offset();
        let shifted: Vec<crate::layout::Placement> = placements
            .iter()
            .map(|placement| crate::layout::Placement {
                x: placement.x + offset_x,
                y: placement.y + offset_y,
                ..placement.clone()
            })
            .collect();
        let placements = shifted.as_slice();
        let (min_x, min_y, span_x, span_y) = match self.canvas {
            Some((columns, rows)) => (0, 0, columns * CELL_WIDTH, rows * CELL_HEIGHT),
            None => view_box(placements),
        };
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{min_x} {min_y} {span_x} {span_y}\">"
        );
        svg.push_str(&style_block());
        svg.push_str(&background_rect(min_x, min_y, span_x, span_y));
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
        for placement in placements {
            if let PlacementNode::Cursor(_) = &placement.node {
                svg.push_str(&cursor_rect(placement));
            }
        }
        svg.push_str("</svg>");
        svg
    }
}

impl Renderer for SvgRenderer {
    fn render(&mut self, doc: &Document, out: &mut impl Write) -> io::Result<()> {
        let placements = with_cursor(layout(&doc.boxes), doc.selected.clone());
        out.write_all(self.draw(&placements).as_bytes())
    }
}

fn style_block() -> String {
    let (r, g, b) = INK;
    format!(
        "<style>svg {{ --ink: rgb({r},{g},{b}); --bg: rgb(255,255,255) }}@media (prefers-color-scheme: dark) {{ svg {{ --ink: rgb(255,255,255); --bg: rgb({r},{g},{b}) }} }}</style>"
    )
}

fn background_rect(min_x: i64, min_y: i64, span_x: i64, span_y: i64) -> String {
    format!(
        "<rect x=\"{min_x}\" y=\"{min_y}\" width=\"{span_x}\" height=\"{span_y}\" fill=\"var(--bg)\"/>"
    )
}

fn cursor_rect(placement: &crate::layout::Placement) -> String {
    format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{CELL_WIDTH}\" height=\"{CELL_HEIGHT}\" fill=\"var(--ink)\"/>",
        placement.x * CELL_WIDTH,
        placement.y * CELL_HEIGHT
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

fn rect(placement: &crate::layout::Placement, node: &crate::diagram::Node) -> String {
    use super::{BORDER, OPAQUE, ROUNDED_RADIUS};
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
        let (fr, fg, fb) = crate::diagram::palette(node.colour.unwrap()).unwrap();
        let opacity = super::FILL_ALPHA as f64 / OPAQUE as f64;
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
        "<text xml:space=\"preserve\" font-family=\"monospace\" font-size=\"{}\" text-anchor=\"start\" dominant-baseline=\"central\" x=\"{}\" y=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\" fill=\"var(--ink)\">{}</text>",
        label_font_size(),
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
    use super::super::arrowhead_depth;
    use super::super::arrowhead_slope;
    use super::super::colour;
    use super::super::BORDER;
    use super::super::CELL_HEIGHT;
    use super::super::CELL_WIDTH;
    use super::super::FILL_ALPHA;
    use super::super::OPAQUE;
    use super::super::ROUNDED_RADIUS;
    use crate::diagram::{node, node_with_children, palette, Document, Node, Path};

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
    fn a_plain_leaf_box_renders_with_margins_a_transparent_fill_and_no_rounding() {
        let nodes = vec![node("hi")];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer::default().draw(&placements);

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
            .find(|element| element.starts_with("rect ") && !element.contains("var(--bg)"))
            .expect("a plain leaf box renders a rect");
        assert!(rect.contains("fill=\"none\""));
    }

    #[test]
    fn a_coloured_filled_rounded_box_renders_stroke_fill_and_rx() {
        let nodes = vec![boxed("hi", Some(1), true, true)];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer::default().draw(&placements);

        assert!(svg.contains(&format!("stroke=\"{}\"", rgb(colour(Some(1))))));
        assert!(svg.contains(&format!("stroke-width=\"{}\"", BORDER / 2)));
        assert!(svg.contains(&format!("rx=\"{ROUNDED_RADIUS}\"")));
        assert!(svg.contains(&format!("fill=\"{}\"", rgb(palette(1).unwrap()))));
        assert!(svg.contains(&format!("fill-opacity=\"{}\"", fill_opacity())));
    }

    #[test]
    fn empty_placements_render_a_document_with_the_empty_bbox_view_box() {
        let svg = SvgRenderer::default().draw(&[]);

        let expected = view_box(
            -CELL_HEIGHT,
            -CELL_HEIGHT,
            2 * CELL_HEIGHT,
            2 * CELL_HEIGHT,
        );
        assert!(svg.contains(&format!("viewBox=\"{expected}\"")));
        let body = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag");
        let rects: Vec<&str> =
            body.split('<').filter(|element| element.starts_with("rect ")).collect();
        assert_eq!(rects.len(), 1, "only the background rect is drawn on an empty document");
        assert!(rects[0].contains("fill=\"var(--bg)\""), "the rect is the background rect");
        assert!(!svg.contains("stroke="), "no boxes means no strokes");
    }

    #[test]
    fn every_document_paints_a_single_background_rect_filling_the_view_box() {
        let svg = SvgRenderer::default().draw(&[]);

        let (min_x, min_y, span_x, span_y) = super::view_box(&[]);
        let background = format!(
            "<rect x=\"{min_x}\" y=\"{min_y}\" width=\"{span_x}\" height=\"{span_y}\" fill=\"var(--bg)\"/>"
        );
        let style_end = svg.find("</style>").expect("every document has a style block")
            + "</style>".len();
        assert_eq!(
            svg.find(&background).expect("the background rect is emitted"),
            style_end,
            "the background rect immediately follows the style block"
        );

        let body = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag");
        let rects: Vec<&str> =
            body.split('<').filter(|element| element.starts_with("rect ")).collect();
        assert_eq!(rects.len(), 1, "the background rect is the only rect");
        assert!(rects[0].contains("fill=\"var(--bg)\""));
        assert!(!rects[0].contains("stroke"));
    }

    #[test]
    fn the_background_rect_is_painted_before_defs_boxes_and_labels() {
        let parent = node_with_children("parent", vec![node("a"), node("b")]);
        let nodes = vec![boxed("warn", Some(1), false, false), parent];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer::default().draw(&placements);

        let (min_x, min_y, span_x, span_y) = super::view_box(&placements);
        let background = format!(
            "<rect x=\"{min_x}\" y=\"{min_y}\" width=\"{span_x}\" height=\"{span_y}\" fill=\"var(--bg)\"/>"
        );
        let background_pos =
            svg.find(&background).expect("the background rect is emitted");
        let style_end =
            svg.find("</style>").expect("every document has a style block") + "</style>".len();
        assert_eq!(background_pos, style_end);

        let body = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag");
        let mut rects: Vec<&str> = body
            .split('<')
            .filter(|element| element.starts_with("rect "))
            .collect();
        let background_rect =
            rects.remove(0);
        assert!(background_rect.contains("fill=\"var(--bg)\""));
        assert!(!background_rect.contains("stroke="));
        for rect in &rects {
            assert!(rect.contains("stroke="), "each box rect is stroked and painted atop the background");
        }

        assert!(
            background_pos < svg.find("<defs>").expect("arrows emit defs"),
            "the background rect precedes defs"
        );
        assert!(
            background_pos < svg.find("<text").expect("labels draw text"),
            "the background rect precedes labels"
        );
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

        let svg = SvgRenderer::default().draw(&placements);

        let rects: Vec<&str> = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag")
            .split('<')
            .filter(|element| element.starts_with("rect ") && !element.contains("var(--bg)"))
            .collect();

        assert_eq!(rects.len(), 4);

        for rect in &rects {
            assert!(rect.contains("fill="));
        }

        let (pr, pg, pb) = palette(1).unwrap();
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

        let svg = SvgRenderer::default().draw(&placements);

        let ink_stroke = "stroke=\"var(--ink)\"".to_string();
        let coloured_stroke = format!("stroke=\"{}\"", rgb(palette(2).unwrap()));

        let rects: Vec<&str> = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag")
            .split('<')
            .filter(|element| element.starts_with("rect ") && !element.contains("var(--bg)"))
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
                path: crate::diagram::Path { ancestors: vec![], index: 0 },
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

        let svg = SvgRenderer::default().draw(&placements);

        assert!(svg.contains(&format!(
            "<text xml:space=\"preserve\" font-family=\"monospace\" font-size=\"{}\" text-anchor=\"start\" dominant-baseline=\"central\" x=\"{}\" y=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\" fill=\"var(--ink)\"",
            label_font_size(),
            label_x * CELL_WIDTH,
            label_y * CELL_HEIGHT + CELL_HEIGHT / 2,
            2 * CELL_WIDTH,
        )));
        assert!(svg.contains(">hi</text>"));
    }

    #[test]
    fn label_with_trailing_space_preserves_whitespace_and_covers_it() {
        let label_x = 3;
        let label_y = 2;
        let placements = vec![label_placement("Pl ", label_x, label_y)];

        let svg = SvgRenderer::default().draw(&placements);

        assert!(svg.contains("<text xml:space=\"preserve\" "));
        assert!(svg.contains(&format!("textLength=\"{}\"", 3 * CELL_WIDTH)));
        assert!(svg.contains(">Pl </text>"));
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

        let svg = SvgRenderer::default().draw(&placements);

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

        let svg = SvgRenderer::default().draw(&placements);

        assert!(svg.contains(">a&lt;b&gt;&amp;c</text>"));
        assert!(!svg.contains("a<b>&c"));
    }

    #[test]
    fn an_arrow_with_two_stops_renders_a_defs_marker_before_boxes_and_labels() {
        let parent = node_with_children("parent", vec![node("a"), node("b")]);
        let nodes = vec![parent];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer::default().draw(&placements);

        let defs = svg.find("<defs>").expect("arrows emit a defs block");
        let box_rect = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag")
            .split('<')
            .find(|element| element.starts_with("rect ") && !element.contains("var(--bg)"))
            .expect("the parent box draws a rect");
        let rect = svg.find(box_rect).expect("the box rect is emitted");
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

        let svg = SvgRenderer::default().draw(&placements);

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

        let svg = SvgRenderer::default().draw(&placements);

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

        let svg = SvgRenderer::default().draw(&placements);

        assert!(!svg.contains("<defs>"));
        assert!(!svg.contains("marker-end"));
        assert!(!svg.contains("arrowhead"));
    }

    fn expected_style() -> String {
        format!(
            "<style>svg {{ --ink: {}; --bg: rgb(255,255,255) }}@media (prefers-color-scheme: dark) {{ svg {{ --ink: rgb(255,255,255); --bg: {} }} }}</style>",
            rgb(INK),
            rgb(INK)
        )
    }

    fn expected_background(min_x: i64, min_y: i64, span_x: i64, span_y: i64) -> String {
        format!(
            "<rect x=\"{min_x}\" y=\"{min_y}\" width=\"{span_x}\" height=\"{span_y}\" fill=\"var(--bg)\"/>"
        )
    }

    #[test]
    fn the_style_block_follows_the_svg_tag_on_every_document() {
        let svg = SvgRenderer::default().draw(&[]);

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
        let svg = SvgRenderer::default().draw(&[]);

        assert!(svg.contains(&format!(
            "svg {{ --ink: {}; --bg: rgb(255,255,255) }}",
            rgb(INK)
        )));
    }

    #[test]
    fn the_style_dark_override_uses_a_media_query_with_white_ink_and_black_bg() {
        let svg = SvgRenderer::default().draw(&[]);

        assert!(svg.contains(&format!(
            "@media (prefers-color-scheme: dark) {{ svg {{ --ink: rgb(255,255,255); --bg: {} }} }}",
            rgb(INK)
        )));
    }

    #[test]
    fn the_style_block_precedes_defs_and_immediately_follows_the_svg_tag() {
        let parent = node_with_children("parent", vec![node("a"), node("b")]);
        let nodes = vec![parent];
        let placements = crate::layout::layout(&nodes);

        let svg = SvgRenderer::default().draw(&placements);

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
            let (fr, fg, fb) = palette(colour_index.unwrap()).unwrap();
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
            "<text xml:space=\"preserve\" font-family=\"monospace\" font-size=\"{}\" text-anchor=\"start\" dominant-baseline=\"central\" x=\"{}\" y=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\" fill=\"var(--ink)\">{text}</text>",
            label_font_size(),
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

        let svg = SvgRenderer::default().draw(&placements);

        let (min_x, min_y, span_x, span_y) = (
            -CELL_HEIGHT,
            -CELL_HEIGHT,
            18 * CELL_WIDTH + 2 * CELL_HEIGHT,
            33 * CELL_HEIGHT + 2 * CELL_HEIGHT,
        );
        let expected = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{}\">{}{}{}{}{}{}</svg>",
            view_box(min_x, min_y, span_x, span_y),
            expected_style(),
            expected_background(min_x, min_y, span_x, span_y),
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

    fn rendered(doc: &Document) -> String {
        let mut out = Vec::new();
        SvgRenderer::default().render(doc, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn rendering_a_document_writes_the_drawing_of_its_layout() {
        let doc = Document {
            boxes: vec![node_with_children("root", vec![node("A"), node("B")])],
            selected: None,
        };

        assert_eq!(rendered(&doc), SvgRenderer::default().draw(&layout(&doc.boxes)));
    }

    fn cursor_rect_at_label_end_of(doc: &Document, selected: &Path) -> String {
        let boxes = layout(&doc.boxes);
        let label = boxes
            .iter()
            .find(|placement| matches!(&placement.node, PlacementNode::Label(label) if &label.path == selected))
            .unwrap();
        cursor_rect_at_cell(label.x + label.width - 1, label.y)
    }

    fn cursor_rect_at_cell(column: i64, row: i64) -> String {
        format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{CELL_WIDTH}\" height=\"{CELL_HEIGHT}\" fill=\"var(--ink)\"/>",
            column * CELL_WIDTH,
            row * CELL_HEIGHT
        )
    }

    fn two_children_document(selected: Option<Path>) -> Document {
        Document {
            boxes: vec![node_with_children("root", vec![node("A"), node("B")])],
            selected,
        }
    }

    #[test]
    fn a_selected_box_shows_the_cursor_at_the_end_of_its_label() {
        let path = Path {
            ancestors: vec![0],
            index: 1,
        };
        let doc = two_children_document(Some(path.clone()));

        assert!(rendered(&doc).contains(&cursor_rect_at_label_end_of(&doc, &path)));
    }

    #[test]
    fn no_selection_shows_no_cursor() {
        let doc = two_children_document(None);

        assert!(!rendered(&doc).contains("fill=\"var(--ink)\"/>"));
    }

    #[test]
    fn moving_the_selection_moves_the_cursor() {
        let first = Path {
            ancestors: vec![0],
            index: 0,
        };
        let second = Path {
            ancestors: vec![0],
            index: 1,
        };
        let doc = two_children_document(Some(second.clone()));
        let first_cursor = cursor_rect_at_label_end_of(&doc, &first);
        let second_cursor = cursor_rect_at_label_end_of(&doc, &second);

        let svg = rendered(&doc);

        assert!(svg.contains(&second_cursor));
        assert!(!svg.contains(&first_cursor));
    }

    #[test]
    fn the_cursor_is_painted_after_the_labels() {
        let placements = vec![
            label_placement("hi", 1, 1),
            crate::layout::Placement {
                node: PlacementNode::Cursor(crate::layout::Cursor),
                x: 2,
                y: 1,
                width: 1,
                height: 1,
            },
        ];

        let svg = SvgRenderer::default().draw(&placements);

        let cursor = svg.find(&cursor_rect_at_cell(2, 1)).unwrap();
        assert!(cursor > svg.find("<text").unwrap());
    }

    #[test]
    fn a_fixed_canvas_sets_the_view_box_regardless_of_content() {
        let expected = format!(
            "viewBox=\"0 0 {} {}\"",
            160 * CELL_WIDTH,
            50 * CELL_HEIGHT
        );
        assert!(SvgRenderer::with_canvas(160, 50).draw(&[]).contains(&expected));
        let background = format!(
            "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\"",
            160 * CELL_WIDTH,
            50 * CELL_HEIGHT
        );
        assert!(SvgRenderer::with_canvas(160, 50).draw(&[]).contains(&background));
    }

    #[test]
    fn without_a_canvas_the_view_box_hugs_the_content() {
        let expected = format!(
            "viewBox=\"{} {} {} {}\"",
            -CELL_HEIGHT,
            -CELL_HEIGHT,
            2 * CELL_HEIGHT,
            2 * CELL_HEIGHT
        );
        assert!(SvgRenderer::default().draw(&[]).contains(&expected));
    }

    fn first_rect_origin(svg: &str) -> (i64, i64) {
        let rect = svg.split("<rect x=\"").nth(2).unwrap();
        let x: i64 = rect.split('"').next().unwrap().parse().unwrap();
        let y: i64 = rect.split("y=\"").nth(1).unwrap().split('"').next().unwrap().parse().unwrap();
        (x, y)
    }

    fn box_origin_when_centered(canvas: (i64, i64), extent: (i64, i64)) -> (i64, i64) {
        let nodes = vec![node("hi")];
        let placements = crate::layout::layout(&nodes);
        let renderer = SvgRenderer::with_canvas(canvas.0, canvas.1).centered_on(extent.0, extent.1);
        first_rect_origin(&renderer.draw(&placements))
    }

    #[test]
    fn an_extent_smaller_than_the_canvas_is_centered_in_it() {
        let uncentered = box_origin_when_centered((100, 40), (100, 40));

        let (x, y) = box_origin_when_centered((100, 40), (30, 10));

        assert_eq!((x - uncentered.0, y - uncentered.1), (35 * CELL_WIDTH, 15 * CELL_HEIGHT));
    }

    #[test]
    fn an_odd_leftover_space_rounds_the_offset_down() {
        let uncentered = box_origin_when_centered((100, 40), (100, 40));

        let (x, y) = box_origin_when_centered((100, 40), (30, 9));

        assert_eq!((x - uncentered.0, y - uncentered.1), (35 * CELL_WIDTH, 15 * CELL_HEIGHT));
    }

    #[test]
    fn an_extent_equal_to_the_canvas_gets_no_offset() {
        let nodes = vec![node("hi")];
        let placements = crate::layout::layout(&nodes);

        let centered = SvgRenderer::with_canvas(100, 40).centered_on(100, 40).draw(&placements);

        assert_eq!(centered, SvgRenderer::with_canvas(100, 40).draw(&placements));
    }

    #[test]
    fn an_extent_larger_than_the_canvas_is_clamped_to_no_offset() {
        let nodes = vec![node("hi")];
        let placements = crate::layout::layout(&nodes);

        let centered = SvgRenderer::with_canvas(100, 40).centered_on(120, 50).draw(&placements);

        assert_eq!(centered, SvgRenderer::with_canvas(100, 40).draw(&placements));
    }

    #[test]
    fn centering_leaves_the_view_box_fixed() {
        let expected = format!("viewBox=\"0 0 {} {}\"", 100 * CELL_WIDTH, 40 * CELL_HEIGHT);

        assert!(SvgRenderer::with_canvas(100, 40).centered_on(30, 10).draw(&[]).contains(&expected));
    }
}

#[cfg(test)]
mod font_size_tests {
    use super::label_font_size;

    #[test]
    fn label_font_size_makes_the_monospace_advance_equal_a_cell_width() {
        assert_eq!(label_font_size(), 13.33);
    }
}
