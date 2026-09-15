use crate::Node;

pub(crate) const BOX_HEIGHT: i64 = 3;
// Unused here too in layout.py; kept for parity with the constant set rather than dropped.
#[allow(dead_code)]
pub(crate) const GAP_HEIGHT: i64 = 3;
pub(crate) const GAP_WIDTH: i64 = 8;
pub(crate) const BORDERS: i64 = 2;
#[allow(dead_code)]
pub(crate) const ROW_PITCH: i64 = BOX_HEIGHT + GAP_HEIGHT;
pub(crate) const HALF_PITCH: i64 = BOX_HEIGHT;
pub(crate) const LEAF_STRIDE: i64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Track {
    pub(crate) offset: i64,
    pub(crate) extent: i64,
}

pub(crate) fn tracks(extents: &[i64], indices: &[i64]) -> Vec<Track> {
    let column_count = *indices.iter().max().expect("indices must not be empty") as usize + 1;
    let mut sizes = vec![0i64; column_count];
    for (&extent, &index) in extents.iter().zip(indices.iter()) {
        let slot = &mut sizes[index as usize];
        *slot = (*slot).max(extent);
    }
    let mut offset = 0;
    let mut laid = Vec::with_capacity(sizes.len());
    for size in sizes {
        laid.push(Track { offset, extent: size });
        offset += size;
    }
    laid
}

pub(crate) fn span(laid: &[Track]) -> i64 {
    laid.iter().map(|track| track.extent).sum()
}

pub(crate) fn interior(label: &str) -> i64 {
    (label.chars().count() as i64).max(1)
}

pub(crate) fn width(box_: &Node) -> i64 {
    interior(&box_.label) + BORDERS
}

// Unused here too in layout.py; kept for parity with the helper set rather than dropped.
#[allow(dead_code)]
pub(crate) fn height(_box_: &Node) -> i64 {
    BOX_HEIGHT
}

pub(crate) fn centre(width: i64, label: &str) -> i64 {
    let leftover = width - BORDERS - interior(label);
    1 + leftover - leftover.div_euclid(2)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Celled {
    pub(crate) box_: Node,
    pub(crate) width: i64,
    pub(crate) column: i64,
    pub(crate) row: i64,
    pub(crate) path: Vec<i64>,
    pub(crate) children: Vec<Celled>,
}

pub(crate) fn anchor(children: &[Celled]) -> i64 {
    let middle = children.len() / 2;
    if children.len() % 2 == 1 {
        children[middle].row
    } else {
        children[middle - 1].row + 1
    }
}

pub(crate) fn assign(box_: &Node, column: i64, path: Vec<i64>, free: i64) -> (Celled, i64) {
    if box_.children.is_empty() {
        let node = Celled {
            box_: box_.clone(),
            width: width(box_),
            column,
            row: free,
            path,
            children: Vec::new(),
        };
        return (node, free + LEAF_STRIDE);
    }
    let mut children = Vec::with_capacity(box_.children.len());
    let mut free = free;
    for (index, child) in box_.children.iter().enumerate() {
        let mut child_path = path.clone();
        child_path.push(index as i64);
        let (node, next_free) = assign(child, column + 1, child_path, free);
        children.push(node);
        free = next_free;
    }
    let row = anchor(&children);
    let node = Celled {
        box_: box_.clone(),
        width: width(box_),
        column,
        row,
        path,
        children,
    };
    (node, free)
}

pub(crate) fn forest(boxes: &[Node]) -> Vec<Celled> {
    let mut trees = Vec::with_capacity(boxes.len());
    let mut free = 0;
    for (index, box_) in boxes.iter().enumerate() {
        let (tree, next_free) = assign(box_, 0, vec![index as i64], free);
        trees.push(tree);
        free = next_free;
    }
    trees
}

pub(crate) fn walk(nodes: &[Celled]) -> Vec<Celled> {
    let mut out = Vec::new();
    for node in nodes {
        out.push(node.clone());
        out.extend(walk(&node.children));
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Positioned {
    pub(crate) box_: Node,
    pub(crate) path: Vec<i64>,
    pub(crate) x: i64,
    pub(crate) y: i64,
    pub(crate) width: i64,
    pub(crate) height: i64,
    pub(crate) children: Vec<Positioned>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Label {
    pub(crate) text: String,
    pub(crate) path: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Arrow {
    pub(crate) stops: Vec<i64>,
    pub(crate) shaft: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Cursor;

// Named Node, not Box, to avoid the std::boxed::Box collision.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PlacementNode {
    Node(Node),
    Label(Label),
    Arrow(Arrow),
    Cursor(Cursor),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Placement {
    pub(crate) node: PlacementNode,
    pub(crate) x: i64,
    pub(crate) y: i64,
    pub(crate) width: i64,
    pub(crate) height: i64,
}

fn position(node: &Celled, columns: &[Track], left: i64, top: i64) -> Positioned {
    let track = columns[(2 * node.column) as usize];
    let x = left + track.offset;
    let y = top + node.row * HALF_PITCH;
    Positioned {
        box_: node.box_.clone(),
        path: node.path.clone(),
        x,
        y,
        width: track.extent,
        height: BOX_HEIGHT,
        children: node
            .children
            .iter()
            .map(|child| position(child, columns, left, top))
            .collect(),
    }
}

fn emit(here: &Positioned, children: &[Positioned]) -> Vec<Placement> {
    let mut placements = vec![Placement {
        node: PlacementNode::Node(here.box_.clone()),
        x: here.x,
        y: here.y,
        width: here.width,
        height: here.height,
    }];

    let start = here.x + centre(here.width, &here.box_.label);
    let middle = here.y + here.height / 2;
    placements.push(Placement {
        node: PlacementNode::Label(Label { text: here.box_.label.clone(), path: here.path.clone() }),
        x: start,
        y: middle,
        width: interior(&here.box_.label),
        height: 1,
    });

    if !children.is_empty() {
        let origin = children[0].y + children[0].height / 2;
        let stops: Vec<i64> = children.iter().map(|child| child.y + child.height / 2 - origin).collect();
        let shaft = here.y + here.height / 2 - origin;
        placements.push(Placement {
            node: PlacementNode::Arrow(Arrow { stops: stops.clone(), shaft }),
            x: here.x + here.width,
            y: origin,
            width: GAP_WIDTH,
            height: stops[stops.len() - 1] - stops[0] + 1,
        });
    }

    placements
}

fn emit_tree(here: &Positioned) -> Vec<Placement> {
    let mut placements = emit(here, &here.children);
    for child in &here.children {
        placements.extend(emit_tree(child));
    }
    placements
}

// Doubles column index into track index, with a gap track after every
// column that has a parent, so an arrow has somewhere to draw.
fn column_tracks(nodes: &[Celled]) -> Vec<Track> {
    let parents: Vec<&Celled> = nodes.iter().filter(|node| !node.children.is_empty()).collect();

    let mut extents: Vec<i64> = nodes.iter().map(|node| node.width).collect();
    extents.extend(parents.iter().map(|_| GAP_WIDTH));

    let mut indices: Vec<i64> = nodes.iter().map(|node| 2 * node.column).collect();
    indices.extend(parents.iter().map(|node| 2 * node.column + 1));

    tracks(&extents, &indices)
}

pub(crate) fn layout(boxes: Vec<Node>, cols: i64, rows: i64) -> Vec<Placement> {
    let trees = forest(&boxes);
    let nodes = walk(&trees);
    if nodes.is_empty() {
        return Vec::new();
    }

    let columns = column_tracks(&nodes);
    let total_height = nodes.iter().map(|node| node.row).max().expect("nodes is non-empty") * HALF_PITCH + BOX_HEIGHT;
    let left = (cols - span(&columns)).div_euclid(2);
    let top = (rows - total_height).div_euclid(2);

    let placements: Vec<Placement> = trees
        .iter()
        .flat_map(|tree| emit_tree(&position(tree, &columns, left, top)))
        .collect();

    // Boxes are opaque, so they are drawn before the arrows and cursor that
    // must show on top of them.
    let mut boxes_first: Vec<Placement> = placements
        .iter()
        .filter(|placement| matches!(placement.node, PlacementNode::Node(_)))
        .cloned()
        .collect();
    let mut rest: Vec<Placement> = placements
        .into_iter()
        .filter(|placement| !matches!(placement.node, PlacementNode::Node(_)))
        .collect();
    boxes_first.append(&mut rest);
    boxes_first
}

pub(crate) fn with_cursor(placements: Vec<Placement>, selected: Vec<i64>) -> Vec<Placement> {
    for placement in &placements {
        if let PlacementNode::Label(label) = &placement.node {
            if label.path == selected {
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

    fn node(label: &str) -> Node {
        Node { label: label.to_string(), ..Default::default() }
    }

    fn node_with_children(label: &str, children: Vec<Node>) -> Node {
        Node { label: label.to_string(), children, ..Default::default() }
    }

    #[test]
    fn tracks_distributes_max_extent_per_column_index() {
        let laid = tracks(&[3, 5, 2], &[0, 0, 1]);
        assert_eq!(
            laid,
            vec![Track { offset: 0, extent: 5 }, Track { offset: 5, extent: 2 }]
        );
    }

    #[test]
    fn tracks_leaves_untouched_columns_at_zero_extent() {
        let laid = tracks(&[4], &[2]);
        assert_eq!(
            laid,
            vec![
                Track { offset: 0, extent: 0 },
                Track { offset: 0, extent: 0 },
                Track { offset: 0, extent: 4 },
            ]
        );
    }

    #[test]
    fn span_sums_track_extents() {
        let laid = vec![Track { offset: 0, extent: 3 }, Track { offset: 3, extent: 5 }];
        assert_eq!(span(&laid), 8);
    }

    #[test]
    fn span_of_no_tracks_is_zero() {
        assert_eq!(span(&[]), 0);
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
    fn anchor_of_odd_children_is_the_middle_childs_row() {
        let children = vec![
            Celled { row: 0, ..leaf_at(0) },
            Celled { row: 2, ..leaf_at(0) },
            Celled { row: 4, ..leaf_at(0) },
        ];
        assert_eq!(anchor(&children), 2);
    }

    #[test]
    fn anchor_of_even_children_is_one_past_the_row_before_the_middle() {
        let children = vec![
            Celled { row: 0, ..leaf_at(0) },
            Celled { row: 2, ..leaf_at(0) },
        ];
        assert_eq!(anchor(&children), 1);
    }

    fn leaf_at(row: i64) -> Celled {
        Celled { box_: node(""), width: 0, column: 0, row, path: vec![], children: vec![] }
    }

    #[test]
    fn assign_of_a_leaf_places_it_at_the_free_row_and_advances_by_leaf_stride() {
        let (cell, next_free) = assign(&node("hi"), 0, vec![0], 5);
        assert_eq!(cell.box_, node("hi"));
        assert_eq!(cell.width, width(&node("hi")));
        assert_eq!(cell.column, 0);
        assert_eq!(cell.row, 5);
        assert_eq!(cell.path, vec![0]);
        assert!(cell.children.is_empty());
        assert_eq!(next_free, 5 + LEAF_STRIDE);
    }

    #[test]
    fn assign_of_a_parent_places_children_in_the_next_column_with_indexed_paths() {
        let tree = node_with_children("parent", vec![node("a"), node("b")]);
        let (cell, next_free) = assign(&tree, 0, vec![3], 0);

        assert_eq!(cell.column, 0);
        assert_eq!(cell.path, vec![3]);
        assert_eq!(cell.children.len(), 2);

        assert_eq!(cell.children[0].column, 1);
        assert_eq!(cell.children[0].path, vec![3, 0]);
        assert_eq!(cell.children[0].row, 0);

        assert_eq!(cell.children[1].column, 1);
        assert_eq!(cell.children[1].path, vec![3, 1]);
        assert_eq!(cell.children[1].row, LEAF_STRIDE);

        assert_eq!(next_free, 2 * LEAF_STRIDE);
        assert_eq!(cell.row, anchor(&cell.children));
    }

    #[test]
    fn forest_threads_the_free_row_counter_across_top_level_trees() {
        let boxes = vec![node("a"), node("b")];
        let trees = forest(&boxes);

        assert_eq!(trees.len(), 2);
        assert_eq!(trees[0].path, vec![0]);
        assert_eq!(trees[0].row, 0);
        assert_eq!(trees[1].path, vec![1]);
        assert_eq!(trees[1].row, LEAF_STRIDE);
    }

    #[test]
    fn forest_threads_free_across_a_multi_child_tree_and_a_following_leaf() {
        let boxes = vec![node_with_children("parent", vec![node("a"), node("b")]), node("c")];
        let trees = forest(&boxes);

        assert_eq!(trees.len(), 2);
        assert_eq!(trees[1].row, 2 * LEAF_STRIDE);
    }

    #[test]
    fn walk_flattens_a_forest_in_pre_order() {
        let boxes = vec![node_with_children("parent", vec![node("a"), node("b")])];
        let trees = forest(&boxes);
        let flat = walk(&trees);

        let paths: Vec<Vec<i64>> = flat.iter().map(|cell| cell.path.clone()).collect();
        assert_eq!(paths, vec![vec![0], vec![0, 0], vec![0, 1]]);
    }

    #[test]
    fn walk_of_multiple_top_level_trees_visits_each_tree_before_its_next_sibling_tree() {
        let boxes = vec![node("a"), node_with_children("b", vec![node("c")])];
        let trees = forest(&boxes);
        let flat = walk(&trees);

        let paths: Vec<Vec<i64>> = flat.iter().map(|cell| cell.path.clone()).collect();
        assert_eq!(paths, vec![vec![0], vec![1], vec![1, 0]]);
    }

    fn columns_for(boxes: &[Node]) -> Vec<Track> {
        let trees = forest(boxes);
        let nodes = walk(&trees);
        column_tracks(&nodes)
    }

    #[test]
    fn column_tracks_has_no_gap_track_for_a_leaf_only_forest() {
        let boxes = vec![node("aa"), node("b")];
        let columns = columns_for(&boxes);
        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].extent, width(&node("aa")));
    }

    #[test]
    fn column_tracks_adds_a_gap_track_after_a_parent_column() {
        let boxes = vec![node_with_children("parent", vec![node("a")])];
        let columns = columns_for(&boxes);
        assert_eq!(columns.len(), 3);
        assert_eq!(columns[1].extent, GAP_WIDTH);
    }

    #[test]
    fn position_places_the_box_using_its_track_and_row() {
        let boxes = vec![node("hi")];
        let trees = forest(&boxes);
        let columns = columns_for(&boxes);
        let positioned = position(&trees[0], &columns, 10, 20);

        assert_eq!(positioned.box_, node("hi"));
        assert_eq!(positioned.path, vec![0]);
        assert_eq!(positioned.x, 10 + columns[0].offset);
        assert_eq!(positioned.y, 20);
        assert_eq!(positioned.width, columns[0].extent);
        assert_eq!(positioned.height, BOX_HEIGHT);
        assert!(positioned.children.is_empty());
    }

    #[test]
    fn position_recurses_into_children() {
        let boxes = vec![node_with_children("parent", vec![node("a"), node("b")])];
        let trees = forest(&boxes);
        let columns = columns_for(&boxes);
        let positioned = position(&trees[0], &columns, 0, 0);

        assert_eq!(positioned.children.len(), 2);
        assert_eq!(positioned.children[0].path, vec![0, 0]);
        assert_eq!(positioned.children[1].path, vec![0, 1]);
    }

    fn leaf_positioned(label: &str, path: Vec<i64>, x: i64, y: i64, width: i64) -> Positioned {
        Positioned { box_: node(label), path, x, y, width, height: BOX_HEIGHT, children: vec![] }
    }

    #[test]
    fn emit_of_a_leaf_yields_only_a_box_and_a_label_placement() {
        let here = leaf_positioned("hi", vec![0], 5, 5, 10);
        let placements = emit(&here, &[]);

        assert_eq!(placements.len(), 2);
        match &placements[0].node {
            PlacementNode::Node(box_) => assert_eq!(box_, &node("hi")),
            _ => panic!("expected the first placement to wrap the box"),
        }
        assert_eq!(placements[0].x, 5);
        assert_eq!(placements[0].y, 5);
        assert_eq!(placements[0].width, 10);
        assert_eq!(placements[0].height, BOX_HEIGHT);

        match &placements[1].node {
            PlacementNode::Label(label) => {
                assert_eq!(label.text, "hi");
                assert_eq!(label.path, vec![0]);
            }
            _ => panic!("expected the second placement to be a label"),
        }
    }

    #[test]
    fn emit_of_a_parent_yields_a_third_arrow_placement_with_stops_and_shaft() {
        let here = leaf_positioned("parent", vec![0], 0, 3, 10);
        let child_a = leaf_positioned("a", vec![0, 0], 20, 0, 5);
        let child_b = leaf_positioned("b", vec![0, 1], 20, 6, 5);
        let placements = emit(&here, &[child_a.clone(), child_b.clone()]);

        assert_eq!(placements.len(), 3);
        let origin = child_a.y + child_a.height / 2;
        let expected_stops = vec![
            child_a.y + child_a.height / 2 - origin,
            child_b.y + child_b.height / 2 - origin,
        ];
        let expected_shaft = here.y + here.height / 2 - origin;
        match &placements[2].node {
            PlacementNode::Arrow(arrow) => {
                assert_eq!(arrow.stops, expected_stops);
                assert_eq!(arrow.shaft, expected_shaft);
            }
            _ => panic!("expected the third placement to be an arrow"),
        }
        assert_eq!(placements[2].x, here.x + here.width);
        assert_eq!(placements[2].y, origin);
        assert_eq!(placements[2].width, GAP_WIDTH);
    }

    #[test]
    fn layout_of_no_boxes_is_empty() {
        assert_eq!(layout(vec![], 80, 24), vec![]);
    }

    #[test]
    fn layout_of_a_single_leaf_box_has_no_arrow_placements() {
        let placements = layout(vec![node("hi")], 80, 24);
        assert!(placements.iter().all(|p| !matches!(p.node, PlacementNode::Arrow(_))));
        assert!(placements.iter().any(|p| matches!(p.node, PlacementNode::Node(_))));
        assert!(placements.iter().any(|p| matches!(p.node, PlacementNode::Label(_))));
    }

    #[test]
    fn layout_of_a_parent_and_child_has_an_arrow_placement() {
        let boxes = vec![node_with_children("parent", vec![node("child")])];
        let placements = layout(boxes, 80, 24);
        assert!(placements.iter().any(|p| matches!(p.node, PlacementNode::Arrow(_))));
    }

    #[test]
    fn layout_draws_boxes_before_labels_and_arrows() {
        let boxes = vec![node_with_children("parent", vec![node("child")])];
        let placements = layout(boxes, 80, 24);

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
        let placements = layout(vec![node("hi")], 80, 24);
        let label = placements
            .iter()
            .find(|p| matches!(p.node, PlacementNode::Label(_)))
            .expect("layout of a leaf box includes a label placement")
            .clone();

        let result = with_cursor(placements.clone(), vec![0]);
        assert_eq!(result.len(), placements.len() + 1);
        let cursor = result.last().unwrap();
        assert!(matches!(cursor.node, PlacementNode::Cursor(_)));
        assert_eq!(cursor.x, label.x + label.width - 1);
        assert_eq!(cursor.y, label.y);
    }

    #[test]
    fn with_cursor_leaves_placements_unchanged_when_nothing_matches() {
        let placements = layout(vec![node("hi")], 80, 24);
        let result = with_cursor(placements.clone(), vec![99]);
        assert_eq!(result, placements);
    }

    #[test]
    fn label_constructor_defaults_path_to_empty() {
        let label = Label { text: "hi".to_string(), path: vec![] };
        assert_eq!(label.text, "hi");
        assert_eq!(label.path, Vec::<i64>::new());
    }

    #[test]
    fn arrow_stores_stops_and_shaft_as_plain_fields() {
        let arrow = Arrow { stops: vec![1, 2, 3], shaft: 5 };
        assert_eq!(arrow.stops, vec![1, 2, 3]);
        assert_eq!(arrow.shaft, 5);
    }

    #[test]
    fn placement_node_holds_the_matching_variants_inner_value() {
        let box_placement = Placement { node: PlacementNode::Node(node("a")), x: 0, y: 0, width: 3, height: 3 };
        match box_placement.node {
            PlacementNode::Node(box_) => assert_eq!(box_, node("a")),
            _ => panic!("expected a Node variant"),
        }

        let label_placement = Placement {
            node: PlacementNode::Label(Label { text: "a".to_string(), path: vec![0] }),
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        match label_placement.node {
            PlacementNode::Label(label) => {
                assert_eq!(label, Label { text: "a".to_string(), path: vec![0] })
            }
            _ => panic!("expected a Label variant"),
        }

        let arrow_placement = Placement {
            node: PlacementNode::Arrow(Arrow { stops: vec![0], shaft: 0 }),
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        match arrow_placement.node {
            PlacementNode::Arrow(arrow) => assert_eq!(arrow, Arrow { stops: vec![0], shaft: 0 }),
            _ => panic!("expected an Arrow variant"),
        }

        let cursor_placement = Placement { node: PlacementNode::Cursor(Cursor), x: 0, y: 0, width: 1, height: 1 };
        assert!(matches!(cursor_placement.node, PlacementNode::Cursor(_)));
    }
}
