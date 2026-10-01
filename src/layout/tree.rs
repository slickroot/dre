use std::borrow::Cow;

use crate::diagram::{children, Node};
use crate::style::{self, BOX_FILL_OPACITY};
use crate::view::{
    self, Arrow, Caret, Label, Placement, PlacementNode, ALL_SIDES, BORDER, BOX_HEIGHT,
    BRACKET_MARGIN, GAP_WIDTH,
};
use types::Tree;

#[cfg(test)]
const BORDER_COLUMNS: i64 = 2;

#[allow(dead_code)]
pub(crate) const GAP_HEIGHT: i64 = 3;
#[allow(dead_code)]
pub(crate) const ROW_PITCH: i64 = BOX_HEIGHT + GAP_HEIGHT;
pub(crate) const HALF_PITCH: i64 = BOX_HEIGHT;
pub(crate) const LEAF_STRIDE: i64 = 2;

// Diagram-node padding, independent of the footer's own per-column padding.
const NODE_PADDING: i64 = 2;

fn width(node: &Node) -> i64 {
    crate::view::interior(node.label()) + NODE_PADDING * 2
}

#[allow(dead_code)]
fn height(_node: &Node) -> i64 {
    BOX_HEIGHT
}

fn edit_room(path: &[usize], editing: Option<(&[usize], usize)>) -> i64 {
    i64::from(editing.map(|(p, _)| p) == Some(path))
}

fn is_selected(path: &[usize], selected: Option<&[usize]>) -> bool {
    selected == Some(path)
}

fn measure_columns(tree: &Tree<Node>, editing: Option<(&[usize], usize)>) -> Vec<i64> {
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
fn place<'a>(
    tree: &'a Tree<Node>,
    offsets: &[i64],
    editing: Option<(&[usize], usize)>,
    selected: Option<&[usize]>,
) -> Vec<Placement<'a>> {
    fn median(rows: &[usize]) -> usize {
        let middle = rows.len() / 2;
        if rows.len() % 2 == 1 {
            rows[middle]
        } else {
            rows[middle - 1] + 1
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit<'a>(
        node: &'a Node,
        x: i64,
        row: usize,
        width: i64,
        child_rows: &[usize],
        edit_room: i64,
        selected: bool,
        caret_index: Option<usize>,
    ) -> Vec<Placement<'a>> {
        let y = row as i64 * HALF_PITCH;
        let fill = node.filled().then_some(node.colour()).flatten();
        let rounded = node.rounded();
        let mut placements = vec![Placement {
            node: PlacementNode::Box {
                colour: style::rgb(node.colour()),
                fill,
                opacity: fill.is_some().then_some(BOX_FILL_OPACITY),
                solid_fill: None,
                rounded,
                sides: ALL_SIDES,
                border: BORDER,
                grow: false,
            },
            x,
            y,
            width,
            height: BOX_HEIGHT,
            depth: 0,
        }];

        if selected {
            placements.push(Placement {
                node: PlacementNode::Brackets { border: BORDER },
                x: x - BRACKET_MARGIN,
                y: y - BRACKET_MARGIN,
                width: width + 2 * BRACKET_MARGIN,
                height: BOX_HEIGHT + 2 * BRACKET_MARGIN,
                depth: 0,
            });
        }

        let start = x + view::label_centre(width - edit_room, node.label());
        let middle = y + BOX_HEIGHT / 2;
        placements.push(Placement {
            node: PlacementNode::Label(Label {
                text: Cow::Borrowed(node.label()),
                colour: style::rgb(None),
                bold: false,
            }),
            x: start,
            y: middle,
            width: view::interior(node.label()),
            height: 1,
            depth: 1,
        });

        if let Some(index) = caret_index {
            placements.push(Placement {
                node: PlacementNode::Caret(Caret),
                x: start + index as i64,
                y: middle,
                width: 1,
                height: 1,
                depth: 0,
            });
        }

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
                depth: 0,
            });
        }

        placements
    }

    #[allow(clippy::too_many_arguments)]
    fn visit<'a>(
        tree: &'a Tree<Node>,
        col: usize,
        path: Vec<usize>,
        offsets: &[i64],
        editing: Option<(&[usize], usize)>,
        selected: Option<&[usize]>,
        free: &mut usize,
    ) -> (Vec<Placement<'a>>, usize) {
        let node = tree.value(&path);
        let x = offsets[col];
        let width = offsets[col + 1] - offsets[col];
        let child_paths: Vec<Vec<usize>> = children(tree, &path).collect();
        let selected_flag = is_selected(&path, selected);
        let caret_index = editing.and_then(|(p, i)| (p == path.as_slice()).then_some(i));

        if child_paths.is_empty() {
            let row = *free;
            *free += LEAF_STRIDE as usize;
            let placements = emit(
                node,
                x,
                row,
                width,
                &[],
                edit_room(&path, editing),
                selected_flag,
                caret_index,
            );
            return (placements, row);
        }

        let child_col = col + 2;
        let mut child_placements = Vec::new();
        let mut child_rows = Vec::new();
        for child_path in child_paths {
            let (placements, row) = visit(
                tree, child_col, child_path, offsets, editing, selected, free,
            );
            child_placements.extend(placements);
            child_rows.push(row);
        }
        let row = median(&child_rows);
        let mut placements = emit(
            node,
            x,
            row,
            width,
            &child_rows,
            edit_room(&path, editing),
            selected_flag,
            caret_index,
        );
        placements.extend(child_placements);
        (placements, row)
    }

    let mut placements = Vec::new();
    let mut free = 0usize;
    for path in children(tree, &[]) {
        let (node_placements, _) = visit(tree, 0, path, offsets, editing, selected, &mut free);
        placements.extend(node_placements);
    }
    placements
}

pub(crate) fn diagram<'a>(
    tree: &'a Tree<Node>,
    editing: Option<(&[usize], usize)>,
    selected: Option<&[usize]>,
) -> Vec<Placement<'a>> {
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

    place(tree, &offsets, editing, selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{labelled, node, node_with_children};
    use crate::view::{
        interior, label_centre as centre, Arrow, Caret, Label, PlacementNode, ALL_SIDES, BORDER,
        BOX_HEIGHT, BRACKET_MARGIN,
    };

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
        assert_eq!(width(&labelled("hi")), 2 + NODE_PADDING * 2);
    }

    #[test]
    fn width_of_empty_label_box_is_one_plus_side_padding() {
        assert_eq!(width(&labelled("")), 1 + NODE_PADDING * 2);
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
        let width = interior("hi") + NODE_PADDING * 2;
        let start = centre(width, "hi");
        let left_padding = start - 1;
        let right_padding = width - 1 - interior("hi") - start;
        assert_eq!(left_padding, right_padding);
        assert_eq!(left_padding, NODE_PADDING - 1);
    }

    #[test]
    fn place_places_a_node_using_its_offset_and_row() {
        let nodes = Tree::root(vec![node("hi")]);
        let offsets = offsets_for(&nodes);
        let placements = place(&nodes, &offsets, None, None);

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
    fn place_of_a_leaf_yields_only_a_box_and_a_label_placement() {
        let nodes = Tree::root(vec![node("hi")]);
        let offsets = offsets_for(&nodes);
        let placements = place(&nodes, &offsets, None, None);

        assert_eq!(placements.len(), 2);
        match &placements[0].node {
            PlacementNode::Box { .. } => {}
            _ => panic!("expected the first placement to be the box"),
        }
        match &placements[1].node {
            PlacementNode::Label(label) => {
                assert_eq!(label.text, "hi");
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
        let placements = place(&nodes, &offsets, None, None);

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
        let placements = place(&nodes, &offsets, None, None);

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
        let placements = place(&nodes, &offsets, None, None);

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
        let placements = place(&nodes, &offsets, None, None);

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
        match diagram(&nodes, None, None)[0].node {
            PlacementNode::Box {
                colour,
                fill,
                opacity,
                solid_fill,
                rounded,
                sides,
                border,
                grow,
            } => PlacementNode::Box {
                colour,
                fill,
                opacity,
                solid_fill,
                rounded,
                sides,
                border,
                grow,
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
                colour: style::rgb(Some(3)),
                fill: None,
                opacity: None,
                solid_fill: None,
                rounded: true,
                sides: ALL_SIDES,
                border: BORDER,
                grow: false,
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
        let placements = diagram(&nodes, None, None);
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
        assert_eq!(diagram(&Tree::root(vec![]), None, None), vec![]);
    }

    #[test]
    fn layout_of_a_single_leaf_box_starts_at_the_origin() {
        let nodes = Tree::root(vec![node("hi")]);
        let placements = diagram(&nodes, None, None);
        let box_placement = placements[0].clone();
        assert!(matches!(box_placement.node, PlacementNode::Box { .. }));
        assert_eq!(box_placement.x, 0);
        assert_eq!(box_placement.y, 0);
    }

    #[test]
    fn layout_of_a_single_leaf_box_has_no_arrow_placements() {
        let nodes = Tree::root(vec![node("hi")]);
        let placements = diagram(&nodes, None, None);
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
        let placements = diagram(&boxes, None, None);
        assert!(placements
            .iter()
            .any(|p| matches!(p.node, PlacementNode::Arrow(_))));
    }

    #[test]
    fn diagram_never_emits_a_selection_state() {
        let nodes = Tree::root(vec![node_with_children(
            "parent",
            vec![node("a"), node("b")],
        )]);
        let placements = diagram(&nodes, None, None);

        assert!(placements
            .iter()
            .all(|p| !matches!(&p.node, PlacementNode::Brackets { .. })));
    }

    #[test]
    fn diagram_emits_brackets_immediately_after_the_selected_box() {
        let nodes = Tree::root(vec![node_with_children(
            "parent",
            vec![node("a"), node("b")],
        )]);
        let placements = diagram(&nodes, None, Some(&[0, 1]));

        let label_index = placements
            .iter()
            .position(|p| matches!(&p.node, PlacementNode::Label(label) if label.text == "b"))
            .expect("the selected node has a label placement");
        let selected_box = &placements[label_index - 2];
        let brackets = &placements[label_index - 1];

        assert!(matches!(selected_box.node, PlacementNode::Box { .. }));
        match brackets.node {
            PlacementNode::Brackets { border } => assert_eq!(border, BORDER),
            _ => panic!("expected brackets right after the selected box"),
        }
        assert_eq!(brackets.x, selected_box.x - BRACKET_MARGIN);
        assert_eq!(brackets.y, selected_box.y - BRACKET_MARGIN);
        assert_eq!(brackets.width, selected_box.width + 2 * BRACKET_MARGIN);
        assert_eq!(brackets.height, selected_box.height + 2 * BRACKET_MARGIN);

        assert_eq!(
            placements
                .iter()
                .filter(|p| matches!(p.node, PlacementNode::Brackets { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn diagram_emits_no_brackets_when_the_selection_matches_no_node() {
        let nodes = Tree::root(vec![node("hi")]);
        let placements = diagram(&nodes, None, Some(&[99]));
        assert!(placements
            .iter()
            .all(|p| !matches!(&p.node, PlacementNode::Brackets { .. })));
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
    fn diagram_emits_a_caret_immediately_after_the_edited_label() {
        let nodes = Tree::root(vec![node("hi")]);
        let placements = diagram(&nodes, Some((&[0], 1)), None);
        let (_, label) = box_and_label(&placements);

        let label_index = placements
            .iter()
            .position(|p| matches!(p.node, PlacementNode::Label(_)))
            .expect("layout of a leaf box includes a label placement");
        let caret = &placements[label_index + 1];
        assert!(matches!(caret.node, PlacementNode::Caret(_)));
        assert_eq!(caret.x, label.x + 1);
        assert_eq!(caret.y, label.y);
    }

    #[test]
    fn the_edited_label_is_one_cell_wider_than_its_text() {
        let nodes = Tree::root(vec![node("hi")]);
        let editing = diagram(&nodes, Some((&[0], 0)), None);
        let plain = diagram(&nodes, None, None);
        let (edited_box, _) = box_and_label(&editing);
        let (plain_box, _) = box_and_label(&plain);
        assert_eq!(edited_box.width, plain_box.width + 1);
    }

    #[test]
    fn a_label_that_is_not_edited_is_not_widened() {
        let nodes = Tree::root(vec![node_with_children("hi", vec![node("yo")])]);
        let editing = diagram(&nodes, Some((&[0, 0], 0)), None);
        let plain = diagram(&nodes, None, None);
        assert_eq!(editing[0].width, plain[0].width);
    }

    #[test]
    fn the_widened_width_does_not_change_as_the_caret_moves() {
        let nodes = Tree::root(vec![node("hi")]);
        let widths: Vec<i64> = (0..=2)
            .map(|index| {
                let placements = diagram(&nodes, Some((&[0], index)), None);
                box_and_label(&placements).0.width
            })
            .collect();
        assert!(widths.windows(2).all(|pair| pair[0] == pair[1]));
    }

    #[test]
    fn in_insert_mode_the_caret_lands_at_the_edit_index() {
        let nodes = Tree::root(vec![node("hi")]);
        for index in 0..=2 {
            let placements = diagram(&nodes, Some((&[0], index)), None);
            let (edited_box, label) = box_and_label(&placements);
            let caret = placements
                .iter()
                .find(|p| matches!(p.node, PlacementNode::Caret(_)))
                .expect("insert mode emits a caret placement");
            assert_eq!(caret.x, label.x + index as i64);
            assert!(caret.x < edited_box.x + edited_box.width - NODE_PADDING + 1);
        }
    }

    #[test]
    fn diagram_emits_no_caret_without_editing() {
        let nodes = Tree::root(vec![node("hi")]);
        let placements = diagram(&nodes, None, None);
        assert!(placements
            .iter()
            .all(|p| !matches!(p.node, PlacementNode::Caret(_))));
    }

    #[test]
    fn diagram_emits_no_caret_when_the_editing_path_matches_no_node() {
        let nodes = Tree::root(vec![node("hi")]);
        let placements = diagram(&nodes, Some((&[99], 0)), None);
        assert!(placements
            .iter()
            .all(|p| !matches!(p.node, PlacementNode::Caret(_))));
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
                colour: style::rgb(Some(1)),
                fill: Some(1),
                opacity: Some(BOX_FILL_OPACITY),
                solid_fill: None,
                rounded: true,
                sides: ALL_SIDES,
                border: BORDER,
                grow: false,
            },
            x: 0,
            y: 0,
            width: 3,
            height: 3,
            depth: 0,
        };
        match box_placement.node {
            PlacementNode::Box {
                colour,
                fill,
                rounded,
                ..
            } => assert_eq!(
                (colour, fill, rounded),
                (style::rgb(Some(1)), Some(1), true)
            ),
            _ => panic!("expected a Box variant"),
        }

        let label_placement = Placement {
            node: PlacementNode::Label(Label {
                text: "a".into(),
                colour: style::rgb(None),
                bold: false,
            }),
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            depth: 1,
        };
        match label_placement.node {
            PlacementNode::Label(label) => {
                assert_eq!(
                    label,
                    Label {
                        text: "a".into(),
                        colour: style::rgb(None),
                        bold: false,
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
            depth: 0,
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

        let caret_placement = Placement {
            node: PlacementNode::Caret(Caret),
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            depth: 0,
        };
        assert!(matches!(caret_placement.node, PlacementNode::Caret(_)));
    }
}
