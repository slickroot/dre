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

#[cfg(test)]
mod tests {
    use super::*;

    fn node(label: &str) -> Node {
        Node::new(label.to_string(), crate::PLAIN, crate::PLAIN, false, vec![])
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
}
