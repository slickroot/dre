use std::borrow::Cow;

use crate::diagram::{children, Node};
use crate::palette::FOREGROUND;
use crate::render::{BOX_FILL_OPACITY, FOOTER_FILL_OPACITY};
use crate::state::FooterView;
use types::Tree;

pub(crate) const BOX_HEIGHT: i64 = 3;
#[allow(dead_code)]
pub(crate) const GAP_HEIGHT: i64 = 3;
pub(crate) const GAP_WIDTH: i64 = 8;
pub(crate) const SIDE_PADDING: i64 = 2;
pub(crate) const BORDER: i64 = 4;
const BORDER_COLUMNS: i64 = 2;

pub type Sides = (bool, bool, bool, bool);
pub const ALL_SIDES: Sides = (true, true, true, true);
pub const NO_SIDES: Sides = (false, false, false, false);
#[allow(dead_code)]
pub(crate) const ROW_PITCH: i64 = BOX_HEIGHT + GAP_HEIGHT;
pub(crate) const HALF_PITCH: i64 = BOX_HEIGHT;
pub(crate) const LEAF_STRIDE: i64 = 2;

pub(crate) fn interior(label: &str) -> i64 {
    (label.chars().count() as i64).max(1)
}

pub(crate) fn width(node: &Node) -> i64 {
    interior(node.label()) + SIDE_PADDING * 2
}

#[allow(dead_code)]
pub(crate) fn height(_node: &Node) -> i64 {
    BOX_HEIGHT
}

pub(crate) fn centre(width: i64, label: &str) -> i64 {
    let leftover = width - BORDER_COLUMNS - interior(label);
    1 + leftover - leftover.div_euclid(2)
}

fn edit_room(path: &[usize], editing: Option<&[usize]>) -> i64 {
    i64::from(editing == Some(path))
}

pub(crate) fn measure_columns(tree: &Tree<Node>, editing: Option<&[usize]>) -> Vec<i64> {
    fn widen(widths: &mut Vec<i64>, col: usize, width: i64) {
        if widths.len() <= col {
            widths.resize(col + 1, 0);
        }
        widths[col] = widths[col].max(width);
    }

    let mut widths = Vec::new();
    for (path, node) in tree.walk() {
        let col = 2 * (path.len() - 1);
        widen(&mut widths, col, width(node) + edit_room(&path, editing));
        if children(tree, &path).next().is_some() {
            widen(&mut widths, col + 1, GAP_WIDTH);
        }
    }
    widths
}

// `offsets` has one more entry than there are columns, so `offsets[col + 1] - offsets[col]` gives column `col`'s width.
pub(crate) fn place<'a>(
    tree: &'a Tree<Node>,
    offsets: &[i64],
    editing: Option<&[usize]>,
) -> Vec<Placement<'a>> {
    fn median(rows: &[usize]) -> usize {
        let middle = rows.len() / 2;
        if rows.len() % 2 == 1 {
            rows[middle]
        } else {
            rows[middle - 1] + 1
        }
    }

    fn emit<'a>(
        node: &'a Node,
        x: i64,
        row: usize,
        width: i64,
        path: &[usize],
        child_rows: &[usize],
        edit_room: i64,
    ) -> Vec<Placement<'a>> {
        let y = row as i64 * HALF_PITCH;
        let fill = node.filled().then_some(node.colour()).flatten();
        let mut placements = vec![Placement {
            node: PlacementNode::Box {
                colour: node.colour(),
                fill,
                opacity: fill.is_some().then_some(BOX_FILL_OPACITY),
                rounded: node.rounded(),
                sides: ALL_SIDES,
                border: BORDER,
            },
            x,
            y,
            width,
            height: BOX_HEIGHT,
        }];

        let start = x + centre(width - edit_room, node.label());
        let middle = y + BOX_HEIGHT / 2;
        placements.push(Placement {
            node: PlacementNode::Label(Label {
                text: Cow::Borrowed(node.label()),
                path: path.to_vec(),
            }),
            x: start,
            y: middle,
            width: interior(node.label()),
            height: 1,
        });

        if !child_rows.is_empty() {
            let child_ys: Vec<i64> = child_rows
                .iter()
                .map(|&row| row as i64 * HALF_PITCH)
                .collect();
            let origin = child_ys[0] + BOX_HEIGHT / 2;
            let stops: Vec<i64> = child_ys
                .iter()
                .map(|&child_y| child_y + BOX_HEIGHT / 2 - origin)
                .collect();
            let shaft = y + BOX_HEIGHT / 2 - origin;
            placements.push(Placement {
                node: PlacementNode::Arrow(Arrow {
                    stops: stops.clone(),
                    shaft,
                }),
                x: x + width,
                y: origin,
                width: GAP_WIDTH,
                height: stops[stops.len() - 1] - stops[0] + 1,
            });
        }

        placements
    }

    fn visit<'a>(
        tree: &'a Tree<Node>,
        col: usize,
        path: Vec<usize>,
        offsets: &[i64],
        editing: Option<&[usize]>,
        free: &mut usize,
    ) -> (Vec<Placement<'a>>, usize) {
        let node = tree.value(&path);
        let x = offsets[col];
        let width = offsets[col + 1] - offsets[col];
        let child_paths: Vec<Vec<usize>> = children(tree, &path).collect();

        if child_paths.is_empty() {
            let row = *free;
            *free += LEAF_STRIDE as usize;
            let placements = emit(node, x, row, width, &path, &[], edit_room(&path, editing));
            return (placements, row);
        }

        let child_col = col + 2;
        let mut child_placements = Vec::new();
        let mut child_rows = Vec::new();
        for child_path in child_paths {
            let (placements, row) = visit(tree, child_col, child_path, offsets, editing, free);
            child_placements.extend(placements);
            child_rows.push(row);
        }
        let row = median(&child_rows);
        let mut placements = emit(
            node,
            x,
            row,
            width,
            &path,
            &child_rows,
            edit_room(&path, editing),
        );
        placements.extend(child_placements);
        (placements, row)
    }

    let mut placements = Vec::new();
    let mut free = 0usize;
    for path in children(tree, &[]) {
        let (node_placements, _) = visit(tree, 0, path, offsets, editing, &mut free);
        placements.extend(node_placements);
    }
    placements
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Label<'a> {
    pub(crate) text: Cow<'a, str>,
    pub(crate) path: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Arrow {
    pub(crate) stops: Vec<i64>,
    pub(crate) shaft: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Cursor;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PlacementNode<'a> {
    Box {
        colour: Option<u8>,
        fill: Option<u8>,
        opacity: Option<f64>,
        rounded: bool,
        sides: Sides,
        border: i64,
    },
    Label(Label<'a>),
    Arrow(Arrow),
    Cursor(Cursor),
    /// A round status light, e.g. the footer's mode LED.
    Led {
        colour: u8,
        lit: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Placement<'a> {
    pub(crate) node: PlacementNode<'a>,
    pub(crate) x: i64,
    pub(crate) y: i64,
    pub(crate) width: i64,
    pub(crate) height: i64,
}

pub(crate) const FOOTER_ROWS: i64 = BOX_HEIGHT;

pub(crate) const LED_GAP: i64 = 2;

pub(crate) fn footer(view: &FooterView) -> Vec<Placement<'static>> {
    let text = view.text.as_str();
    let text_width = text.chars().count() as i64;
    let inset_box_width = interior(text) + SIDE_PADDING * 2;
    let box_width = inset_box_width + LED_GAP;
    let corner_box = PlacementNode::Box {
        colour: None,
        fill: Some(FOREGROUND),
        opacity: Some(FOOTER_FILL_OPACITY),
        rounded: false,
        sides: NO_SIDES,
        border: 1,
    };
    let led = PlacementNode::Box {
        colour: Some(view.led_colour),
        fill: Some(view.led_colour),
        opacity: Some(if view.lit { 1.0 } else { FOOTER_FILL_OPACITY }),
        rounded: true,
        sides: ALL_SIDES,
        border: 0,
    };
    let label = PlacementNode::Label(Label {
        text: Cow::Owned(text.to_string()),
        path: vec![],
    });
    let led_x = centre(inset_box_width, text);
    let label_x = led_x + LED_GAP;
    let placements = vec![
        Placement {
            node: corner_box,
            x: 0,
            y: 0,
            width: box_width,
            height: BOX_HEIGHT,
        },
        Placement {
            node: led,
            x: led_x,
            y: BOX_HEIGHT / 2,
            width: 1,
            height: 1,
        },
        Placement {
            node: label,
            x: label_x,
            y: BOX_HEIGHT / 2,
            width: text_width,
            height: 1,
        },
    ];
    with_cursor(placements, view.cursor.map(|_| Vec::new()), view.cursor)
}

pub(crate) fn diagram<'a>(tree: &'a Tree<Node>, editing: Option<&[usize]>) -> Vec<Placement<'a>> {
    if !tree.contains(&[0]) {
        return Vec::new();
    }

    let widths = measure_columns(tree, editing);

    let mut offsets = Vec::with_capacity(widths.len() + 1);
    let mut offset = 0;
    for w in &widths {
        offsets.push(offset);
        offset += w;
    }
    offsets.push(offset);

    let placements = place(tree, &offsets, editing);

    let mut boxes_first: Vec<Placement<'a>> = placements
        .iter()
        .filter(|placement| matches!(placement.node, PlacementNode::Box { .. }))
        .cloned()
        .collect();
    let mut rest: Vec<Placement<'a>> = placements
        .into_iter()
        .filter(|placement| !matches!(placement.node, PlacementNode::Box { .. }))
        .collect();
    boxes_first.append(&mut rest);
    boxes_first
}

pub(crate) fn with_cursor<'a>(
    placements: Vec<Placement<'a>>,
    selected: Option<Vec<usize>>,
    edit_index: Option<usize>,
) -> Vec<Placement<'a>> {
    for placement in &placements {
        if let PlacementNode::Label(label) = &placement.node {
            if Some(&label.path) == selected.as_ref() {
                let mut result = placements.clone();
                result.push(Placement {
                    node: PlacementNode::Cursor(Cursor),
                    x: placement.x + edit_index.map_or(placement.width - 1, |index| index as i64),
                    y: placement.y,
                    width: 1,
                    height: 1,
                });
                return result;
            }
        }
    }
    placements
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{labelled, node, node_with_children};
    use crate::palette;

    fn footer_view(text: &str, led_colour: u8, lit: bool, cursor: Option<usize>) -> FooterView {
        FooterView {
            led_colour,
            lit,
            text: text.to_string(),
            cursor,
        }
    }

    #[test]
    fn footer_box_is_borderless_and_tinted_with_the_foreground_colour() {
        let text = "MOVE plans \u{2022} dre";
        let text_width = text.chars().count() as i64;
        let view = footer_view(text, palette::LIME, false, None);
        let placements = footer(&view);
        assert_eq!(placements.len(), 3);
        let inset_box_width = interior(text) + SIDE_PADDING * 2;
        let box_width = inset_box_width + LED_GAP;
        assert_eq!(
            placements[0].node,
            PlacementNode::Box {
                colour: None,
                fill: Some(FOREGROUND),
                opacity: Some(FOOTER_FILL_OPACITY),
                rounded: false,
                sides: NO_SIDES,
                border: 1,
            }
        );
        assert_eq!(
            (
                placements[0].x,
                placements[0].y,
                placements[0].width,
                placements[0].height,
            ),
            (0, 0, box_width, BOX_HEIGHT)
        );

        let led_x = centre(inset_box_width, text);
        assert_eq!(
            placements[1].node,
            PlacementNode::Box {
                colour: Some(palette::LIME),
                fill: Some(palette::LIME),
                opacity: Some(FOOTER_FILL_OPACITY),
                rounded: true,
                sides: ALL_SIDES,
                border: 0,
            }
        );
        assert_eq!(
            (
                placements[1].x,
                placements[1].y,
                placements[1].width,
                placements[1].height,
            ),
            (led_x, BOX_HEIGHT / 2, 1, 1)
        );

        assert_eq!(
            placements[2].node,
            PlacementNode::Label(Label {
                text: text.into(),
                path: vec![]
            })
        );
        assert_eq!(
            (
                placements[2].x,
                placements[2].y,
                placements[2].width,
                placements[2].height,
            ),
            (led_x + LED_GAP, BOX_HEIGHT / 2, text_width, 1)
        );
    }

    #[test]
    fn a_lit_led_uses_full_opacity() {
        let view = footer_view("WRITE plans \u{2022} dre", palette::VIOLET, true, None);
        let placements = footer(&view);
        match &placements[1].node {
            PlacementNode::Box {
                opacity, colour, ..
            } => {
                assert_eq!(*opacity, Some(1.0));
                assert_eq!(*colour, Some(palette::VIOLET));
            }
            _ => panic!("expected the second placement to be the led box"),
        }
    }

    #[test]
    fn footer_with_no_cursor_has_no_cursor_placement() {
        let view = footer_view("MOVE plans \u{2022} dre", palette::LIME, false, None);
        let placements = footer(&view);
        assert!(placements
            .iter()
            .all(|p| !matches!(p.node, PlacementNode::Cursor(_))));
    }

    #[test]
    fn footer_with_a_cursor_places_it_at_the_edit_index_on_the_label() {
        let text = "MOVE type a name \u{2022} dre";
        let view = footer_view(text, palette::LIME, false, Some(3));
        let placements = footer(&view);
        let label = placements
            .iter()
            .find(|p| matches!(p.node, PlacementNode::Label(_)))
            .expect("the footer has a label placement");
        let cursor = placements
            .iter()
            .find(|p| matches!(p.node, PlacementNode::Cursor(_)))
            .expect("a cursor is present when view.cursor is Some");
        assert_eq!(cursor.x, label.x + 3);
        assert_eq!(cursor.y, label.y);
    }

    fn offsets_for(nodes: &Tree<Node>) -> Vec<i64> {
        let widths = measure_columns(nodes, None);
        let mut offsets = Vec::with_capacity(widths.len() + 1);
        let mut offset = 0;
        for w in &widths {
            offsets.push(offset);
            offset += w;
        }
        offsets.push(offset);
        offsets
    }

    #[test]
    fn measure_columns_of_leaf_only_forest_is_one_column_of_the_max_width() {
        let nodes = Tree::root(vec![node("aa"), node("b")]);
        let widths = measure_columns(&nodes, None);
        assert_eq!(widths, vec![width(nodes.value(&[0]))]);
    }

    #[test]
    fn measure_columns_of_a_parent_and_child_has_parent_gap_child_widths() {
        let nodes = Tree::root(vec![node_with_children("parent", vec![node("a")])]);
        let widths = measure_columns(&nodes, None);
        assert_eq!(
            widths,
            vec![
                width(nodes.value(&[0])),
                GAP_WIDTH,
                width(nodes.value(&[0, 0]))
            ]
        );
    }

    #[test]
    fn measure_columns_aligns_columns_across_multiple_top_level_trees() {
        let nodes = Tree::root(vec![
            node("a"),
            node_with_children("bb", vec![node("ccc")]),
            node("d"),
        ]);
        let widths = measure_columns(&nodes, None);
        let expected_col0 = width(nodes.value(&[0]))
            .max(width(nodes.value(&[1])))
            .max(width(nodes.value(&[2])));
        assert_eq!(
            widths,
            vec![expected_col0, GAP_WIDTH, width(nodes.value(&[1, 0]))]
        );
    }

    #[test]
    fn measure_columns_of_no_nodes_is_empty() {
        let widths = measure_columns(&Tree::root(vec![]), None);
        assert_eq!(widths, Vec::<i64>::new());
    }

    #[test]
    fn interior_is_label_length() {
        assert_eq!(interior("hello"), 5);
    }

    #[test]
    fn interior_treats_empty_label_as_width_one() {
        assert_eq!(interior(""), 1);
    }

    #[test]
    fn width_is_interior_plus_side_padding() {
        assert_eq!(width(&labelled("hi")), 2 + SIDE_PADDING * 2);
    }

    #[test]
    fn width_of_empty_label_box_is_one_plus_side_padding() {
        assert_eq!(width(&labelled("")), 1 + SIDE_PADDING * 2);
    }

    #[test]
    fn height_is_always_box_height() {
        assert_eq!(height(&labelled("anything")), BOX_HEIGHT);
        assert_eq!(height(&labelled("")), BOX_HEIGHT);
    }

    #[test]
    fn centre_centers_the_label_within_the_box() {
        assert_eq!(centre(7, "hi"), 3);
    }

    #[test]
    fn centre_of_a_tightly_fit_label_is_one() {
        assert_eq!(centre(2 + BORDER_COLUMNS, "hi"), 1);
    }

    #[test]
    fn centre_of_a_boxs_minimum_width_pads_both_sides_evenly() {
        let width = interior("hi") + SIDE_PADDING * 2;
        let start = centre(width, "hi");
        let left_padding = start - 1;
        let right_padding = width - 1 - interior("hi") - start;
        assert_eq!(left_padding, right_padding);
        assert_eq!(left_padding, SIDE_PADDING - 1);
    }

    #[test]
    fn place_places_a_node_using_its_offset_and_row() {
        let nodes = Tree::root(vec![node("hi")]);
        let offsets = offsets_for(&nodes);
        let placements = place(&nodes, &offsets, None);

        let box_placement = &placements[0];
        match &box_placement.node {
            PlacementNode::Box { .. } => {}
            _ => panic!("expected the first placement to be the box"),
        }
        assert_eq!(box_placement.x, offsets[0]);
        assert_eq!(box_placement.y, 0);
        assert_eq!(box_placement.width, offsets[1] - offsets[0]);
        assert_eq!(box_placement.height, BOX_HEIGHT);
    }

    #[test]
    fn place_recurses_into_children_building_correct_paths() {
        let nodes = Tree::root(vec![node_with_children(
            "parent",
            vec![node("a"), node("b")],
        )]);
        let offsets = offsets_for(&nodes);
        let placements = place(&nodes, &offsets, None);

        let paths: Vec<Vec<usize>> = placements
            .iter()
            .filter_map(|placement| match &placement.node {
                PlacementNode::Label(label) => Some(label.path.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(paths, vec![vec![0], vec![0, 0], vec![0, 1],]);
    }

    #[test]
    fn place_of_a_leaf_yields_only_a_box_and_a_label_placement() {
        let nodes = Tree::root(vec![node("hi")]);
        let offsets = offsets_for(&nodes);
        let placements = place(&nodes, &offsets, None);

        assert_eq!(placements.len(), 2);
        match &placements[0].node {
            PlacementNode::Box { .. } => {}
            _ => panic!("expected the first placement to be the box"),
        }
        match &placements[1].node {
            PlacementNode::Label(label) => {
                assert_eq!(label.text, "hi");
                assert_eq!(label.path, vec![0]);
            }
            _ => panic!("expected the second placement to be a label"),
        }
    }

    #[test]
    fn place_of_a_parent_yields_a_third_arrow_placement_with_stops_and_shaft() {
        let nodes = Tree::root(vec![node_with_children(
            "parent",
            vec![node("a"), node("b")],
        )]);
        let offsets = offsets_for(&nodes);
        let placements = place(&nodes, &offsets, None);

        assert_eq!(placements.len(), 7);

        let child_a_y = 0;
        let child_b_y = LEAF_STRIDE * HALF_PITCH;
        let origin = child_a_y + BOX_HEIGHT / 2;
        let parent_row: i64 = 1;
        let parent_y = parent_row * HALF_PITCH;
        let expected_stops = vec![
            child_a_y + BOX_HEIGHT / 2 - origin,
            child_b_y + BOX_HEIGHT / 2 - origin,
        ];
        let expected_shaft = parent_y + BOX_HEIGHT / 2 - origin;

        let arrow = placements
            .iter()
            .find_map(|placement| match &placement.node {
                PlacementNode::Arrow(arrow) => Some(arrow.clone()),
                _ => None,
            })
            .expect("a parent yields an arrow placement");
        assert_eq!(arrow.stops, expected_stops);
        assert_eq!(arrow.shaft, expected_shaft);
    }

    fn box_labelled<'a>(placements: &'a [Placement<'a>], text: &str) -> &'a Placement<'a> {
        let label = placements
            .iter()
            .position(|placement| matches!(&placement.node, PlacementNode::Label(label) if label.text == text))
            .expect("the box has a label placement");
        let box_placement = &placements[label - 1];
        assert!(matches!(box_placement.node, PlacementNode::Box { .. }));
        box_placement
    }

    #[test]
    fn place_centres_a_label_on_its_boxs_midline() {
        let nodes = Tree::root(vec![node("hi"), node("a wider label")]);
        let offsets = offsets_for(&nodes);
        let placements = place(&nodes, &offsets, None);

        let box_placement = box_labelled(&placements, "hi");
        let label = placements
            .iter()
            .find(|placement| matches!(&placement.node, PlacementNode::Label(label) if label.text == "hi"))
            .expect("the box has a label placement");
        assert!(box_placement.width > width(&labelled("hi")));
        assert_eq!(label.x, box_placement.x + centre(box_placement.width, "hi"));
        assert_eq!(label.y, box_placement.y + BOX_HEIGHT / 2);
        assert_eq!(label.width, interior("hi"));
        assert_eq!(label.height, 1);
    }

    #[test]
    fn place_assigns_a_parents_row_as_the_middle_childs_row_when_odd() {
        let nodes = Tree::root(vec![node_with_children(
            "parent",
            vec![node("a"), node("b"), node("c")],
        )]);
        let offsets = offsets_for(&nodes);
        let placements = place(&nodes, &offsets, None);

        let parent_box = box_labelled(&placements, "parent");
        assert_eq!(parent_box.y, LEAF_STRIDE * HALF_PITCH);
    }

    #[test]
    fn place_assigns_a_parents_row_one_past_the_row_before_the_middle_when_even() {
        let nodes = Tree::root(vec![node_with_children(
            "parent",
            vec![node("a"), node("b")],
        )]);
        let offsets = offsets_for(&nodes);
        let placements = place(&nodes, &offsets, None);

        let parent_box = box_labelled(&placements, "parent");
        assert_eq!(parent_box.y, HALF_PITCH);
    }

    fn laid_out_box(colour: Option<u8>, filled: bool, rounded: bool) -> PlacementNode<'static> {
        let nodes = Tree::root(vec![Tree::leaf(
            labelled("hi")
                .with_colour(colour)
                .with_fill(filled)
                .with_rounded(rounded),
        )]);
        match diagram(&nodes, None)[0].node {
            PlacementNode::Box {
                colour,
                fill,
                opacity,
                rounded,
                sides,
                border,
            } => PlacementNode::Box {
                colour,
                fill,
                opacity,
                rounded,
                sides,
                border,
            },
            _ => panic!("the first placement is the box"),
        }
    }

    fn opacity_of(node: &PlacementNode) -> Option<f64> {
        match node {
            PlacementNode::Box { opacity, .. } => *opacity,
            _ => panic!("expected a Box"),
        }
    }

    #[test]
    fn layout_carries_a_boxs_colour_and_rounding() {
        assert_eq!(
            laid_out_box(Some(3), false, true),
            PlacementNode::Box {
                colour: Some(3),
                fill: None,
                opacity: None,
                rounded: true,
                sides: ALL_SIDES,
                border: BORDER,
            }
        );
    }

    #[test]
    fn a_filled_coloured_box_has_the_box_fill_opacity() {
        assert_eq!(
            opacity_of(&laid_out_box(Some(2), true, false)),
            Some(BOX_FILL_OPACITY)
        );
    }

    #[test]
    fn an_unfilled_box_has_no_opacity() {
        assert_eq!(opacity_of(&laid_out_box(Some(2), false, false)), None);
    }

    #[test]
    fn diagram_boxes_have_all_sides_and_the_border_width() {
        let nodes = Tree::root(vec![Tree::leaf(labelled("hi"))]);
        let placements = diagram(&nodes, None);
        let boxes: Vec<_> = placements
            .iter()
            .filter_map(|placement| match placement.node {
                PlacementNode::Box { sides, border, .. } => Some((sides, border)),
                _ => None,
            })
            .collect();
        assert!(!boxes.is_empty());
        assert!(boxes.iter().all(|&found| found == (ALL_SIDES, BORDER)));
    }

    #[test]
    fn a_filled_coloured_box_is_filled_with_its_colour() {
        assert!(matches!(
            laid_out_box(Some(2), true, false),
            PlacementNode::Box { fill: Some(2), .. }
        ));
    }

    #[test]
    fn an_unfilled_box_has_no_fill() {
        assert!(matches!(
            laid_out_box(Some(2), false, false),
            PlacementNode::Box { fill: None, .. }
        ));
    }

    #[test]
    fn a_filled_box_without_a_colour_has_no_fill() {
        assert!(matches!(
            laid_out_box(None, true, false),
            PlacementNode::Box { fill: None, .. }
        ));
    }

    #[test]
    fn layout_of_no_boxes_is_empty() {
        assert_eq!(diagram(&Tree::root(vec![]), None), vec![]);
    }

    #[test]
    fn layout_of_a_single_leaf_box_starts_at_the_origin() {
        let nodes = Tree::root(vec![node("hi")]);
        let placements = diagram(&nodes, None);
        let box_placement = placements[0].clone();
        assert!(matches!(box_placement.node, PlacementNode::Box { .. }));
        assert_eq!(box_placement.x, 0);
        assert_eq!(box_placement.y, 0);
    }

    #[test]
    fn layout_of_a_single_leaf_box_has_no_arrow_placements() {
        let nodes = Tree::root(vec![node("hi")]);
        let placements = diagram(&nodes, None);
        assert!(placements
            .iter()
            .all(|p| !matches!(p.node, PlacementNode::Arrow(_))));
        assert!(placements
            .iter()
            .any(|p| matches!(p.node, PlacementNode::Box { .. })));
        assert!(placements
            .iter()
            .any(|p| matches!(p.node, PlacementNode::Label(_))));
    }

    #[test]
    fn layout_of_a_parent_and_child_has_an_arrow_placement() {
        let boxes = Tree::root(vec![node_with_children("parent", vec![node("child")])]);
        let placements = diagram(&boxes, None);
        assert!(placements
            .iter()
            .any(|p| matches!(p.node, PlacementNode::Arrow(_))));
    }

    #[test]
    fn layout_draws_boxes_before_labels_and_arrows() {
        let boxes = Tree::root(vec![node_with_children("parent", vec![node("child")])]);
        let placements = diagram(&boxes, None);

        let first_non_box = placements
            .iter()
            .position(|p| !matches!(p.node, PlacementNode::Box { .. }))
            .expect("there is at least one non-box placement");
        assert!(placements[..first_non_box]
            .iter()
            .all(|p| matches!(p.node, PlacementNode::Box { .. })));
    }

    #[test]
    fn with_cursor_appends_a_cursor_when_selected_matches_a_labels_path() {
        let nodes = Tree::root(vec![node("hi")]);
        let placements = diagram(&nodes, None);
        let label = placements
            .iter()
            .find(|p| matches!(p.node, PlacementNode::Label(_)))
            .expect("layout of a leaf box includes a label placement")
            .clone();

        let result = with_cursor(placements.clone(), Some(vec![0]), None);
        assert_eq!(result.len(), placements.len() + 1);
        let cursor = result.last().unwrap();
        assert!(matches!(cursor.node, PlacementNode::Cursor(_)));
        assert_eq!(cursor.x, label.x + label.width - 1);
        assert_eq!(cursor.y, label.y);
    }

    fn box_and_label<'a>(placements: &[Placement<'a>]) -> (Placement<'a>, Placement<'a>) {
        let find = |wanted: fn(&PlacementNode) -> bool| {
            placements.iter().find(|p| wanted(&p.node)).unwrap().clone()
        };
        (
            find(|node| matches!(node, PlacementNode::Box { .. })),
            find(|node| matches!(node, PlacementNode::Label(_))),
        )
    }

    #[test]
    fn the_edited_label_is_one_cell_wider_than_its_text() {
        let nodes = Tree::root(vec![node("hi")]);
        let editing = diagram(&nodes, Some(&[0]));
        let plain = diagram(&nodes, None);
        let (edited_box, _) = box_and_label(&editing);
        let (plain_box, _) = box_and_label(&plain);
        assert_eq!(edited_box.width, plain_box.width + 1);
    }

    #[test]
    fn a_label_that_is_not_edited_is_not_widened() {
        let nodes = Tree::root(vec![node_with_children("hi", vec![node("yo")])]);
        let editing = diagram(&nodes, Some(&[0, 0]));
        let plain = diagram(&nodes, None);
        assert_eq!(editing[0].width, plain[0].width);
    }

    #[test]
    fn the_widened_width_does_not_change_as_the_cursor_moves() {
        let nodes = Tree::root(vec![node("hi")]);
        let widths: Vec<i64> = (0..=2)
            .map(|cursor| {
                let placements = diagram(&nodes, Some(&[0]));
                let placements = with_cursor(placements, Some(vec![0]), Some(cursor));
                box_and_label(&placements).0.width
            })
            .collect();
        assert!(widths.windows(2).all(|pair| pair[0] == pair[1]));
    }

    #[test]
    fn in_insert_mode_the_cursor_lands_at_the_edit_index() {
        let nodes = Tree::root(vec![node("hi")]);
        let placements = diagram(&nodes, Some(&[0]));
        let (edited_box, label) = box_and_label(&placements);
        for index in 0..=2 {
            let result = with_cursor(placements.clone(), Some(vec![0]), Some(index));
            let cursor = result.last().unwrap();
            assert!(matches!(cursor.node, PlacementNode::Cursor(_)));
            assert_eq!(cursor.x, label.x + index as i64);
            assert!(cursor.x < edited_box.x + edited_box.width - SIDE_PADDING + 1);
        }
    }

    #[test]
    fn in_command_mode_the_cursor_stays_on_the_last_letter_of_an_unwidened_label() {
        let nodes = Tree::root(vec![node("hi")]);
        let placements = diagram(&nodes, None);
        let (plain_box, label) = box_and_label(&placements);
        assert_eq!(plain_box.width, width(nodes.value(&[0])));
        let result = with_cursor(placements, Some(vec![0]), None);
        assert_eq!(result.last().unwrap().x, label.x + label.width - 1);
    }

    #[test]
    fn with_cursor_adds_nothing_without_a_selection() {
        let nodes = Tree::root(vec![node("hi")]);
        let placements = diagram(&nodes, None);
        assert_eq!(with_cursor(placements.clone(), None, None), placements);
    }

    #[test]
    fn with_cursor_leaves_placements_unchanged_when_nothing_matches() {
        let nodes = Tree::root(vec![node("hi")]);
        let placements = diagram(&nodes, None);
        let result = with_cursor(placements.clone(), Some(vec![99]), None);
        assert_eq!(result, placements);
    }

    #[test]
    fn label_constructor_defaults_path_to_empty() {
        let label = Label {
            text: "hi".into(),
            path: vec![0],
        };
        assert_eq!(label.text, "hi");
        assert_eq!(label.path, vec![0]);
    }

    #[test]
    fn arrow_stores_stops_and_shaft_as_plain_fields() {
        let arrow = Arrow {
            stops: vec![1, 2, 3],
            shaft: 5,
        };
        assert_eq!(arrow.stops, vec![1, 2, 3]);
        assert_eq!(arrow.shaft, 5);
    }

    #[test]
    fn placement_node_holds_the_matching_variants_inner_value() {
        let box_placement = Placement {
            node: PlacementNode::Box {
                colour: Some(1),
                fill: Some(1),
                opacity: Some(BOX_FILL_OPACITY),
                rounded: true,
                sides: ALL_SIDES,
                border: BORDER,
            },
            x: 0,
            y: 0,
            width: 3,
            height: 3,
        };
        match box_placement.node {
            PlacementNode::Box {
                colour,
                fill,
                rounded,
                ..
            } => assert_eq!((colour, fill, rounded), (Some(1), Some(1), true)),
            _ => panic!("expected a Box variant"),
        }

        let label_placement = Placement {
            node: PlacementNode::Label(Label {
                text: "a".into(),
                path: vec![0],
            }),
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        match label_placement.node {
            PlacementNode::Label(label) => {
                assert_eq!(
                    label,
                    Label {
                        text: "a".into(),
                        path: vec![0],
                    }
                )
            }
            _ => panic!("expected a Label variant"),
        }

        let arrow_placement = Placement {
            node: PlacementNode::Arrow(Arrow {
                stops: vec![0],
                shaft: 0,
            }),
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        match arrow_placement.node {
            PlacementNode::Arrow(arrow) => assert_eq!(
                arrow,
                Arrow {
                    stops: vec![0],
                    shaft: 0
                }
            ),
            _ => panic!("expected an Arrow variant"),
        }

        let cursor_placement = Placement {
            node: PlacementNode::Cursor(Cursor),
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        assert!(matches!(cursor_placement.node, PlacementNode::Cursor(_)));
    }
}
