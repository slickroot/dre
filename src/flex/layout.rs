use types::Tree;

use crate::view;

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
