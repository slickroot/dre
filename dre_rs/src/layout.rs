// This submodule is grown incrementally (spec 051, slice 1 of 4): today it
// only holds the pure geometry helpers, none of which are wired into a
// Python-visible entry point yet, so the compiler can't see any of this is
// used. Later slices build `layout()`/`with_cursor()` on top of it.
#![allow(dead_code)]

use crate::Node;

pub(crate) const BOX_HEIGHT: i64 = 3;
pub(crate) const GAP_HEIGHT: i64 = 3;
pub(crate) const GAP_WIDTH: i64 = 8;
pub(crate) const BORDERS: i64 = 2;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn node(label: &str) -> Node {
        Node::new(label.to_string(), crate::PLAIN, crate::PLAIN, false, vec![])
    }

    fn node_with_children(label: &str, children: Vec<Node>) -> Node {
        Node::new(label.to_string(), crate::PLAIN, crate::PLAIN, false, children)
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
        // width 7, label "hi" -> interior 2, leftover = 7 - 2 - 2 = 3
        // centre = 1 + 3 - 3 // 2 = 1 + 3 - 1 = 3
        assert_eq!(centre(7, "hi"), 3);
    }

    #[test]
    fn centre_of_a_tightly_fit_label_is_one() {
        // width == interior(label) + BORDERS -> leftover == 0
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
        // middle = 1, so we look at children[0].row + 1
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

        // two leaves consume 2 * LEAF_STRIDE rows of "free" space
        assert_eq!(next_free, 2 * LEAF_STRIDE);

        // parent's own row is anchored between its two children
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
        // second tree's row starts where the first tree's free counter left off
        assert_eq!(trees[1].row, LEAF_STRIDE);
    }

    #[test]
    fn forest_threads_free_across_a_multi_child_tree_and_a_following_leaf() {
        let boxes = vec![node_with_children("parent", vec![node("a"), node("b")]), node("c")];
        let trees = forest(&boxes);

        assert_eq!(trees.len(), 2);
        // the parent tree consumed 2 leaf slots (its two children)
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
}
