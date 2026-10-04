use crate::canvas::Canvas;
use crate::kitty::{ImageId, PlacementId};
use crate::view::Rgb;
use std::collections::HashMap;
use std::num::NonZeroU32;

use super::brackets::BracketKey;
use super::terminal::{GlyphKey, SpriteKey};
use super::tiles::TileKey;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SourceRect {
    pub(crate) x: i64,
    pub(crate) y: i64,
    pub(crate) width: i64,
    pub(crate) height: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum ImageKey {
    Glyph(GlyphKey),
    Tile(TileKey),
    Bracket(BracketKey),
    Sprite(SpriteKey),
    Caret(CaretKey),
    TypingCaret(TypingCaretKey),
    Grow(GrowKey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct TypingCaretKey {
    pub(super) colour: Rgb,
    pub(super) bold: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct CaretKey {
    pub(super) cols: i64,
    pub(super) rows: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct GrowKey {
    pub(super) style_key: SpriteKey,
    pub(super) stamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Desired {
    pub(super) image: ImageKey,
    pub(crate) col: i64,
    pub(crate) row: i64,
    pub(super) z: i32,
    pub(super) source: Option<SourceRect>,
    pub(super) cells: Option<(i64, i64)>,
}

pub(crate) fn caret_desired(cols: i64, rows: i64, col: i64, row: i64, z: i32) -> Desired {
    Desired {
        image: ImageKey::Caret(CaretKey { cols, rows }),
        col,
        row,
        z,
        source: None,
        cells: None,
    }
}

pub enum Content {
    Still(Canvas),
    Animation { root: Canvas, frames: Vec<Canvas> },
}

pub(super) trait Sprites {
    fn content(&mut self, key: &ImageKey) -> Content;
}

struct ContentSource<'a>(&'a mut dyn FnMut() -> Content);

impl Sprites for ContentSource<'_> {
    fn content(&mut self, _key: &ImageKey) -> Content {
        (self.0)()
    }
}

pub(crate) enum Op {
    Upload {
        image: ImageId,
        content: Content,
    },
    Place {
        image: ImageId,
        placement: PlacementId,
        col: i64,
        row: i64,
        z: i32,
        source: Option<SourceRect>,
        cells: Option<(i64, i64)>,
    },
    Delete {
        image: ImageId,
        placement: PlacementId,
    },
    Free {
        image: ImageId,
    },
}

#[derive(Clone, PartialEq)]
struct PlacedProps {
    col: i64,
    row: i64,
    z: i32,
    source: Option<SourceRect>,
    cells: Option<(i64, i64)>,
}

pub struct VirtualTerminal {
    images: HashMap<ImageKey, (ImageId, Vec<(PlacementId, PlacedProps)>)>,
    next_image_id: u32,
}

fn placement_id_value(p: PlacementId) -> u32 {
    p.value()
}

impl VirtualTerminal {
    pub(super) fn new() -> Self {
        VirtualTerminal {
            images: HashMap::new(),
            next_image_id: 1,
        }
    }

    pub(super) fn commit(&mut self, desired: &[Desired], sprites: &mut dyn Sprites) -> Vec<Op> {
        let mut desired_unclaimed: Vec<(usize, &Desired)> = desired.iter().enumerate().collect();

        let mut deletes: Vec<Op> = Vec::new();
        let mut uploads: Vec<Op> = Vec::new();
        let mut places: Vec<Op> = Vec::new();
        let mut frees: Vec<Op> = Vec::new();

        let mut kept: HashMap<ImageKey, Vec<usize>> = HashMap::new();

        for (key, (_id, placements)) in &self.images {
            for (idx, (_, props)) in placements.iter().enumerate() {
                let pos = desired_unclaimed.iter().position(|(_, d)| {
                    d.image == *key
                        && d.col == props.col
                        && d.row == props.row
                        && d.z == props.z
                        && d.source == props.source
                        && d.cells == props.cells
                });
                if let Some(pos) = pos {
                    desired_unclaimed.remove(pos);
                    kept.entry(key.clone()).or_default().push(idx);
                }
            }
        }

        let mut spares: HashMap<ImageKey, Vec<usize>> = HashMap::new();
        for (key, (_id, placements)) in &self.images {
            let kept_indices = kept.get(key).cloned().unwrap_or_default();
            for idx in 0..placements.len() {
                if !kept_indices.contains(&idx) {
                    spares.entry(key.clone()).or_default().push(idx);
                }
            }
        }

        let mut new_placements: Vec<(ImageKey, ImageId, PlacementId, PlacedProps)> = Vec::new();
        let mut used_spares: HashMap<ImageKey, Vec<usize>> = HashMap::new();

        for (_, d) in &desired_unclaimed {
            let props = PlacedProps {
                col: d.col,
                row: d.row,
                z: d.z,
                source: d.source.clone(),
                cells: d.cells,
            };

            let spare = spares.get_mut(&d.image).and_then(|v| {
                if v.is_empty() {
                    None
                } else {
                    Some(v.remove(0))
                }
            });

            if let Some(spare_idx) = spare {
                used_spares
                    .entry(d.image.clone())
                    .or_default()
                    .push(spare_idx);
                let (image_id, placements) = self.images.get(&d.image).unwrap();
                let (placement_id, _) = placements[spare_idx];
                places.push(Op::Place {
                    image: *image_id,
                    placement: placement_id,
                    col: d.col,
                    row: d.row,
                    z: d.z,
                    source: d.source.clone(),
                    cells: d.cells,
                });
                new_placements.push((d.image.clone(), *image_id, placement_id, props));
            } else if let Some((image_id, existing_placements)) = self.images.get(&d.image) {
                let max_existing = existing_placements
                    .iter()
                    .map(|(p, _)| placement_id_value(*p))
                    .max()
                    .unwrap_or(0);
                let extra = new_placements
                    .iter()
                    .filter(|(k, _, _, _)| k == &d.image)
                    .count() as u32;
                let placement_id =
                    PlacementId::new(NonZeroU32::new(max_existing + 1 + extra).unwrap());
                places.push(Op::Place {
                    image: *image_id,
                    placement: placement_id,
                    col: d.col,
                    row: d.row,
                    z: d.z,
                    source: d.source.clone(),
                    cells: d.cells,
                });
                new_placements.push((d.image.clone(), *image_id, placement_id, props));
            } else if let Some(batch_id) = new_placements
                .iter()
                .find(|(k, _, _, _)| k == &d.image)
                .map(|(_, id, _, _)| *id)
            {
                let extra = new_placements
                    .iter()
                    .filter(|(k, _, _, _)| k == &d.image)
                    .count() as u32;
                let placement_id = PlacementId::new(NonZeroU32::new(extra + 1).unwrap());
                places.push(Op::Place {
                    image: batch_id,
                    placement: placement_id,
                    col: d.col,
                    row: d.row,
                    z: d.z,
                    source: d.source.clone(),
                    cells: d.cells,
                });
                new_placements.push((d.image.clone(), batch_id, placement_id, props));
            } else {
                let image_id = ImageId::new(NonZeroU32::new(self.next_image_id).unwrap());
                self.next_image_id += 1;
                let placement_id = PlacementId::new(NonZeroU32::new(1).unwrap());
                let content = sprites.content(&d.image);
                uploads.push(Op::Upload {
                    image: image_id,
                    content,
                });
                places.push(Op::Place {
                    image: image_id,
                    placement: placement_id,
                    col: d.col,
                    row: d.row,
                    z: d.z,
                    source: d.source.clone(),
                    cells: d.cells,
                });
                new_placements.push((d.image.clone(), image_id, placement_id, props));
            }
        }

        for (key, (image_id, placements)) in &self.images {
            let kept_indices = kept.get(key).cloned().unwrap_or_default();
            let used_spare_indices = used_spares.get(key).cloned().unwrap_or_default();
            for (idx, (placement_id, _)) in placements.iter().enumerate() {
                if !kept_indices.contains(&idx) && !used_spare_indices.contains(&idx) {
                    deletes.push(Op::Delete {
                        image: *image_id,
                        placement: *placement_id,
                    });
                }
            }
        }

        let mut new_state: HashMap<ImageKey, (ImageId, Vec<(PlacementId, PlacedProps)>)> =
            HashMap::new();

        for (key, (image_id, placements)) in &self.images {
            let kept_indices = kept.get(key).cloned().unwrap_or_default();
            for idx in &kept_indices {
                let (placement_id, props) = &placements[*idx];
                new_state
                    .entry(key.clone())
                    .or_insert_with(|| (*image_id, Vec::new()))
                    .1
                    .push((*placement_id, props.clone()));
            }
        }

        for (key, image_id, placement_id, props) in new_placements {
            new_state
                .entry(key)
                .or_insert_with(|| (image_id, Vec::new()))
                .1
                .push((placement_id, props));
        }

        for (key, (image_id, _)) in &self.images {
            if !new_state.contains_key(key) {
                frees.push(Op::Free { image: *image_id });
            }
        }

        self.images = new_state;

        let mut ops = Vec::new();
        ops.extend(deletes);
        ops.extend(uploads);
        ops.extend(places);
        ops.extend(frees);
        ops
    }

    pub(crate) fn commit_for_bench(
        &mut self,
        desired: &[Desired],
        content: &mut dyn FnMut() -> Content,
    ) -> Vec<Op> {
        self.commit(desired, &mut ContentSource(content))
    }

    pub(super) fn reset(&mut self) -> Vec<Op> {
        let ops = self
            .images
            .values()
            .map(|(image_id, _)| Op::Free { image: *image_id })
            .collect();
        self.images.clear();
        ops
    }
}

impl Default for VirtualTerminal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::{Rgba, Shape};

    struct Solid(Rgba);
    impl Shape for Solid {
        fn colour_at(&self, _x: i64, _y: i64) -> Option<Rgba> {
            Some(self.0)
        }
    }

    fn test_key(id: u8) -> ImageKey {
        ImageKey::Caret(CaretKey {
            cols: id as i64,
            rows: 1,
        })
    }

    fn test_desired(key: ImageKey, col: i64, row: i64) -> Desired {
        Desired {
            image: key,
            col,
            row,
            z: 0,
            source: None,
            cells: None,
        }
    }

    struct FakeSprites {
        call_count: usize,
    }

    impl FakeSprites {
        fn new() -> Self {
            FakeSprites { call_count: 0 }
        }
    }

    impl Sprites for FakeSprites {
        fn content(&mut self, _key: &ImageKey) -> Content {
            self.call_count += 1;
            Content::Still(Canvas::fill(1, 1, &Solid([255, 0, 0, 255])))
        }
    }

    fn first_upload_image_id(ops: &[Op]) -> Option<ImageId> {
        ops.iter().find_map(|op| match op {
            Op::Upload { image, .. } => Some(*image),
            _ => None,
        })
    }

    fn first_place_placement_id(ops: &[Op]) -> Option<PlacementId> {
        ops.iter().find_map(|op| match op {
            Op::Place { placement, .. } => Some(*placement),
            _ => None,
        })
    }

    #[test]
    fn the_first_commit_uploads_each_image_once_and_places_everything() {
        let mut vt = VirtualTerminal::new();
        let mut sprites = FakeSprites::new();
        let desired = vec![
            test_desired(test_key(1), 0, 0),
            test_desired(test_key(2), 1, 0),
        ];
        let ops = vt.commit(&desired, &mut sprites);

        let upload_count = ops
            .iter()
            .filter(|op| matches!(op, Op::Upload { .. }))
            .count();
        let place_count = ops
            .iter()
            .filter(|op| matches!(op, Op::Place { .. }))
            .count();
        assert_eq!(upload_count, 2);
        assert_eq!(place_count, 2);
    }

    #[test]
    fn an_unchanged_frame_commits_no_ops() {
        let mut vt = VirtualTerminal::new();
        let mut sprites = FakeSprites::new();
        let desired = vec![test_desired(test_key(1), 0, 0)];
        vt.commit(&desired, &mut sprites);
        let ops = vt.commit(&desired, &mut sprites);
        assert!(ops.is_empty());
    }

    #[test]
    fn a_moved_placement_is_re_placed_under_its_old_id() {
        let mut vt = VirtualTerminal::new();
        let mut sprites = FakeSprites::new();

        let first = vec![test_desired(test_key(1), 0, 0)];
        let ops1 = vt.commit(&first, &mut sprites);
        let orig_image_id = first_upload_image_id(&ops1).unwrap();
        let orig_placement_id = first_place_placement_id(&ops1).unwrap();

        let second = vec![test_desired(test_key(1), 1, 0)];
        let ops2 = vt.commit(&second, &mut sprites);

        let place_ops: Vec<_> = ops2
            .iter()
            .filter(|op| matches!(op, Op::Place { .. }))
            .collect();
        assert_eq!(place_ops.len(), 1);
        match &place_ops[0] {
            Op::Place {
                image,
                placement,
                col,
                ..
            } => {
                assert_eq!(*image, orig_image_id);
                assert_eq!(*placement, orig_placement_id);
                assert_eq!(*col, 1);
            }
            _ => panic!("expected Place"),
        }
        assert!(!ops2.iter().any(|op| matches!(op, Op::Upload { .. })));
    }

    #[test]
    fn a_placement_is_re_placed_from_any_spare_placement_of_the_same_image() {
        let mut vt = VirtualTerminal::new();
        let mut sprites = FakeSprites::new();

        let first = vec![
            test_desired(test_key(1), 0, 0),
            test_desired(test_key(1), 2, 0),
        ];
        let ops1 = vt.commit(&first, &mut sprites);
        let orig_image_id = first_upload_image_id(&ops1).unwrap();

        let second = vec![
            test_desired(test_key(1), 0, 0),
            test_desired(test_key(1), 3, 0),
        ];
        let ops2 = vt.commit(&second, &mut sprites);

        assert!(!ops2.iter().any(|op| matches!(op, Op::Upload { .. })));
        let place_ops: Vec<_> = ops2
            .iter()
            .filter(|op| matches!(op, Op::Place { .. }))
            .collect();
        assert_eq!(place_ops.len(), 1);
        match &place_ops[0] {
            Op::Place { image, col, .. } => {
                assert_eq!(*image, orig_image_id);
                assert_eq!(*col, 3);
            }
            _ => panic!("expected Place"),
        }
    }

    #[test]
    fn a_new_placement_with_no_spare_gets_a_new_placement_id() {
        let mut vt = VirtualTerminal::new();
        let mut sprites = FakeSprites::new();

        let first = vec![
            test_desired(test_key(1), 0, 0),
            test_desired(test_key(1), 1, 0),
        ];
        let ops1 = vt.commit(&first, &mut sprites);
        let orig_placement_ids: Vec<PlacementId> = ops1
            .iter()
            .filter_map(|op| match op {
                Op::Place { placement, .. } => Some(*placement),
                _ => None,
            })
            .collect();

        let second = vec![
            test_desired(test_key(1), 0, 0),
            test_desired(test_key(1), 1, 0),
            test_desired(test_key(1), 2, 0),
        ];
        let ops2 = vt.commit(&second, &mut sprites);

        assert!(!ops2.iter().any(|op| matches!(op, Op::Upload { .. })));
        let place_ops: Vec<_> = ops2
            .iter()
            .filter(|op| matches!(op, Op::Place { .. }))
            .collect();
        assert_eq!(place_ops.len(), 1);
        match &place_ops[0] {
            Op::Place { placement, .. } => {
                assert!(!orig_placement_ids.contains(placement));
            }
            _ => panic!("expected Place"),
        }
    }

    #[test]
    fn a_placement_no_longer_desired_is_deleted() {
        let mut vt = VirtualTerminal::new();
        let mut sprites = FakeSprites::new();

        vt.commit(&[test_desired(test_key(1), 0, 0)], &mut sprites);
        let ops = vt.commit(&[], &mut sprites);
        assert!(ops.iter().any(|op| matches!(op, Op::Delete { .. })));
    }

    #[test]
    fn an_image_with_no_placements_left_is_freed() {
        let mut vt = VirtualTerminal::new();
        let mut sprites = FakeSprites::new();

        vt.commit(&[test_desired(test_key(1), 0, 0)], &mut sprites);
        let ops = vt.commit(&[], &mut sprites);

        let delete_count = ops
            .iter()
            .filter(|op| matches!(op, Op::Delete { .. }))
            .count();
        let free_count = ops
            .iter()
            .filter(|op| matches!(op, Op::Free { .. }))
            .count();
        assert_eq!(delete_count, 1);
        assert_eq!(free_count, 1);

        let delete_pos = ops
            .iter()
            .position(|op| matches!(op, Op::Delete { .. }))
            .unwrap();
        let free_pos = ops
            .iter()
            .position(|op| matches!(op, Op::Free { .. }))
            .unwrap();
        assert!(delete_pos < free_pos);
    }

    #[test]
    fn a_freed_image_is_uploaded_again_when_it_comes_back() {
        let mut vt = VirtualTerminal::new();
        let mut sprites = FakeSprites::new();

        let desired = vec![test_desired(test_key(1), 0, 0)];
        vt.commit(&desired, &mut sprites);
        vt.commit(&[], &mut sprites);

        let ops = vt.commit(&desired, &mut sprites);
        assert!(ops.iter().any(|op| matches!(op, Op::Upload { .. })));
        assert!(ops.iter().any(|op| matches!(op, Op::Place { .. })));
    }

    #[test]
    fn content_is_only_asked_for_on_upload() {
        let mut vt = VirtualTerminal::new();
        let mut sprites = FakeSprites::new();

        let desired = vec![test_desired(test_key(1), 0, 0)];
        vt.commit(&desired, &mut sprites);
        assert_eq!(sprites.call_count, 1);

        vt.commit(&desired, &mut sprites);
        assert_eq!(sprites.call_count, 1);

        vt.commit(&[test_desired(test_key(1), 1, 0)], &mut sprites);
        assert_eq!(sprites.call_count, 1);
    }

    #[test]
    fn reset_clears_and_forgets_everything() {
        let mut vt = VirtualTerminal::new();
        let mut sprites = FakeSprites::new();

        let desired = vec![test_desired(test_key(1), 0, 0)];
        vt.commit(&desired, &mut sprites);
        assert_eq!(sprites.call_count, 1);

        vt.reset();

        let ops = vt.commit(&desired, &mut sprites);
        assert!(ops.iter().any(|op| matches!(op, Op::Upload { .. })));
        assert_eq!(sprites.call_count, 2);
    }

    #[test]
    fn deletes_come_before_uploads_and_places_and_frees_come_last() {
        let mut vt = VirtualTerminal::new();
        let mut sprites = FakeSprites::new();

        vt.commit(&[test_desired(test_key(1), 0, 0)], &mut sprites);

        let second = vec![test_desired(test_key(2), 0, 0)];
        let ops = vt.commit(&second, &mut sprites);

        let delete_pos = ops
            .iter()
            .position(|op| matches!(op, Op::Delete { .. }))
            .unwrap();
        let upload_pos = ops
            .iter()
            .position(|op| matches!(op, Op::Upload { .. }))
            .unwrap();
        let place_pos = ops
            .iter()
            .position(|op| matches!(op, Op::Place { .. }))
            .unwrap();
        let free_pos = ops
            .iter()
            .position(|op| matches!(op, Op::Free { .. }))
            .unwrap();

        assert!(delete_pos < upload_pos);
        assert!(upload_pos < place_pos);
        assert!(place_pos < free_pos);
    }
}
