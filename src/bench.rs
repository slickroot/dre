//! Bench-only seam: lets `benches/` drive `VirtualTerminal::commit` from
//! outside the crate. Not API.

use crate::canvas::{Rgba, Shape};
use crate::render::virtual_terminal::caret_desired;

pub use crate::canvas::Canvas;
pub use crate::render::virtual_terminal::{Content, Desired, VirtualTerminal};

// Cohort shares and pool size measured from real `dre-flex` frames; see
// docs/specs/242-benchmark-the-virtual-terminal-commit.md, "The frame
// generator".
const GLYPH_LIKE_SHARE_PERCENT: usize = 65;
const BORDER_LIKE_SHARE_PERCENT: usize = 15;
const GLYPH_KEY_POOL: usize = 60;

const GLYPH_COHORT: i64 = 1;
const BORDER_COHORT: i64 = 2;
const TILE_COHORT: i64 = 3;

const FRAME_COLS: i64 = 80;
const Z_LEVELS: i32 = 4;

fn cohort_sizes(placements: usize) -> (usize, usize, usize) {
    let glyph_like = placements * GLYPH_LIKE_SHARE_PERCENT / 100;
    let border_like = placements * BORDER_LIKE_SHARE_PERCENT / 100;
    (
        glyph_like,
        border_like,
        placements - glyph_like - border_like,
    )
}

fn key_of(index: usize, placements: usize) -> (i64, i64) {
    let (glyph_like, border_like, _) = cohort_sizes(placements);
    if index < glyph_like {
        (GLYPH_COHORT, (index % GLYPH_KEY_POOL) as i64)
    } else if index < glyph_like + border_like {
        (BORDER_COHORT, 0)
    } else {
        (TILE_COHORT, (index - glyph_like - border_like) as i64)
    }
}

pub fn frame(placements: usize) -> Vec<Desired> {
    (0..placements)
        .map(|index| {
            let (cols, rows) = key_of(index, placements);
            let position = index as i64;
            caret_desired(
                cols,
                rows,
                position % FRAME_COLS,
                position / FRAME_COLS,
                (position % i64::from(Z_LEVELS)) as i32,
            )
        })
        .collect()
}

pub fn frame_with_one_moved(placements: usize) -> Vec<Desired> {
    let mut frame = frame(placements);
    frame[placements / 2].col += FRAME_COLS;
    frame
}

pub fn sprite_content() -> Content {
    struct Opaque;
    impl Shape for Opaque {
        fn colour_at(&self, _x: i64, _y: i64) -> Option<Rgba> {
            Some([0, 0, 0, 255])
        }
    }
    Content::Still(Canvas::fill(1, 1, &Opaque))
}

pub fn commit(
    terminal: &mut VirtualTerminal,
    frame: &[Desired],
    content: &mut dyn FnMut() -> Content,
) -> usize {
    terminal.commit_for_bench(frame, content).len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn cold_ops(frame: &[Desired]) -> usize {
        let mut terminal = VirtualTerminal::default();
        commit(&mut terminal, frame, &mut || sprite_content())
    }

    fn expected_distinct_keys(placements: usize) -> usize {
        let (glyph_like, border_like, tile_like) = cohort_sizes(placements);
        glyph_like.min(GLYPH_KEY_POOL) + usize::from(border_like > 0) + tile_like
    }

    #[test]
    fn cold_commit_uploads_each_distinct_key_once_and_places_every_placement() {
        for placements in [10, 200] {
            let frame = frame(placements);
            assert_eq!(frame.len(), placements);
            assert_eq!(
                cold_ops(&frame) - placements,
                expected_distinct_keys(placements)
            );
        }
    }

    #[test]
    fn the_glyph_cohort_saturates_at_the_key_pool() {
        let placements = 200;
        let (glyph_like, _, tile_like) = cohort_sizes(placements);
        assert!(glyph_like > GLYPH_KEY_POOL);
        assert_eq!(
            expected_distinct_keys(placements),
            GLYPH_KEY_POOL + 1 + tile_like
        );
        assert!(expected_distinct_keys(placements) < placements);
    }

    #[test]
    fn recommitting_the_identical_frame_emits_nothing() {
        for placements in [10, 200] {
            let frame = frame(placements);
            let mut terminal = VirtualTerminal::default();
            commit(&mut terminal, &frame, &mut || sprite_content());
            assert_eq!(commit(&mut terminal, &frame, &mut || sprite_content()), 0);
        }
    }

    #[test]
    fn committing_frame_b_after_frame_a_emits_exactly_one_op() {
        for placements in [10, 200] {
            let a = frame(placements);
            let b = frame_with_one_moved(placements);
            let mut terminal = VirtualTerminal::default();
            commit(&mut terminal, &a, &mut || sprite_content());
            assert_eq!(commit(&mut terminal, &b, &mut || sprite_content()), 1);
            assert_eq!(commit(&mut terminal, &a, &mut || sprite_content()), 1);
        }
    }

    #[test]
    fn every_placement_sits_at_its_own_position() {
        for placements in [10, 200] {
            for frame in [frame(placements), frame_with_one_moved(placements)] {
                let positions: HashSet<(i64, i64)> = frame.iter().map(|d| (d.col, d.row)).collect();
                assert_eq!(positions.len(), placements);
            }
        }
    }

    #[test]
    fn the_generator_is_deterministic() {
        assert_eq!(frame(200), frame(200));
        assert_eq!(frame_with_one_moved(200), frame_with_one_moved(200));
        assert_ne!(frame(200), frame_with_one_moved(200));
    }
}
