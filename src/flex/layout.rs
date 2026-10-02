use types::Tree;

use crate::view::{self, Area};

use super::state::{Direction, FlexBox, Justify};

pub(crate) const FLEX_BORDER: i64 = 1;
pub(crate) const FLEX_SPACE: Size = Size {
    width: 2,
    height: 1,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Size {
    pub(crate) width: i64,
    pub(crate) height: i64,
}

impl Size {
    pub(crate) fn main(self, direction: Direction) -> i64 {
        match direction {
            Direction::Row => self.width,
            Direction::Column => self.height,
        }
    }

    pub(crate) fn cross(self, direction: Direction) -> i64 {
        match direction {
            Direction::Row => self.height,
            Direction::Column => self.width,
        }
    }

    pub(crate) fn along(direction: Direction, main: i64, cross: i64) -> Size {
        match direction {
            Direction::Row => Size {
                width: main,
                height: cross,
            },
            Direction::Column => Size {
                width: cross,
                height: main,
            },
        }
    }
}

fn padding(flex_box: &FlexBox) -> Size {
    if !flex_box.border {
        Size {
            width: 0,
            height: 0,
        }
    } else {
        Size {
            width: FLEX_SPACE.width * (1 + i64::from(flex_box.padding)),
            height: FLEX_SPACE.height,
        }
    }
}

fn text_size(flex_box: &FlexBox) -> Option<Size> {
    flex_box.text.as_ref().map(|text| Size {
        width: view::interior(text),
        height: 1,
    })
}

fn item_sizes(tree: &Tree<FlexBox>, path: &[usize]) -> Vec<Size> {
    text_size(tree.value(path))
        .into_iter()
        .chain(tree.children(path).iter().map(|child| measure(tree, child)))
        .collect()
}

pub(crate) fn measure(tree: &Tree<FlexBox>, path: &[usize]) -> Size {
    let flex_box = tree.value(path);
    let direction = flex_box.direction;
    let items = item_sizes(tree, path);
    let gaps = FLEX_SPACE.main(direction) * items.len().saturating_sub(1) as i64;
    let main = items.iter().map(|item| item.main(direction)).sum::<i64>() + gaps;
    let cross = items
        .iter()
        .map(|item| item.cross(direction))
        .max()
        .unwrap_or(0);
    let content = Size::along(direction, main, cross);
    let padding = padding(flex_box);
    Size {
        width: content.width + 2 * padding.width,
        height: content.height + 2 * padding.height,
    }
}

pub(crate) fn distribute(lengths: &[i64], room: i64, justify: Justify, gap: i64) -> Vec<i64> {
    let gaps = lengths.len().saturating_sub(1) as i64;
    let (gap, widened_from) = match justify {
        Justify::SpaceBetween if gaps > 0 => {
            let free = room - lengths.iter().sum::<i64>();
            (free.div_euclid(gaps), gaps - free.rem_euclid(gaps))
        }
        _ => (gap, gaps),
    };
    let mut cursor = match justify {
        Justify::Center => {
            let total = lengths.iter().sum::<i64>() + gap * gaps;
            (room - total).div_euclid(2)
        }
        _ => 0,
    };
    lengths
        .iter()
        .enumerate()
        .map(|(index, length)| {
            let offset = cursor;
            cursor += length + gap + i64::from(index as i64 >= widened_from);
            offset
        })
        .collect()
}

fn share(room: i64, count: usize, gap: i64) -> Vec<i64> {
    let count = count as i64;
    let free = room - gap * (count - 1).max(0);
    (0..count)
        .map(|index| free.div_euclid(count) + i64::from(index < free.rem_euclid(count)))
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rect {
    pub(crate) x: i64,
    pub(crate) y: i64,
    pub(crate) width: i64,
    pub(crate) height: i64,
}

impl Rect {
    pub(crate) fn inner(self, padding: Size) -> Rect {
        Rect {
            x: self.x + padding.width,
            y: self.y + padding.height,
            width: self.width - 2 * padding.width,
            height: self.height - 2 * padding.height,
        }
    }

    pub(crate) fn size(self) -> Size {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    pub(crate) fn main_start(self, direction: Direction) -> i64 {
        match direction {
            Direction::Row => self.x,
            Direction::Column => self.y,
        }
    }

    pub(crate) fn cross_start(self, direction: Direction) -> i64 {
        match direction {
            Direction::Row => self.y,
            Direction::Column => self.x,
        }
    }

    pub(crate) fn along(
        direction: Direction,
        main_start: i64,
        cross_start: i64,
        size: Size,
    ) -> Rect {
        let (x, y) = match direction {
            Direction::Row => (main_start, cross_start),
            Direction::Column => (cross_start, main_start),
        };
        Rect {
            x,
            y,
            width: size.width,
            height: size.height,
        }
    }
}

pub(crate) struct Arranged {
    pub(crate) path: Vec<usize>,
    pub(crate) rect: Rect,
    pub(crate) text: Option<Rect>,
}

pub(crate) fn arrange(tree: &Tree<FlexBox>, path: &[usize], rect: Rect, out: &mut Vec<Arranged>) {
    let flex_box = tree.value(path);
    let direction = flex_box.direction;
    let children = tree.children(path);
    let sizes = item_sizes(tree, path);
    let inner = rect.inner(padding(flex_box));
    let inner_cross = inner.size().cross(direction);
    let mains: Vec<i64> = match (direction, flex_box.justify) {
        (Direction::Row, Justify::Start) => share(inner.width, sizes.len(), FLEX_SPACE.width),
        _ => sizes.iter().map(|size| size.main(direction)).collect(),
    };
    let offsets = distribute(
        &mains,
        inner.size().main(direction),
        flex_box.justify,
        FLEX_SPACE.main(direction),
    );
    let item_rects: Vec<Rect> = sizes
        .iter()
        .zip(&mains)
        .zip(offsets)
        .map(|((size, main), offset)| {
            let (cross, cross_offset) = match direction {
                Direction::Column => (inner_cross, 0),
                Direction::Row => {
                    let cross = size.cross(direction);
                    (cross, (inner_cross - cross) / 2)
                }
            };
            Rect::along(
                direction,
                inner.main_start(direction) + offset,
                inner.cross_start(direction) + cross_offset,
                Size::along(direction, *main, cross),
            )
        })
        .collect();
    let (text, child_rects) = if flex_box.text.is_some() {
        (Some(item_rects[0]), &item_rects[1..])
    } else {
        (None, &item_rects[..])
    };
    out.push(Arranged {
        path: path.to_vec(),
        rect,
        text,
    });
    for (child, child_rect) in children.iter().zip(child_rects) {
        arrange(tree, child, *child_rect, out);
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Heading {
    Left,
    Right,
    Up,
    Down,
}

pub(crate) struct Layout {
    arranged: Vec<Arranged>,
    area: Area,
}

#[cfg_attr(not(test), allow(dead_code))]
impl Layout {
    pub(crate) fn arrange(tree: &Tree<FlexBox>, area: Area) -> Layout {
        let window = Rect {
            x: area.col,
            y: area.row,
            width: area.cols,
            height: area.rows,
        };
        let mut arranged = Vec::new();
        arrange(tree, &[], window, &mut arranged);
        Layout { arranged, area }
    }

    pub(crate) fn rect(&self, path: &[usize]) -> Option<Rect> {
        self.arranged
            .iter()
            .find(|arranged| arranged.path == path)
            .map(|arranged| arranged.rect)
    }

    pub(crate) fn arranged(&self) -> &[Arranged] {
        &self.arranged
    }

    pub(crate) fn area(&self) -> Area {
        self.area
    }

    pub(crate) fn nearest(&self, from: &[usize], heading: Heading) -> Option<Vec<usize>> {
        let current = self.rect(from)?;
        self.arranged
            .iter()
            .filter(|a| a.path != from)
            .filter(|a| is_beyond(a.rect, current, heading))
            .min_by_key(|a| {
                (
                    leading_gap(a.rect, current, heading),
                    cross_offset(a.rect, current, heading),
                )
            })
            .map(|a| a.path.clone())
    }
}

fn is_beyond(r: Rect, c: Rect, heading: Heading) -> bool {
    match heading {
        Heading::Down => r.y > c.y,
        Heading::Up => r.y < c.y,
        Heading::Right => r.x > c.x,
        Heading::Left => r.x < c.x,
    }
}

fn leading_gap(r: Rect, c: Rect, heading: Heading) -> i64 {
    match heading {
        Heading::Down => r.y - c.y,
        Heading::Up => c.y - r.y,
        Heading::Right => r.x - c.x,
        Heading::Left => c.x - r.x,
    }
}

fn cross_offset(r: Rect, c: Rect, heading: Heading) -> i64 {
    match heading {
        Heading::Up | Heading::Down => (r.x - c.x).abs(),
        Heading::Left | Heading::Right => (r.y - c.y).abs(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(path: &[usize], x: i64, y: i64, width: i64, height: i64) -> Arranged {
        Arranged {
            path: path.to_vec(),
            rect: Rect {
                x,
                y,
                width,
                height,
            },
            text: None,
        }
    }

    fn layout(nodes: Vec<Arranged>) -> Layout {
        Layout {
            arranged: nodes,
            area: Area {
                col: 0,
                row: 0,
                cols: 0,
                rows: 0,
            },
        }
    }

    #[test]
    fn down_picks_the_box_whose_top_left_corner_is_lowest() {
        let from = [0];
        let layout = layout(vec![
            at(&from, 0, 0, 1, 1),
            at(&[1], 0, 8, 1, 1),
            at(&[2], 0, 3, 1, 1),
            at(&[3], 0, 5, 1, 1),
        ]);
        assert_eq!(layout.nearest(&from, Heading::Down), Some(vec![2]));
    }

    #[test]
    fn up_picks_the_box_whose_top_left_corner_is_highest() {
        let from = [0];
        let layout = layout(vec![
            at(&from, 0, 10, 1, 1),
            at(&[1], 0, 5, 1, 1),
            at(&[2], 0, 8, 1, 1),
            at(&[3], 0, 1, 1, 1),
        ]);
        assert_eq!(layout.nearest(&from, Heading::Up), Some(vec![2]));
    }

    #[test]
    fn left_and_right_pick_on_the_x_corner() {
        let from = [0];
        let layout = layout(vec![
            at(&from, 10, 0, 1, 1),
            at(&[1], 3, 0, 1, 1),
            at(&[2], 25, 0, 1, 1),
            at(&[3], 18, 0, 1, 1),
        ]);
        assert_eq!(layout.nearest(&from, Heading::Right), Some(vec![3]));
        assert_eq!(layout.nearest(&from, Heading::Left), Some(vec![1]));
    }

    #[test]
    fn a_box_level_with_the_current_one_is_not_to_its_left_or_right() {
        let from = [0];
        let layout = layout(vec![at(&from, 5, 0, 1, 1), at(&[1], 5, 10, 1, 1)]);
        assert_eq!(layout.nearest(&from, Heading::Left), None);
        assert_eq!(layout.nearest(&from, Heading::Right), None);
    }

    #[test]
    fn an_equal_leading_corner_is_broken_by_the_closest_across_axis() {
        let from = [0];
        let layout = layout(vec![
            at(&from, 0, 0, 1, 1),
            at(&[1], 20, 5, 1, 1),
            at(&[2], 2, 5, 1, 1),
        ]);
        assert_eq!(layout.nearest(&from, Heading::Down), Some(vec![2]));
    }

    #[test]
    fn nothing_in_that_direction_selects_nothing() {
        let from = [0];
        let layout = layout(vec![at(&from, 0, 0, 1, 1), at(&[1], 0, 10, 1, 1)]);
        assert_eq!(layout.nearest(&from, Heading::Left), None);
        assert_eq!(layout.nearest(&from, Heading::Right), None);
        assert_eq!(layout.nearest(&from, Heading::Up), None);
    }

    #[test]
    fn a_descendant_below_is_picked_by_down() {
        let from = [0];
        let layout = layout(vec![at(&from, 0, 0, 10, 1), at(&[0, 0], 0, 5, 1, 1)]);
        assert_eq!(layout.nearest(&from, Heading::Down), Some(vec![0, 0]));
    }

    #[test]
    fn an_ancestor_up_and_left_is_picked_by_up() {
        let from = [0, 0];
        let layout = layout(vec![at(&[0], 0, 0, 1, 1), at(&from, 10, 10, 1, 1)]);
        assert_eq!(layout.nearest(&from, Heading::Up), Some(vec![0]));
        assert_eq!(layout.nearest(&from, Heading::Left), Some(vec![0]));
    }

    #[test]
    fn the_root_is_a_candidate_like_any_other_box() {
        let from = [0, 0];
        let layout = layout(vec![at(&[], 0, 0, 1, 1), at(&from, 5, 5, 1, 1)]);
        assert_eq!(layout.nearest(&from, Heading::Up), Some(vec![]));
    }

    #[test]
    fn a_missing_from_path_selects_nothing() {
        let layout = layout(vec![at(&[0], 0, 0, 1, 1), at(&[1], 0, 5, 1, 1)]);
        assert_eq!(layout.nearest(&[9], Heading::Down), None);
    }
}
