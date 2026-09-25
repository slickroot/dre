use std::io::{self, Write};

use super::{
    arrowhead_depth, arrowhead_slope, colour, editor, Renderer, ARROW_OPACITY, BORDER, CELL_HEIGHT,
    CELL_WIDTH,
};
use crate::composer::Area;
use crate::layout::{diagram, Placement, PlacementNode, FOOTER_ROWS};
use crate::state::State;

const ARROW_STROKE: i64 = BORDER / 4;
const ARROW_JOIN_OVERLAP: i64 = ARROW_STROKE / 2;
const MONOSPACE_ADVANCE_RATIO: f64 = 0.6;

fn label_font_size() -> f64 {
    (CELL_WIDTH as f64 / MONOSPACE_ADVANCE_RATIO * 100.0).round() / 100.0
}

#[derive(Default)]
pub struct SvgRenderer {
    canvas: Option<(i64, i64)>,
}

impl SvgRenderer {
    pub fn with_canvas(columns: i64, rows: i64) -> Self {
        SvgRenderer {
            canvas: Some((columns, rows)),
        }
    }

    fn window(&self, state: &State) -> Area {
        let (cols, rows) = self.canvas.unwrap_or_else(|| {
            let (width, height) = extent(&diagram(state.doc.tree()));
            (width, height + FOOTER_ROWS)
        });
        Area {
            col: 0,
            row: 0,
            cols,
            rows,
        }
    }
}

impl Renderer for SvgRenderer {
    fn render(&mut self, state: &State, out: &mut impl Write) -> io::Result<()> {
        let window = self.window(state);
        out.write_all(document(window, &editor(state, window)).as_bytes())
    }
}

fn extent(placements: &[Placement]) -> (i64, i64) {
    let width = placements
        .iter()
        .map(|placement| placement.x + placement.width)
        .max()
        .unwrap_or(0);
    let height = placements
        .iter()
        .map(|placement| placement.y + placement.height)
        .max()
        .unwrap_or(0);
    (width, height)
}

fn pixels(area: Area) -> (i64, i64, i64, i64) {
    (
        area.col * CELL_WIDTH,
        area.row * CELL_HEIGHT,
        area.cols * CELL_WIDTH,
        area.rows * CELL_HEIGHT,
    )
}

fn document(window: Area, areas: &[(Area, Vec<Placement>)]) -> String {
    let (x, y, width, height) = pixels(window);
    let mut svg =
        format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{x} {y} {width} {height}\">");
    svg.push_str(&background_rect(x, y, width, height));
    if areas.iter().any(|(_, placements)| {
        placements
            .iter()
            .any(|placement| matches!(placement.node, PlacementNode::Arrow(_)))
    }) {
        svg.push_str(&marker_defs());
    }
    for (area, placements) in areas {
        let (x, y, width, height) = pixels(*area);
        svg.push_str(&format!(
            "<svg x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" viewBox=\"{x} {y} {width} {height}\">"
        ));
        svg.push_str(&paint(placements));
        svg.push_str("</svg>");
    }
    svg.push_str("</svg>");
    svg
}

fn paint(placements: &[Placement]) -> String {
    let mut svg = String::new();
    for placement in placements {
        if let PlacementNode::Arrow(arrow) = &placement.node {
            svg.push_str(&arrow_paths(placement, arrow));
        }
    }
    for placement in placements {
        if let PlacementNode::Box {
            colour,
            fill,
            rounded,
        } = &placement.node
        {
            svg.push_str(&rect(placement, *colour, *fill, *rounded));
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
    svg
}

fn background_rect(min_x: i64, min_y: i64, span_x: i64, span_y: i64) -> String {
    let (r, g, b) = crate::palette::palette(crate::palette::BACKGROUND).unwrap();
    format!(
        "<rect x=\"{min_x}\" y=\"{min_y}\" width=\"{span_x}\" height=\"{span_y}\" fill=\"rgb({r},{g},{b})\"/>"
    )
}

fn cursor_rect(placement: &crate::layout::Placement) -> String {
    let (r, g, b) = colour(None);
    format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{CELL_WIDTH}\" height=\"{CELL_HEIGHT}\" fill=\"rgb({r},{g},{b})\"/>",
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

fn marker_defs() -> String {
    let depth = arrowhead_depth();
    let slope = arrowhead_slope();
    let arm = depth * slope;
    let box_width = depth.ceil() as i64;
    let box_height = (arm * 2.0).ceil() as i64;
    let tip_x = box_width as f64;
    let tip_y = box_height as f64 / 2.0;
    let base_x = box_width as f64 - depth;
    let (r, g, b) = colour(None);
    format!(
        "<defs><marker id=\"arrowhead\" orient=\"auto\" markerUnits=\"userSpaceOnUse\" markerWidth=\"{box_width}\" markerHeight=\"{box_height}\" refX=\"{tip_x}\" refY=\"{tip_y}\" viewBox=\"0 0 {box_width} {box_height}\"><path d=\"M {tip_x} {tip_y} L {base_x} {} M {tip_x} {tip_y} L {base_x} {}\" stroke=\"rgb({r},{g},{b})\" stroke-width=\"{}\" fill=\"none\"/></marker></defs>",
        tip_y - arm,
        tip_y + arm,
        ARROW_STROKE,
    )
}

fn arrow_paths(placement: &crate::layout::Placement, arrow: &crate::layout::Arrow) -> String {
    let left = placement.x * CELL_WIDTH;
    let right = placement.x * CELL_WIDTH + placement.width * CELL_WIDTH - 1;
    let trunk_x = placement.x * CELL_WIDTH + (placement.width * CELL_WIDTH) / 2;
    let shaft_row = (placement.y + arrow.shaft) * CELL_HEIGHT + CELL_HEIGHT / 2;
    let stop_rows: Vec<i64> = arrow
        .stops
        .iter()
        .map(|stop| (placement.y + stop) * CELL_HEIGHT + CELL_HEIGHT / 2)
        .collect();
    let trunk_top = *stop_rows
        .iter()
        .min()
        .expect("an arrow always has at least one stop");
    let trunk_bottom = *stop_rows
        .iter()
        .max()
        .expect("an arrow always has at least one stop");
    let (r, g, b) = colour(None);
    let stroke = format!("rgb({r},{g},{b})");
    let mut paths = format!("<g opacity=\"{ARROW_OPACITY}\">");
    paths.push_str(&format!(
        "<path d=\"M {left} {shaft_row} L {} {shaft_row}\" stroke=\"{stroke}\" stroke-width=\"{}\" fill=\"none\"/>",
        trunk_x + ARROW_JOIN_OVERLAP,
        ARROW_STROKE
    ));
    paths.push_str(&format!(
        "<path d=\"M {trunk_x} {trunk_top} L {trunk_x} {trunk_bottom}\" stroke=\"{stroke}\" stroke-width=\"{}\" fill=\"none\"/>",
        ARROW_STROKE
    ));
    for row in stop_rows {
        paths.push_str(&format!(
            "<path d=\"M {} {row} L {right} {row}\" marker-end=\"url(#arrowhead)\" stroke=\"{stroke}\" stroke-width=\"{}\" fill=\"none\"/>",
            trunk_x - ARROW_JOIN_OVERLAP,
            ARROW_STROKE
        ));
    }
    paths.push_str("</g>");
    paths
}

fn rect(
    placement: &crate::layout::Placement,
    edge: Option<u8>,
    fill: Option<u8>,
    rounded: bool,
) -> String {
    use super::{BORDER, OPAQUE, ROUNDED_RADIUS};
    use std::fmt::Write as _;

    let (r, g, b) = colour(edge);
    let stroke = format!("rgb({r},{g},{b})");
    let mut rect = format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" stroke=\"{stroke}\" stroke-width=\"{}\"",
        placement.x * CELL_WIDTH,
        placement.y * CELL_HEIGHT,
        placement.width * CELL_WIDTH,
        placement.height * CELL_HEIGHT,
        BORDER / 2,
    );
    if rounded {
        write!(rect, " rx=\"{ROUNDED_RADIUS}\"").unwrap();
    }
    if let Some(colour) = fill {
        let (fr, fg, fb) = crate::palette::palette(colour).unwrap();
        let opacity = super::FILL_ALPHA as f64 / OPAQUE as f64;
        write!(
            rect,
            " fill=\"rgb({fr},{fg},{fb})\" fill-opacity=\"{opacity}\""
        )
        .unwrap();
    } else {
        rect.push_str(" fill=\"none\"");
    }
    rect.push_str("/>");
    rect
}

fn label_text(placement: &crate::layout::Placement, label: &crate::layout::Label) -> String {
    let chars = label.text.chars().count() as i64;
    let (r, g, b) = colour(None);
    format!(
        "<text xml:space=\"preserve\" font-family=\"monospace\" font-size=\"{}\" text-anchor=\"start\" dominant-baseline=\"central\" x=\"{}\" y=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\" fill=\"rgb({r},{g},{b})\">{}</text>",
        label_font_size(),
        placement.x * CELL_WIDTH,
        placement.y * CELL_HEIGHT + CELL_HEIGHT / 2,
        chars * CELL_WIDTH,
        escape(label.text),
    )
}

#[cfg(test)]
mod tests {
    use super::super::arrowhead_depth;
    use super::super::arrowhead_slope;
    use super::super::centre;
    use super::super::colour;
    use super::super::BORDER;
    use super::super::CELL_HEIGHT;
    use super::super::CELL_WIDTH;
    use super::super::FILL_ALPHA;
    use super::super::OPAQUE;
    use super::super::ROUNDED_RADIUS;
    use super::*;
    use crate::composer::Area;
    use crate::diagram::{node, node_with_children};
    use crate::layout::with_cursor;
    use crate::layout::{Arrow, Cursor, Label, Placement};
    use crate::palette::{palette, BACKGROUND};
    use crate::state::Mode;

    fn box_placement(
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        colour: Option<u8>,
        fill: Option<u8>,
        rounded: bool,
    ) -> Placement<'static> {
        Placement {
            node: PlacementNode::Box {
                colour,
                fill,
                rounded,
            },
            x,
            y,
            width,
            height,
        }
    }

    fn plain_box(x: i64, y: i64, width: i64, height: i64) -> Placement<'static> {
        box_placement(x, y, width, height, None, None, false)
    }

    fn arrow_placement(
        x: i64,
        y: i64,
        width: i64,
        shaft: i64,
        stops: &[i64],
    ) -> Placement<'static> {
        Placement {
            node: PlacementNode::Arrow(Arrow {
                stops: stops.to_vec(),
                shaft,
            }),
            x,
            y,
            width,
            height: stops[stops.len() - 1] - stops[0] + 1,
        }
    }

    fn a_parent_with_two_children() -> Vec<Placement<'static>> {
        vec![
            plain_box(0, 3, 8, 3),
            plain_box(16, 0, 3, 3),
            plain_box(16, 6, 3, 3),
            label_placement("parent", 1, 4),
            arrow_placement(8, 1, 8, 3, &[0, 6]),
            label_placement("a", 17, 1),
            label_placement("b", 17, 7),
        ]
    }

    fn arrow_of(placements: &[Placement<'static>]) -> (Placement<'static>, Arrow) {
        placements
            .iter()
            .find_map(|placement| match &placement.node {
                PlacementNode::Arrow(arrow) => Some((placement.clone(), arrow.clone())),
                _ => None,
            })
            .expect("the placements include an arrow")
    }

    fn draw(placements: &[Placement]) -> String {
        let (cols, rows) = extent(placements);
        let window = Area {
            col: 0,
            row: 0,
            cols,
            rows,
        };
        document(window, &[(window, placements.to_vec())])
    }

    fn rgb(colour: (u8, u8, u8)) -> String {
        format!("rgb({},{},{})", colour.0, colour.1, colour.2)
    }

    fn background_fill() -> String {
        rgb(palette(BACKGROUND).unwrap())
    }

    fn view_box(min_x: i64, min_y: i64, span_x: i64, span_y: i64) -> String {
        format!("{min_x} {min_y} {span_x} {span_y}")
    }

    fn fill_opacity() -> String {
        format!("{}", FILL_ALPHA as f64 / OPAQUE as f64)
    }

    #[test]
    fn a_plain_leaf_box_renders_a_transparent_fill_and_no_rounding() {
        let (box_width, box_height) = (4, 3);
        let placements = vec![plain_box(0, 0, box_width, box_height)];

        let svg = draw(&placements);

        let expected = view_box(0, 0, box_width * CELL_WIDTH, box_height * CELL_HEIGHT);
        assert!(svg.contains(&format!("viewBox=\"{expected}\"")));
        assert!(svg.contains(&format!(
            "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" stroke=\"{}\" stroke-width=\"{}\" fill=\"none\"/>",
            box_width * CELL_WIDTH,
            box_height * CELL_HEIGHT,
            rgb(colour(None)),
            BORDER / 2,
        )));
        assert!(svg.contains(&format!("stroke=\"{}\"", rgb(colour(None)))));
        assert!(!svg.contains("rx"));
        let rect = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag")
            .split('<')
            .find(|element| element.starts_with("rect ") && !element.contains(&background_fill()))
            .expect("a plain leaf box renders a rect");
        assert!(rect.contains("fill=\"none\""));
    }

    #[test]
    fn a_coloured_filled_rounded_box_renders_stroke_fill_and_rx() {
        let placements = vec![box_placement(0, 0, 4, 3, Some(1), Some(1), true)];

        let svg = draw(&placements);

        assert!(svg.contains(&format!("stroke=\"{}\"", rgb(colour(Some(1))))));
        assert!(svg.contains(&format!("stroke-width=\"{}\"", BORDER / 2)));
        assert!(svg.contains(&format!("rx=\"{ROUNDED_RADIUS}\"")));
        assert!(svg.contains(&format!("fill=\"{}\"", rgb(palette(1).unwrap()))));
        assert!(svg.contains(&format!("fill-opacity=\"{}\"", fill_opacity())));
    }

    #[test]
    fn empty_placements_render_a_document_with_an_empty_view_box() {
        let svg = draw(&[]);

        let expected = view_box(0, 0, 0, 0);
        assert!(svg.contains(&format!("viewBox=\"{expected}\"")));
        let body = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag");
        let rects: Vec<&str> = body
            .split('<')
            .filter(|element| element.starts_with("rect "))
            .collect();
        assert_eq!(
            rects.len(),
            1,
            "only the background rect is drawn on an empty document"
        );
        assert!(
            rects[0].contains(&format!("fill=\"{}\"", background_fill())),
            "the rect is the background rect"
        );
        assert!(!svg.contains("stroke="), "no boxes means no strokes");
    }

    #[test]
    fn every_document_paints_a_single_background_rect_filling_the_view_box() {
        let svg = draw(&[]);

        let background = expected_background(0, 0, 0, 0);
        let opening_end = svg.find('>').expect("the document opens with the svg tag") + 1;
        assert_eq!(
            svg.find(&background)
                .expect("the background rect is emitted"),
            opening_end,
            "the background rect immediately follows the opening svg tag"
        );

        let body = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag");
        let rects: Vec<&str> = body
            .split('<')
            .filter(|element| element.starts_with("rect "))
            .collect();
        assert_eq!(rects.len(), 1, "the background rect is the only rect");
        assert!(rects[0].contains(&format!("fill=\"{}\"", background_fill())));
        assert!(!rects[0].contains("stroke"));
    }

    #[test]
    fn the_background_rect_is_painted_before_defs_boxes_and_labels() {
        let mut placements = vec![box_placement(0, 12, 6, 3, Some(1), None, false)];
        placements.extend(a_parent_with_two_children());

        let svg = draw(&placements);

        let (cols, rows) = extent(&placements);
        let background = expected_background(0, 0, cols * CELL_WIDTH, rows * CELL_HEIGHT);
        let background_pos = svg
            .find(&background)
            .expect("the background rect is emitted");
        let opening_end = svg.find('>').expect("the document opens with the svg tag") + 1;
        assert_eq!(background_pos, opening_end);

        let body = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag");
        let mut rects: Vec<&str> = body
            .split('<')
            .filter(|element| element.starts_with("rect "))
            .collect();
        let background_rect = rects.remove(0);
        assert!(background_rect.contains(&format!("fill=\"{}\"", background_fill())));
        assert!(!background_rect.contains("stroke="));
        for rect in &rects {
            assert!(
                rect.contains("stroke="),
                "each box rect is stroked and painted atop the background"
            );
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
    fn every_box_declares_a_fill_and_only_a_box_with_a_fill_gets_a_colour_fill() {
        let placements = vec![
            box_placement(0, 0, 4, 3, None, None, false),
            box_placement(0, 6, 4, 3, Some(2), None, false),
            box_placement(0, 12, 4, 3, Some(1), Some(1), false),
        ];

        let svg = draw(&placements);

        let rects: Vec<&str> = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag")
            .split('<')
            .filter(|element| element.starts_with("rect ") && !element.contains(&background_fill()))
            .collect();

        assert_eq!(rects.len(), 3);

        for rect in &rects {
            assert!(rect.contains("fill="));
        }

        let (pr, pg, pb) = palette(1).unwrap();
        let palette_fill = format!("fill=\"rgb({pr},{pg},{pb})\"");
        let colour_filled_count = rects.iter().filter(|r| r.contains(&palette_fill)).count();
        assert_eq!(colour_filled_count, 1);

        let none_fill_count = rects.iter().filter(|r| r.contains("fill=\"none\"")).count();
        assert_eq!(none_fill_count, 2);
    }

    #[test]
    fn colourless_boxes_are_foreground_while_coloured_boxes_keep_palette_colours() {
        let placements = vec![
            plain_box(0, 0, 4, 3),
            box_placement(0, 6, 4, 3, Some(2), None, false),
            plain_box(0, 12, 4, 3),
        ];

        let svg = draw(&placements);

        let foreground_stroke = format!("stroke=\"{}\"", rgb(colour(None)));
        let coloured_stroke = format!("stroke=\"{}\"", rgb(palette(2).unwrap()));

        let rects: Vec<&str> = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag")
            .split('<')
            .filter(|element| element.starts_with("rect ") && !element.contains(&background_fill()))
            .collect();

        assert!(rects
            .iter()
            .all(|rect| rect.contains(&foreground_stroke) || rect.contains(&coloured_stroke)));
        assert_eq!(
            rects
                .iter()
                .filter(|rect| rect.contains(&coloured_stroke))
                .count(),
            1
        );
        assert_eq!(
            rects
                .iter()
                .filter(|rect| rect.contains(&foreground_stroke))
                .count(),
            rects.len() - 1
        );
    }

    fn label_placement(text: &str, x: i64, y: i64) -> Placement<'_> {
        Placement {
            node: PlacementNode::Label(Label {
                text,
                path: vec![0],
            }),
            x,
            y,
            width: text.chars().count() as i64,
            height: 1,
        }
    }

    #[test]
    fn a_label_over_a_coloured_box_is_vertically_centred_on_the_box_midline() {
        let label_x = 2;
        let label_y = 1;
        let placements = vec![
            box_placement(0, 0, 4, 3, Some(1), None, false),
            label_placement("hi", label_x, label_y),
        ];

        let svg = draw(&placements);

        assert!(svg.contains(&format!(
            "<text xml:space=\"preserve\" font-family=\"monospace\" font-size=\"{}\" text-anchor=\"start\" dominant-baseline=\"central\" x=\"{}\" y=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\" fill=\"{}\"",
            label_font_size(),
            label_x * CELL_WIDTH,
            label_y * CELL_HEIGHT + CELL_HEIGHT / 2,
            2 * CELL_WIDTH,
            rgb(colour(None)),
        )));
        assert!(svg.contains(">hi</text>"));
    }

    #[test]
    fn label_with_trailing_space_preserves_whitespace_and_covers_it() {
        let label_x = 3;
        let label_y = 2;
        let placements = vec![label_placement("Pl ", label_x, label_y)];

        let svg = draw(&placements);

        assert!(svg.contains("<text xml:space=\"preserve\" "));
        assert!(svg.contains(&format!("textLength=\"{}\"", 3 * CELL_WIDTH)));
        assert!(svg.contains(">Pl </text>"));
    }

    #[test]
    fn boxes_are_drawn_before_labels() {
        let placements = vec![plain_box(0, 0, 4, 3), label_placement("hi", 2, 1)];

        let svg = draw(&placements);

        let rect = svg.find("<rect").expect("a box placement draws a rect");
        let text = svg.find("<text").expect("a label placement draws a text");
        assert!(rect < text);
    }

    #[test]
    fn a_label_with_xml_special_characters_escapes_them_in_the_text_content() {
        let placements = vec![plain_box(0, 0, 4, 3), label_placement("a<b>&c", 2, 1)];

        let svg = draw(&placements);

        assert!(svg.contains(">a&lt;b&gt;&amp;c</text>"));
        assert!(!svg.contains("a<b>&c"));
    }

    #[test]
    fn an_arrow_with_two_stops_renders_a_defs_marker_before_boxes_and_labels() {
        let placements = a_parent_with_two_children();

        let svg = draw(&placements);

        let defs = svg.find("<defs>").expect("arrows emit a defs block");
        let box_rect = svg
            .split("</svg>")
            .next()
            .expect("the document closes the svg tag")
            .split('<')
            .find(|element| element.starts_with("rect ") && !element.contains(&background_fill()))
            .expect("the parent box draws a rect");
        let rect = svg.find(box_rect).expect("the box rect is emitted");
        let arm = svg
            .find("marker-end=\"url(#arrowhead)\"")
            .expect("each stop arm references the arrowhead");
        let text = svg.find("<text").expect("labels draw text");
        assert!(defs < rect, "defs are emitted before the first rect");
        assert!(defs < arm, "defs are emitted before the stop arms");
        assert!(
            arm < rect,
            "stop arms are drawn before the boxes so box borders sit on top"
        );
        assert!(arm < text, "arrow paths are drawn before labels");
        assert!(
            svg.contains("<marker id=\"arrowhead\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"")
        );

        let (arrow_placement, arrow) = arrow_of(&placements);

        let left = arrow_placement.x * CELL_WIDTH;
        let right = arrow_placement.x * CELL_WIDTH + arrow_placement.width * CELL_WIDTH - 1;
        let trunk_x = arrow_placement.x * CELL_WIDTH + (arrow_placement.width * CELL_WIDTH) / 2;
        let shaft_row = (arrow_placement.y + arrow.shaft) * CELL_HEIGHT + CELL_HEIGHT / 2;
        let stop_rows: Vec<i64> = arrow
            .stops
            .iter()
            .map(|stop| (arrow_placement.y + stop) * CELL_HEIGHT + CELL_HEIGHT / 2)
            .collect();
        let trunk_top = *stop_rows
            .iter()
            .min()
            .expect("arrows have at least one stop");
        let trunk_bottom = *stop_rows
            .iter()
            .max()
            .expect("arrows have at least one stop");
        let ink = rgb(colour(None));

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
        let placements = a_parent_with_two_children();

        let svg = draw(&placements);

        let (arrow_placement, arrow) = arrow_of(&placements);

        let left = arrow_placement.x * CELL_WIDTH;
        let right = arrow_placement.x * CELL_WIDTH + arrow_placement.width * CELL_WIDTH - 1;
        let trunk_x = arrow_placement.x * CELL_WIDTH + (arrow_placement.width * CELL_WIDTH) / 2;
        let shaft_row = (arrow_placement.y + arrow.shaft) * CELL_HEIGHT + CELL_HEIGHT / 2;
        let stop_rows: Vec<i64> = arrow
            .stops
            .iter()
            .map(|stop| (arrow_placement.y + stop) * CELL_HEIGHT + CELL_HEIGHT / 2)
            .collect();
        let trunk_top = *stop_rows
            .iter()
            .min()
            .expect("arrows have at least one stop");
        let trunk_bottom = *stop_rows
            .iter()
            .max()
            .expect("arrows have at least one stop");
        let ink = rgb(colour(None));

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
    fn each_arrow_sits_inside_its_own_group_with_the_arrow_opacity() {
        let placements = vec![
            arrow_placement(8, 1, 8, 3, &[0, 6]),
            arrow_placement(8, 13, 8, 3, &[0, 6]),
        ];
        let arrow_count = placements
            .iter()
            .filter(|placement| matches!(placement.node, PlacementNode::Arrow(_)))
            .count();

        let svg = draw(&placements);

        let group_open = format!("<g opacity=\"{ARROW_OPACITY}\">");
        assert_eq!(arrow_count, 2);
        assert_eq!(svg.matches(&group_open).count(), arrow_count);
        assert_eq!(svg.matches("</g>").count(), arrow_count);
        for group in svg.split(&group_open).skip(1) {
            let inside = group.split("</g>").next().expect("groups are closed");
            assert!(inside.starts_with("<path"));
            assert!(!inside.contains("<rect"));
        }
    }

    #[test]
    fn every_arrow_path_is_inside_an_arrow_group() {
        let placements = a_parent_with_two_children();

        let svg = draw(&placements);

        let group_open = format!("<g opacity=\"{ARROW_OPACITY}\">");
        let without_defs = svg.split_once("</defs>").expect("arrows emit defs").1;
        let outside_groups = without_defs
            .split(&group_open)
            .enumerate()
            .map(|(index, part)| {
                if index == 0 {
                    part
                } else {
                    part.split_once("</g>").expect("groups are closed").1
                }
            })
            .collect::<String>();
        assert!(!outside_groups.contains("<path d=\"M"));
    }

    #[test]
    fn arrow_paths_use_the_arrow_stroke_and_box_strokes_keep_the_border_stroke() {
        let placements = a_parent_with_two_children();

        let svg = draw(&placements);

        let group_open = format!("<g opacity=\"{ARROW_OPACITY}\">");
        let group = svg
            .split(&group_open)
            .nth(1)
            .and_then(|part| part.split("</g>").next())
            .expect("the arrow has a group");
        let arrow_stroke = format!("stroke-width=\"{ARROW_STROKE}\"");
        assert_eq!(
            group.matches("<path").count(),
            group.matches(&arrow_stroke).count()
        );
        assert_eq!(ARROW_STROKE, BORDER / 4);
        assert!(svg.contains(&format!("stroke-width=\"{}\"", BORDER / 2)));
    }

    #[test]
    fn the_arrowhead_marker_geometry_is_computed_from_the_mirrored_constants() {
        let placements = a_parent_with_two_children();

        let svg = draw(&placements);

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
        assert!(svg.contains(&format!("stroke=\"{}\"", rgb(colour(None)))));
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
        let placements = vec![plain_box(0, 0, 4, 3), label_placement("hi", 1, 1)];

        let svg = draw(&placements);

        assert!(!svg.contains("<defs>"));
        assert!(!svg.contains("marker-end"));
        assert!(!svg.contains("arrowhead"));
    }

    fn expected_background(min_x: i64, min_y: i64, span_x: i64, span_y: i64) -> String {
        format!(
            "<rect x=\"{min_x}\" y=\"{min_y}\" width=\"{span_x}\" height=\"{span_y}\" fill=\"{}\"/>",
            background_fill()
        )
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
            "<defs><marker id=\"arrowhead\" orient=\"auto\" markerUnits=\"userSpaceOnUse\" markerWidth=\"{box_width}\" markerHeight=\"{box_height}\" refX=\"{tip_x}\" refY=\"{tip_y}\" viewBox=\"0 0 {box_width} {box_height}\"><path d=\"M {tip_x} {tip_y} L {base_x} {} M {tip_x} {tip_y} L {base_x} {}\" stroke=\"{}\" stroke-width=\"{}\" fill=\"none\"/></marker></defs>",
            tip_y - arm,
            tip_y + arm,
            rgb(colour(None)),
            ARROW_STROKE
        )
    }

    fn rect_at(
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        colour_index: Option<u8>,
        fill: Option<u8>,
        rounded: bool,
    ) -> String {
        use std::fmt::Write as _;

        let stroke = rgb(colour(colour_index));
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
        if let Some(fill) = fill {
            let (fr, fg, fb) = palette(fill).unwrap();
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
            "<text xml:space=\"preserve\" font-family=\"monospace\" font-size=\"{}\" text-anchor=\"start\" dominant-baseline=\"central\" x=\"{}\" y=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\" fill=\"{}\">{text}</text>",
            label_font_size(),
            x * CELL_WIDTH,
            y * CELL_HEIGHT + CELL_HEIGHT / 2,
            chars * CELL_WIDTH,
            rgb(colour(None)),
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
        let stroke = rgb(colour(None));
        let mut paths = format!("<g opacity=\"{ARROW_OPACITY}\">");
        paths.push_str(&format!(
            "<path d=\"M {left} {shaft_row} L {} {shaft_row}\" stroke=\"{stroke}\" stroke-width=\"{}\" fill=\"none\"/>",
            trunk_x + ARROW_JOIN_OVERLAP,
            ARROW_STROKE
        ));
        paths.push_str(&format!(
            "<path d=\"M {trunk_x} {trunk_top} L {trunk_x} {trunk_bottom}\" stroke=\"{stroke}\" stroke-width=\"{}\" fill=\"none\"/>",
            ARROW_STROKE
        ));
        for row in stop_rows {
            paths.push_str(&format!(
                "<path d=\"M {} {row} L {right} {row}\" marker-end=\"url(#arrowhead)\" stroke=\"{stroke}\" stroke-width=\"{}\" fill=\"none\"/>",
                trunk_x - ARROW_JOIN_OVERLAP,
                ARROW_STROKE
            ));
        }
        paths.push_str("</g>");
        paths
    }

    #[test]
    fn renders_the_spec_example_diagram_as_a_whole_document() {
        let placements = vec![
            plain_box(0, 0, 7, 3),
            box_placement(0, 6, 7, 3, Some(1), Some(1), true),
            box_placement(0, 12, 7, 3, Some(3), None, false),
            plain_box(0, 24, 7, 3),
            plain_box(15, 18, 3, 3),
            plain_box(15, 24, 3, 3),
            plain_box(15, 30, 3, 3),
            label_placement("start", 1, 1),
            label_placement("greet", 1, 7),
            label_placement("warn", 2, 13),
            label_placement("root", 2, 25),
            arrow_placement(7, 19, 8, 6, &[0, 6, 12]),
            label_placement("A", 16, 19),
            label_placement("B", 16, 25),
            label_placement("C", 16, 31),
        ];

        let window = Area {
            col: 0,
            row: 0,
            cols: 18,
            rows: 33 + FOOTER_ROWS,
        };
        let (body, foot) = body_and_foot(window);
        let footer = vec![label_placement(
            NAME,
            foot.col + foot.cols - name_width(),
            foot.row,
        )];

        let svg = document(window, &[(body, placements), (foot, footer)]);

        let diagram = [
            arrow_at(7, 19, 8, 6, &[0, 6, 12]),
            rect_at(0, 0, 7, 3, None, None, false),
            rect_at(0, 6, 7, 3, Some(1), Some(1), true),
            rect_at(0, 12, 7, 3, Some(3), None, false),
            rect_at(0, 24, 7, 3, None, None, false),
            rect_at(15, 18, 3, 3, None, None, false),
            rect_at(15, 24, 3, 3, None, None, false),
            rect_at(15, 30, 3, 3, None, None, false),
            label_at(1, 1, "start"),
            label_at(1, 7, "greet"),
            label_at(2, 13, "warn"),
            label_at(2, 25, "root"),
            label_at(16, 19, "A"),
            label_at(16, 25, "B"),
            label_at(16, 31, "C"),
        ]
        .concat();
        let expected = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{}\">{}{}{}{}</svg>",
            pixel_box(window),
            expected_background(0, 0, window.cols * CELL_WIDTH, window.rows * CELL_HEIGHT),
            expected_marker(),
            nested(body, &diagram),
            nested(foot, &footer_label(foot)),
        );

        assert_eq!(svg, expected);
    }

    fn rendered(selected: Option<Vec<usize>>) -> String {
        let boxes = vec![node_with_children("root", vec![node("A"), node("B")])];
        let state = crate::state::new_state(boxes, Mode::Command, selected);
        let mut out = Vec::new();
        SvgRenderer::default().render(&state, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn rendering_a_state_draws_its_boxes_labels_and_arrows() {
        let svg = rendered(None);
        let body = svg.split("</svg>").next().unwrap();

        let stroked_rects = body
            .split('<')
            .filter(|element| element.starts_with("rect ") && element.contains("stroke="))
            .count();
        assert_eq!(stroked_rects, 3);
        for label in ["root", "A", "B"] {
            assert!(svg.contains(&format!(">{label}</text>")));
        }
        assert_eq!(
            svg.matches(&format!("<g opacity=\"{ARROW_OPACITY}\">"))
                .count(),
            1
        );
    }

    fn cursor_rect_at_cell(column: i64, row: i64) -> String {
        format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{CELL_WIDTH}\" height=\"{CELL_HEIGHT}\" fill=\"{}\"/>",
            column * CELL_WIDTH,
            row * CELL_HEIGHT,
            rgb(colour(None)),
        )
    }

    fn cursor_rects(svg: &str) -> Vec<&str> {
        let cursor_fill = format!(
            "width=\"{CELL_WIDTH}\" height=\"{CELL_HEIGHT}\" fill=\"{}\"/>",
            rgb(colour(None))
        );
        svg.split('<')
            .filter(|element| element.starts_with("rect ") && element.ends_with(&cursor_fill))
            .collect()
    }

    #[test]
    fn a_selected_box_shows_one_cursor() {
        assert_eq!(cursor_rects(&rendered(Some(vec![0, 1]))).len(), 1);
    }

    #[test]
    fn no_selection_shows_no_cursor() {
        assert!(cursor_rects(&rendered(None)).is_empty());
    }

    #[test]
    fn moving_the_selection_moves_the_cursor() {
        let first = rendered(Some(vec![0, 0]));
        let second = rendered(Some(vec![0, 1]));

        assert_ne!(cursor_rects(&first), cursor_rects(&second));
    }

    #[test]
    fn the_cursor_is_painted_after_the_labels() {
        let placements = vec![
            label_placement("hi", 1, 1),
            Placement {
                node: PlacementNode::Cursor(Cursor),
                x: 2,
                y: 1,
                width: 1,
                height: 1,
            },
        ];

        let svg = draw(&placements);

        let cursor = svg.find(&cursor_rect_at_cell(2, 1)).unwrap();
        assert!(cursor > svg.find("<text").unwrap());
    }

    const CANVAS: Area = Area {
        col: 0,
        row: 0,
        cols: 100,
        rows: 40,
    };

    fn example_state() -> State {
        let boxes = vec![node_with_children("root", vec![node("A"), node("B")])];
        let mut state = crate::state::new_state(boxes, Mode::Command, Some(vec![0, 1]));
        state.save_to = Some(format!("docs/{NAME}.dre"));
        state
    }

    fn render_to_string(mut renderer: SvgRenderer, state: &State) -> String {
        let mut out = Vec::new();
        renderer.render(state, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn pixel_box(area: Area) -> String {
        format!(
            "{} {} {} {}",
            area.col * CELL_WIDTH,
            area.row * CELL_HEIGHT,
            area.cols * CELL_WIDTH,
            area.rows * CELL_HEIGHT
        )
    }

    fn nested(area: Area, contents: &str) -> String {
        format!(
            "<svg x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"{}\">{contents}</svg>",
            area.col * CELL_WIDTH,
            area.row * CELL_HEIGHT,
            area.cols * CELL_WIDTH,
            area.rows * CELL_HEIGHT,
            pixel_box(area)
        )
    }

    fn body_and_foot(window: Area) -> (Area, Area) {
        (
            Area {
                rows: window.rows - FOOTER_ROWS,
                ..window
            },
            Area {
                row: window.row + window.rows - FOOTER_ROWS,
                rows: FOOTER_ROWS,
                ..window
            },
        )
    }

    const NAME: &str = "plans";

    fn name_width() -> i64 {
        NAME.chars().count() as i64
    }

    fn footer_label(foot: Area) -> String {
        label_at(foot.col + foot.cols - name_width(), foot.row, NAME)
    }

    #[test]
    fn a_canvas_is_the_window_in_pixels() {
        let svg = render_to_string(
            SvgRenderer::with_canvas(CANVAS.cols, CANVAS.rows),
            &example_state(),
        );

        assert!(svg.starts_with(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{}\">",
            pixel_box(CANVAS)
        )));
        assert!(svg.contains(&expected_background(
            0,
            0,
            CANVAS.cols * CELL_WIDTH,
            CANVAS.rows * CELL_HEIGHT
        )));
    }

    #[test]
    fn a_canvas_stacks_the_centred_diagram_above_the_footer() {
        let state = example_state();
        let (body, foot) = body_and_foot(CANVAS);

        let svg = render_to_string(SvgRenderer::with_canvas(CANVAS.cols, CANVAS.rows), &state);

        let diagram = centre(
            with_cursor(diagram(state.doc.tree()), state.selected.clone()),
            body,
        );
        let body_svg = nested(body, &paint(&diagram));
        let foot_svg = nested(foot, &footer_label(foot));
        let body_at = svg.find(&body_svg).expect("the body is a nested svg");
        let foot_at = svg.find(&foot_svg).expect("the footer is a nested svg");
        assert!(body_at < foot_at);
        assert!(svg.ends_with(&format!("{foot_svg}</svg>")));
    }

    fn drawing_extent(state: &State) -> (i64, i64) {
        let placements = diagram(state.doc.tree());
        (
            placements.iter().map(|p| p.x + p.width).max().unwrap(),
            placements.iter().map(|p| p.y + p.height).max().unwrap(),
        )
    }

    #[test]
    fn without_a_canvas_the_window_is_the_drawing_plus_the_footer() {
        let state = example_state();
        let (width, height) = drawing_extent(&state);

        let svg = render_to_string(SvgRenderer::default(), &state);

        assert!(svg.starts_with(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\">",
            width * CELL_WIDTH,
            (height + FOOTER_ROWS) * CELL_HEIGHT
        )));
    }

    #[test]
    fn without_a_canvas_the_body_holds_the_whole_diagram_uncropped() {
        let state = example_state();
        let (width, height) = drawing_extent(&state);
        let window = Area {
            col: 0,
            row: 0,
            cols: width,
            rows: height + FOOTER_ROWS,
        };
        let (body, foot) = body_and_foot(window);

        let svg = render_to_string(SvgRenderer::default(), &state);

        let diagram = with_cursor(diagram(state.doc.tree()), state.selected.clone());
        assert!(svg.contains(&nested(body, &paint(&diagram))));
        assert!(svg.contains(&nested(foot, &footer_label(foot))));
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
