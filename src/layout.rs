use crate::diagram::Node;

pub(crate) const BOX_HEIGHT: i64 = 3;
#[allow(dead_code)]
pub(crate) const GAP_HEIGHT: i64 = 3;
pub(crate) const GAP_WIDTH: i64 = 8;
pub(crate) const BORDERS: i64 = 2;
#[allow(dead_code)]
pub(crate) const ROW_PITCH: i64 = BOX_HEIGHT + GAP_HEIGHT;
pub(crate) const HALF_PITCH: i64 = BOX_HEIGHT;
pub(crate) const LEAF_STRIDE: i64 = 2;

pub(crate) fn interior(label: &str) -> i64 {
    (label.chars().count() as i64).max(1)
}

pub(crate) fn width(node: &Node) -> i64 {
    interior(&node.label) + BORDERS
}

#[allow(dead_code)]
pub(crate) fn height(_node: &Node) -> i64 {
    BOX_HEIGHT
}

pub(crate) fn centre(width: i64, label: &str) -> i64 {
    let leftover = width - BORDERS - interior(label);
    1 + leftover - leftover.div_euclid(2)
}

pub(crate) fn measure_columns(nodes: &[Node]) -> Vec<i64> {
    fn visit(node: &Node, col: usize, widths: &mut Vec<i64>) {
        if widths.len() <= col {
            widths.resize(col + 1, 0);
        }
        widths[col] = widths[col].max(width(node));
        if !node.children.is_empty() {
            let gap_col = col + 1;
            if widths.len() <= gap_col {
                widths.resize(gap_col + 1, 0);
            }
            widths[gap_col] = widths[gap_col].max(GAP_WIDTH);
            for child in &node.children {
                visit(child, gap_col + 1, widths);
            }
        }
    }

    let mut widths = Vec::new();
    for node in nodes {
        visit(node, 0, &mut widths);
    }
    widths
}

// `offsets` has one more entry than there are columns, so `offsets[col + 1] - offsets[col]` gives column `col`'s width.
pub(crate) fn place<'a>(nodes: &'a [Node], offsets: &[i64]) -> Vec<Placement<'a>> {
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
    ) -> Vec<Placement<'a>> {
        let y = row as i64 * HALF_PITCH;
        let mut placements = vec![Placement {
            node: PlacementNode::Node(node),
            x,
            y,
            width,
            height: BOX_HEIGHT,
        }];

        let start = x + centre(width, &node.label);
        let middle = y + BOX_HEIGHT / 2;
        placements.push(Placement {
            node: PlacementNode::Label(Label {
                text: &node.label,
                path: path.to_vec(),
                hint: node.hint,
            }),
            x: start,
            y: middle,
            width: interior(&node.label),
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
        node: &'a Node,
        col: usize,
        path: Vec<usize>,
        offsets: &[i64],
        free: &mut usize,
    ) -> (Vec<Placement<'a>>, usize) {
        let x = offsets[col];
        let width = offsets[col + 1] - offsets[col];

        if node.children.is_empty() {
            let row = *free;
            *free += LEAF_STRIDE as usize;
            let placements = emit(node, x, row, width, &path, &[]);
            return (placements, row);
        }

        let child_col = col + 2;
        let mut child_placements = Vec::new();
        let mut child_rows = Vec::new();
        for (index, child) in node.children.iter().enumerate() {
            let child_path = [&path[..], &[index]].concat();
            let (placements, row) = visit(child, child_col, child_path, offsets, free);
            child_placements.extend(placements);
            child_rows.push(row);
        }
        let row = median(&child_rows);
        let mut placements = emit(node, x, row, width, &path, &child_rows);
        placements.extend(child_placements);
        (placements, row)
    }

    let mut placements = Vec::new();
    let mut free = 0usize;
    for (index, node) in nodes.iter().enumerate() {
        let (node_placements, _) = visit(node, 0, vec![index], offsets, &mut free);
        placements.extend(node_placements);
    }
    placements
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Label<'a> {
    pub(crate) text: &'a str,
    pub(crate) path: Vec<usize>,
    pub(crate) hint: bool,
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
    Node(&'a Node),
    Label(Label<'a>),
    Arrow(Arrow),
    Cursor(Cursor),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Placement<'a> {
    pub(crate) node: PlacementNode<'a>,
    pub(crate) x: i64,
    pub(crate) y: i64,
    pub(crate) width: i64,
    pub(crate) height: i64,
}

pub(crate) fn layout<'a>(nodes: &'a [Node]) -> Vec<Placement<'a>> {
    if nodes.is_empty() {
        return Vec::new();
    }

    let widths = measure_columns(nodes);

    let mut offsets = Vec::with_capacity(widths.len() + 1);
    let mut offset = 0;
    for w in &widths {
        offsets.push(offset);
        offset += w;
    }
    offsets.push(offset);

    let placements = place(nodes, &offsets);

    let mut boxes_first: Vec<Placement<'a>> = placements
        .iter()
        .filter(|placement| matches!(placement.node, PlacementNode::Node(_)))
        .cloned()
        .collect();
    let mut rest: Vec<Placement<'a>> = placements
        .into_iter()
        .filter(|placement| !matches!(placement.node, PlacementNode::Node(_)))
        .collect();
    boxes_first.append(&mut rest);
    boxes_first
}

pub(crate) fn with_cursor<'a>(
    placements: Vec<Placement<'a>>,
    selected: Option<Vec<usize>>,
) -> Vec<Placement<'a>> {
    for placement in &placements {
        if let PlacementNode::Label(label) = &placement.node {
            if Some(&label.path) == selected.as_ref() {
                let mut result = placements.clone();
                result.push(Placement {
                    node: PlacementNode::Cursor(Cursor),
                    x: placement.x + placement.width - 1,
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
    use crate::diagram::{node, node_with_children};

    fn offsets_for(nodes: &[Node]) -> Vec<i64> {
        let widths = measure_columns(nodes);
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
        let nodes = vec![node("aa"), node("b")];
        let widths = measure_columns(&nodes);
        assert_eq!(widths, vec![width(&node("aa"))]);
    }

    #[test]
    fn measure_columns_of_a_parent_and_child_has_parent_gap_child_widths() {
        let nodes = vec![node_with_children("parent", vec![node("a")])];
        let widths = measure_columns(&nodes);
        assert_eq!(
            widths,
            vec![width(&node("parent")), GAP_WIDTH, width(&node("a"))]
        );
    }

    #[test]
    fn measure_columns_aligns_columns_across_multiple_top_level_trees() {
        let nodes = vec![
            node("a"),
            node_with_children("bb", vec![node("ccc")]),
            node("d"),
        ];
        let widths = measure_columns(&nodes);
        let expected_col0 = width(&node("a"))
            .max(width(&node("bb")))
            .max(width(&node("d")));
        assert_eq!(widths, vec![expected_col0, GAP_WIDTH, width(&node("ccc"))]);
    }

    #[test]
    fn measure_columns_of_no_nodes_is_empty() {
        let widths = measure_columns(&[]);
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
    fn width_is_interior_plus_borders() {
        assert_eq!(width(&node("hi")), 2 + BORDERS);
    }

    #[test]
    fn width_of_empty_label_box_is_one_plus_borders() {
        assert_eq!(width(&node("")), 1 + BORDERS);
    }

    #[test]
    fn height_is_always_box_height() {
        assert_eq!(height(&node("anything")), BOX_HEIGHT);
        assert_eq!(height(&node("")), BOX_HEIGHT);
    }

    #[test]
    fn centre_centers_the_label_within_the_box() {
        assert_eq!(centre(7, "hi"), 3);
    }

    #[test]
    fn centre_of_a_tightly_fit_label_is_one() {
        assert_eq!(centre(2 + BORDERS, "hi"), 1);
    }

    #[test]
    fn place_places_a_node_using_its_offset_and_row() {
        let hi = node("hi");
        let nodes = vec![hi.clone()];
        let offsets = offsets_for(&nodes);
        let placements = place(&nodes, &offsets);

        let box_placement = &placements[0];
        match &box_placement.node {
            PlacementNode::Node(n) => assert_eq!(*n, &hi),
            _ => panic!("expected the first placement to wrap the node"),
        }
        assert_eq!(box_placement.x, offsets[0]);
        assert_eq!(box_placement.y, 0);
        assert_eq!(box_placement.width, offsets[1] - offsets[0]);
        assert_eq!(box_placement.height, BOX_HEIGHT);
    }

    #[test]
    fn place_recurses_into_children_building_correct_paths() {
        let nodes = vec![node_with_children("parent", vec![node("a"), node("b")])];
        let offsets = offsets_for(&nodes);
        let placements = place(&nodes, &offsets);

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
        let nodes = vec![node("hi")];
        let offsets = offsets_for(&nodes);
        let placements = place(&nodes, &offsets);

        assert_eq!(placements.len(), 2);
        match &placements[0].node {
            PlacementNode::Node(n) => assert_eq!(*n, &node("hi")),
            _ => panic!("expected the first placement to wrap the node"),
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
        let nodes = vec![node_with_children("parent", vec![node("a"), node("b")])];
        let offsets = offsets_for(&nodes);
        let placements = place(&nodes, &offsets);

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

    #[test]
    fn place_assigns_a_parents_row_as_the_middle_childs_row_when_odd() {
        let nodes = vec![node_with_children(
            "parent",
            vec![node("a"), node("b"), node("c")],
        )];
        let offsets = offsets_for(&nodes);
        let placements = place(&nodes, &offsets);

        let parent_box = placements
            .iter()
            .find(|placement| matches!(&placement.node, PlacementNode::Node(n) if n.label == "parent"))
            .expect("the parent has a box placement");
        assert_eq!(parent_box.y, LEAF_STRIDE * HALF_PITCH);
    }

    #[test]
    fn place_assigns_a_parents_row_one_past_the_row_before_the_middle_when_even() {
        let nodes = vec![node_with_children("parent", vec![node("a"), node("b")])];
        let offsets = offsets_for(&nodes);
        let placements = place(&nodes, &offsets);

        let parent_box = placements
            .iter()
            .find(|placement| matches!(&placement.node, PlacementNode::Node(n) if n.label == "parent"))
            .expect("the parent has a box placement");
        assert_eq!(parent_box.y, HALF_PITCH);
    }

    #[test]
    fn layout_of_no_boxes_is_empty() {
        assert_eq!(layout(&[]), vec![]);
    }

    #[test]
    fn layout_of_a_single_leaf_box_starts_at_the_origin() {
        let nodes = vec![node("hi")];
        let placements = layout(&nodes);
        let box_placement = placements[0].clone();
        assert!(matches!(box_placement.node, PlacementNode::Node(_)));
        assert_eq!(box_placement.x, 0);
        assert_eq!(box_placement.y, 0);
    }

    #[test]
    fn layout_of_a_single_leaf_box_has_no_arrow_placements() {
        let nodes = vec![node("hi")];
        let placements = layout(&nodes);
        assert!(placements
            .iter()
            .all(|p| !matches!(p.node, PlacementNode::Arrow(_))));
        assert!(placements
            .iter()
            .any(|p| matches!(p.node, PlacementNode::Node(_))));
        assert!(placements
            .iter()
            .any(|p| matches!(p.node, PlacementNode::Label(_))));
    }

    #[test]
    fn a_hint_node_produces_a_hint_label() {
        let nodes = vec![Node {
            hint: true,
            ..node("hi")
        }];
        let placements = layout(&nodes);
        let label = placements
            .iter()
            .find_map(|p| match &p.node {
                PlacementNode::Label(label) => Some(label),
                _ => None,
            })
            .expect("expected a label placement");
        assert!(label.hint);
    }

    #[test]
    fn layout_of_a_parent_and_child_has_an_arrow_placement() {
        let boxes = vec![node_with_children("parent", vec![node("child")])];
        let placements = layout(&boxes);
        assert!(placements
            .iter()
            .any(|p| matches!(p.node, PlacementNode::Arrow(_))));
    }

    #[test]
    fn layout_draws_boxes_before_labels_and_arrows() {
        let boxes = vec![node_with_children("parent", vec![node("child")])];
        let placements = layout(&boxes);

        let first_non_box = placements
            .iter()
            .position(|p| !matches!(p.node, PlacementNode::Node(_)))
            .expect("there is at least one non-box placement");
        assert!(placements[..first_non_box]
            .iter()
            .all(|p| matches!(p.node, PlacementNode::Node(_))));
    }

    #[test]
    fn with_cursor_appends_a_cursor_when_selected_matches_a_labels_path() {
        let nodes = vec![node("hi")];
        let placements = layout(&nodes);
        let label = placements
            .iter()
            .find(|p| matches!(p.node, PlacementNode::Label(_)))
            .expect("layout of a leaf box includes a label placement")
            .clone();

        let result = with_cursor(placements.clone(), Some(vec![0]));
        assert_eq!(result.len(), placements.len() + 1);
        let cursor = result.last().unwrap();
        assert!(matches!(cursor.node, PlacementNode::Cursor(_)));
        assert_eq!(cursor.x, label.x + label.width - 1);
        assert_eq!(cursor.y, label.y);
    }

    #[test]
    fn with_cursor_leaves_placements_unchanged_when_nothing_matches() {
        let nodes = vec![node("hi")];
        let placements = layout(&nodes);
        let result = with_cursor(placements.clone(), Some(vec![99]));
        assert_eq!(result, placements);
    }

    #[test]
    fn label_constructor_defaults_path_to_empty() {
        let label = Label {
            text: "hi",
            path: vec![0],
            hint: false,
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
        let a = node("a");
        let box_placement = Placement {
            node: PlacementNode::Node(&a),
            x: 0,
            y: 0,
            width: 3,
            height: 3,
        };
        match box_placement.node {
            PlacementNode::Node(n) => assert_eq!(n, &a),
            _ => panic!("expected a Node variant"),
        }

        let label_placement = Placement {
            node: PlacementNode::Label(Label {
                text: "a",
                path: vec![0],
                hint: false,
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
                        text: "a",
                        path: vec![0],
                        hint: false,
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
