use types::Tree;

use super::state::{is_canvas, Direction, FlexBox, FlexMode, FlexState, Justify};

const OUTLINE_MARGIN: i64 = 1;
const FLEX_SPACE: Size = Size {
    width: 2,
    height: 1,
};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Size {
    width: i64,
    height: i64,
}

impl Size {
    fn main(self, direction: Direction) -> i64 {
        match direction {
            Direction::Row => self.width,
            Direction::Column => self.height,
        }
    }

    fn cross(self, direction: Direction) -> i64 {
        match direction {
            Direction::Row => self.height,
            Direction::Column => self.width,
        }
    }

    fn along(direction: Direction, main: i64, cross: i64) -> Size {
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
    Size {
        width: FLEX_SPACE.width * i64::from(flex_box.padding),
        height: FLEX_SPACE.height * i64::from(flex_box.padding),
    }
}

fn border_space(path: &[usize], flex_box: &FlexBox) -> Size {
    if is_canvas(path) {
        Size {
            width: 0,
            height: 0,
        }
    } else {
        padding(flex_box)
    }
}

fn interior(text: &str) -> i64 {
    (text.chars().count() as i64).max(1)
}

fn text_size(flex_box: &FlexBox) -> Option<Size> {
    flex_box.text.as_ref().map(|text| Size {
        width: interior(text),
        height: 1,
    })
}

fn item_sizes(tree: &Tree<FlexBox>, path: &[usize]) -> Vec<Size> {
    text_size(tree.value(path))
        .into_iter()
        .chain(tree.children(path).iter().map(|child| measure(tree, child)))
        .collect()
}

fn measure(tree: &Tree<FlexBox>, path: &[usize]) -> Size {
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

fn distribute(lengths: &[i64], room: i64, justify: Justify, gap: i64) -> Vec<i64> {
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
struct Rect {
    x: i64,
    y: i64,
    width: i64,
    height: i64,
}

impl Rect {
    fn inner(self, padding: Size) -> Rect {
        Rect {
            x: self.x + padding.width,
            y: self.y + padding.height,
            width: self.width - 2 * padding.width,
            height: self.height - 2 * padding.height,
        }
    }

    fn size(self) -> Size {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn main_start(self, direction: Direction) -> i64 {
        match direction {
            Direction::Row => self.x,
            Direction::Column => self.y,
        }
    }

    fn cross_start(self, direction: Direction) -> i64 {
        match direction {
            Direction::Row => self.y,
            Direction::Column => self.x,
        }
    }

    fn along(direction: Direction, main_start: i64, cross_start: i64, size: Size) -> Rect {
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

struct Arranged {
    path: Vec<usize>,
    rect: Rect,
    text: Option<Rect>,
}

fn arrange(tree: &Tree<FlexBox>, path: &[usize], rect: Rect, out: &mut Vec<Arranged>) {
    let flex_box = tree.value(path);
    let direction = flex_box.direction;
    let children = tree.children(path);
    let sizes = item_sizes(tree, path);
    let inner = rect.inner(border_space(path, flex_box));
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Cells {
    pub(crate) cols: i64,
    pub(crate) rows: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CellRect {
    pub(crate) x: i64,
    pub(crate) y: i64,
    pub(crate) width: i64,
    pub(crate) height: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Placement {
    Box { rect: CellRect, filled: bool },
    Text { at: (i64, i64), text: String },
    Highlight { rect: CellRect },
    Caret { at: (i64, i64) },
    Outline { rect: CellRect },
}

impl From<Rect> for CellRect {
    fn from(rect: Rect) -> CellRect {
        CellRect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        }
    }
}

fn is_selected(state: &FlexState, mode: FlexMode, path: &[usize]) -> bool {
    state.mode == mode && state.selected == path
}

fn paint(state: &FlexState, flex_box: &FlexBox, arranged: &Arranged) -> Vec<Placement> {
    let path = &arranged.path;
    let border = flex_box.border.then(|| Placement::Box {
        rect: arranged.rect.into(),
        filled: path.len() == 2 && flex_box.filled,
    });
    let text = flex_box.text.as_ref().zip(arranged.text);
    let highlight = text
        .filter(|_| is_selected(state, FlexMode::Replace, path))
        .map(|(text, rect)| Placement::Highlight {
            rect: CellRect {
                width: text.chars().count() as i64,
                height: 1,
                ..rect.into()
            },
        });
    let label = text.map(|(text, rect)| Placement::Text {
        at: (rect.x, rect.y),
        text: text.clone(),
    });
    let caret = text
        .filter(|_| is_selected(state, FlexMode::Write, path))
        .map(|(text, rect)| Placement::Caret {
            at: (rect.x + text.chars().count() as i64, rect.y),
        });
    border
        .into_iter()
        .chain(highlight)
        .chain(label)
        .chain(caret)
        .collect()
}

fn outline(state: &FlexState, arranged: &Arranged) -> Option<Placement> {
    is_selected(state, FlexMode::Move, &arranged.path).then(|| Placement::Outline {
        rect: CellRect {
            x: arranged.rect.x - OUTLINE_MARGIN,
            y: arranged.rect.y - OUTLINE_MARGIN,
            width: arranged.rect.width + 2 * OUTLINE_MARGIN,
            height: arranged.rect.height + 2 * OUTLINE_MARGIN,
        },
    })
}

pub(crate) fn placements(state: &FlexState, window: Cells) -> Vec<Placement> {
    let window_rect = Rect {
        x: 0,
        y: 0,
        width: window.cols,
        height: window.rows,
    };
    let mut rects = Vec::new();
    arrange(&state.boxes, &[0], window_rect, &mut rects);
    let painted = rects
        .iter()
        .flat_map(|arranged| paint(state, state.boxes.value(&arranged.path), arranged));
    let outlines = rects.iter().filter_map(|arranged| outline(state, arranged));
    painted.chain(outlines).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flex::state::{new_box, new_canvas, reduce};

    const WINDOW: Cells = Cells { cols: 80, rows: 24 };

    fn row() -> FlexBox {
        FlexBox {
            direction: Direction::Row,
            ..FlexBox::default()
        }
    }

    fn text(text: &str) -> Tree<FlexBox> {
        Tree::leaf(FlexBox {
            border: false,
            text: Some(text.to_string()),
            ..FlexBox::default()
        })
    }

    fn outer(the_box: FlexBox, texts: &[&str], inner_boxes: usize) -> Tree<FlexBox> {
        Tree::new(
            the_box,
            texts
                .iter()
                .map(|label| text(label))
                .chain((0..inner_boxes).map(|_| new_box()))
                .collect(),
        )
    }

    fn holding(the_box: FlexBox, texts: &[&str]) -> FlexState {
        FlexState {
            boxes: new_canvas(vec![outer(the_box, texts, 0)]),
            ..FlexState::default()
        }
    }

    fn with_text(text: &str) -> FlexState {
        holding(FlexBox::default(), &[text])
    }

    fn beside(texts: &[&str]) -> FlexState {
        holding(FlexBox::default(), texts)
    }

    fn beside_row(texts: &[&str]) -> FlexState {
        holding(row(), texts)
    }

    fn stacked(texts: &[&str]) -> FlexState {
        FlexState {
            boxes: new_canvas(
                texts
                    .iter()
                    .map(|text| outer(FlexBox::default(), &[text], 0))
                    .collect(),
            ),
            ..FlexState::default()
        }
    }

    fn with_inner_boxes(text: &str, count: usize) -> FlexState {
        FlexState {
            boxes: new_canvas(vec![outer(FlexBox::default(), &[text], count)]),
            ..FlexState::default()
        }
    }

    fn after(state: FlexState, keys: &[&str]) -> FlexState {
        keys.iter().fold(state, |state, key| reduce(state, key).0)
    }

    fn placed(state: &FlexState) -> Vec<Placement> {
        placements(state, WINDOW)
    }

    fn box_placements(placements: &[Placement]) -> Vec<(CellRect, bool)> {
        placements
            .iter()
            .filter_map(|placement| match placement {
                Placement::Box { rect, filled } => Some((*rect, *filled)),
                _ => None,
            })
            .collect()
    }

    fn the_canvas(placements: &[Placement]) -> CellRect {
        box_placements(placements)[0].0
    }

    fn own_boxes(placements: &[Placement]) -> Vec<CellRect> {
        box_placements(placements)
            .into_iter()
            .skip(1)
            .map(|(rect, _)| rect)
            .collect()
    }

    fn the_box(placements: &[Placement]) -> CellRect {
        own_boxes(placements)[0]
    }

    fn texts(placements: &[Placement]) -> Vec<((i64, i64), &str)> {
        placements
            .iter()
            .filter_map(|placement| match placement {
                Placement::Text { at, text } => Some((*at, text.as_str())),
                _ => None,
            })
            .collect()
    }

    fn label_rects(placements: &[Placement]) -> Vec<CellRect> {
        texts(placements)
            .into_iter()
            .map(|((x, y), text)| CellRect {
                x,
                y,
                width: interior(text),
                height: 1,
            })
            .collect()
    }

    fn the_label(placements: &[Placement]) -> CellRect {
        label_rects(placements)[0]
    }

    fn outlines(placements: &[Placement]) -> Vec<CellRect> {
        placements
            .iter()
            .filter_map(|placement| match placement {
                Placement::Outline { rect } => Some(*rect),
                _ => None,
            })
            .collect()
    }

    fn highlights(placements: &[Placement]) -> Vec<CellRect> {
        placements
            .iter()
            .filter_map(|placement| match placement {
                Placement::Highlight { rect } => Some(*rect),
                _ => None,
            })
            .collect()
    }

    fn carets(placements: &[Placement]) -> Vec<(i64, i64)> {
        placements
            .iter()
            .filter_map(|placement| match placement {
                Placement::Caret { at } => Some(*at),
                _ => None,
            })
            .collect()
    }

    fn bottom(rect: CellRect) -> i64 {
        rect.y + rect.height
    }

    fn right(rect: CellRect) -> i64 {
        rect.x + rect.width
    }

    fn assert_inside(inner: CellRect, outer: CellRect) {
        assert!(inner.x >= outer.x);
        assert!(inner.y >= outer.y);
        assert!(right(inner) <= right(outer));
        assert!(bottom(inner) <= bottom(outer));
    }

    fn assert_centred(placements: &[Placement]) {
        let the_box = the_box(placements);
        let left_margin = the_box.x;
        let right_margin = WINDOW.cols - right(the_box);
        assert!((left_margin - right_margin).abs() <= 1);
    }

    #[test]
    fn an_empty_box_spans_the_window_and_is_no_height_at_all() {
        let placements = placed(&FlexState::default());
        let the_box = the_box(&placements);
        assert_eq!(the_box.width, WINDOW.cols);
        assert_eq!(the_box.height, 0);
    }

    #[test]
    fn the_first_box_starts_at_the_top_of_the_window() {
        assert_eq!(the_box(&placed(&FlexState::default())).y, 0);
    }

    #[test]
    fn an_empty_canvas_places_a_box_around_the_whole_window() {
        let state = FlexState {
            boxes: new_canvas(vec![]),
            ..FlexState::default()
        };
        let placements = placed(&state);
        assert_eq!(
            the_canvas(&placements),
            CellRect {
                x: 0,
                y: 0,
                width: WINDOW.cols,
                height: WINDOW.rows
            }
        );
        assert!(!box_placements(&placements)[0].1);
    }

    #[test]
    fn a_new_box_puts_its_text_flush_against_its_top_left_corner() {
        let placements = placed(&with_text("Hello"));
        let (the_box, label) = (the_box(&placements), the_label(&placements));
        assert_eq!((label.x, label.y), (the_box.x, the_box.y));
    }

    #[test]
    fn the_label_sits_inside_the_box() {
        let placements = placed(&with_text(""));
        assert_inside(the_label(&placements), the_box(&placements));
    }

    #[test]
    fn only_the_canvas_box_the_label_and_the_outline_are_placed() {
        let placements = placed(&with_text(""));
        assert_eq!(
            (
                box_placements(&placements).len(),
                texts(&placements).len(),
                outlines(&placements).len(),
                placements.len()
            ),
            (2, 1, 1, 4)
        );
    }

    #[test]
    fn the_text_is_placed_inside_the_box() {
        let state = with_text("Hello");
        let placements = placed(&state);
        assert_eq!(
            texts(&placements)[0].1,
            state.boxes.value(&[0, 0, 0]).text.as_deref().unwrap()
        );
        assert_inside(the_label(&placements), the_box(&placements));
    }

    #[test]
    fn the_box_measures_to_fit_the_text() {
        assert_eq!(
            measure(&with_text("Hello").boxes, &[0, 0]).width,
            interior("Hello")
        );
    }

    #[test]
    fn the_box_measures_narrower_after_backspace() {
        let before = FlexState {
            selected: vec![0, 0, 0],
            mode: FlexMode::Write,
            ..with_text("Hellp")
        };
        let after = after(before.clone(), &["\x7f"]);
        let before_width = measure(&before.boxes, &[0, 0]).width;
        let after_width = measure(&after.boxes, &[0, 0]).width;
        assert!(after_width < before_width);
        assert_eq!(
            after_width,
            measure(&with_text("Hell").boxes, &[0, 0]).width
        );
    }

    #[test]
    fn an_outer_box_with_text_spans_the_window() {
        let the_box = the_box(&placed(&with_text("Hello")));
        assert_eq!((the_box.x, the_box.width), (0, WINDOW.cols));
    }

    #[test]
    fn an_outer_box_follows_the_window() {
        let narrow = Cells { cols: 40, ..WINDOW };
        let placements = placements(&with_text("Hello"), narrow);
        assert_eq!(the_box(&placements).width, narrow.cols);
    }

    #[test]
    fn two_boxes_place_two_boxes_and_two_labels() {
        let placements = placed(&stacked(&["Hello", ""]));
        assert_eq!(
            (own_boxes(&placements).len(), texts(&placements).len()),
            (2, 2)
        );
    }

    #[test]
    fn three_boxes_stack_from_the_top_one_gap_apart() {
        let placements = placed(&stacked(&["Hello", "", "Hi"]));
        let boxes = own_boxes(&placements);
        assert_eq!(boxes[0].y, 0);
        assert_eq!(boxes[1].y, bottom(boxes[0]) + FLEX_SPACE.height);
        assert_eq!(boxes[2].y, bottom(boxes[1]) + FLEX_SPACE.height);
    }

    #[test]
    fn the_boxes_stay_at_the_top_after_a_resize() {
        let resized = Cells {
            cols: 100,
            rows: 40,
        };
        let placements = placements(&stacked(&["Hello", "", "Hi"]), resized);
        let boxes = own_boxes(&placements);
        assert_eq!(boxes[0].y, 0);
        assert_eq!(boxes[1].y, bottom(boxes[0]) + FLEX_SPACE.height);
    }

    #[test]
    fn stacked_boxes_line_up_by_their_centres() {
        let placements = placed(&stacked(&["Hello", ""]));
        let boxes = own_boxes(&placements);
        assert_eq!((boxes[0].width, boxes[1].width), (WINDOW.cols, WINDOW.cols));
        assert_eq!(
            2 * boxes[0].x + boxes[0].width,
            2 * boxes[1].x + boxes[1].width
        );
    }

    #[test]
    fn each_label_sits_inside_its_own_box() {
        let placements = placed(&stacked(&["Hello", "", "Hi"]));
        let boxes = own_boxes(&placements);
        let labels = label_rects(&placements);
        assert_eq!(boxes.len(), labels.len());
        for (the_box, label) in boxes.into_iter().zip(labels) {
            assert_inside(label, the_box);
        }
    }

    #[test]
    fn an_unfilled_box_is_not_filled() {
        let placements = placed(&with_text("Hello"));
        assert!(!box_placements(&placements)[1].1);
    }

    #[test]
    fn a_filled_top_level_box_is_filled() {
        let state = holding(
            FlexBox {
                filled: true,
                ..FlexBox::default()
            },
            &["Hello"],
        );
        let placements = placed(&state);
        assert!(box_placements(&placements)[1].1);
        assert!(!box_placements(&placements)[0].1);
    }

    #[test]
    fn an_inner_box_is_never_filled_with_its_outer_box() {
        let mut state = with_inner_boxes("Hello", 1);
        state.boxes.value_mut(&[0, 0]).filled = true;
        let filled: Vec<bool> = box_placements(&placed(&state))
            .into_iter()
            .map(|(_, filled)| filled)
            .collect();
        assert_eq!(filled, [false, true, false]);
    }

    #[test]
    fn a_box_measures_as_wide_as_its_texts_and_gaps() {
        for texts in [&["Hello", ""][..], &["Hello", "World"][..]] {
            let interiors: i64 = texts.iter().map(|text| interior(text)).sum();
            let gaps = FLEX_SPACE.width * (texts.len() as i64 - 1);
            assert_eq!(
                measure(&beside_row(texts).boxes, &[0, 0]).width,
                interiors + gaps
            );
        }
    }

    #[test]
    fn a_box_with_two_texts_places_one_box_and_two_labels() {
        let placements = placed(&beside_row(&["Hello", "World"]));
        assert_eq!(
            (own_boxes(&placements).len(), texts(&placements).len()),
            (1, 2)
        );
        let the_box = the_box(&placements);
        for label in label_rects(&placements) {
            assert_inside(label, the_box);
            assert_eq!(label.y, the_box.y + (the_box.height - label.height) / 2);
        }
    }

    #[test]
    fn the_second_label_is_one_gap_after_the_first() {
        let centred = justified(&["Hello", "World"], Justify::Center);
        let labels = label_rects(&placed(&centred));
        assert_eq!(labels[1].x, right(labels[0]) + FLEX_SPACE.width);
    }

    #[test]
    fn a_box_with_two_texts_stays_centred() {
        assert_centred(&placed(&beside(&["Hello", "World"])));
        assert_centred(&placed(&beside(&["Hello", ""])));
    }

    #[test]
    fn an_outer_box_with_two_texts_spans_the_window() {
        let placements = placed(&beside(&["Hello", "World"]));
        let the_box = the_box(&placements);
        assert_eq!(the_box.width, WINDOW.cols);
        assert_eq!(label_rects(&placements)[0].x, the_box.x);
    }

    fn justified(texts: &[&str], justify: Justify) -> FlexState {
        holding(FlexBox { justify, ..row() }, texts)
    }

    #[test]
    fn a_space_between_outer_box_spreads_its_texts_to_its_edges() {
        let placements = placed(&justified(&["Hello", "World"], Justify::SpaceBetween));
        let the_box = the_box(&placements);
        let labels = label_rects(&placements);
        assert_eq!(labels[0].x, the_box.x);
        assert_eq!(right(labels[1]), right(the_box));
    }

    #[test]
    fn a_space_between_outer_box_with_one_text_starts_it_at_its_left_edge() {
        let placements = placed(&justified(&["Hello"], Justify::SpaceBetween));
        assert_eq!(the_label(&placements).x, the_box(&placements).x);
    }

    #[test]
    fn a_start_outer_box_keeps_its_texts_one_gap_apart() {
        let placements = placed(&justified(&["Hello", "World"], Justify::Start));
        let the_box = the_box(&placements);
        let widths = share(the_box.width, 2, FLEX_SPACE.width);
        let labels = label_rects(&placements);
        assert_eq!(labels[0].x, the_box.x);
        assert_eq!(labels[1].x, the_box.x + widths[0] + FLEX_SPACE.width);
    }

    fn row_of_boxes(row: FlexBox, own_text: &[&str], box_texts: &[&str]) -> FlexState {
        let boxes: Vec<_> = box_texts
            .iter()
            .map(|label| outer(FlexBox::default(), &[label], 0))
            .collect();
        FlexState {
            boxes: new_canvas(vec![Tree::new(
                FlexBox {
                    direction: Direction::Row,
                    ..row
                },
                own_text
                    .iter()
                    .map(|label| text(label))
                    .chain(boxes)
                    .collect(),
            )]),
            ..FlexState::default()
        }
    }

    fn spreading() -> FlexBox {
        FlexBox {
            justify: Justify::SpaceBetween,
            ..FlexBox::default()
        }
    }

    fn gaps_between(row: &[CellRect]) -> Vec<i64> {
        row.windows(2)
            .map(|pair| pair[1].x - right(pair[0]))
            .collect()
    }

    fn free_space_of_row(state: &FlexState, box_count: usize) -> i64 {
        let widths: i64 = (0..box_count)
            .map(|index| measure(&state.boxes, &[0, 0, index]).width)
            .sum();
        the_box(&placed(state)).width - widths
    }

    #[test]
    fn a_space_between_row_spreads_two_boxes_to_its_edges() {
        let state = FlexState {
            boxes: new_canvas(vec![outer(
                FlexBox {
                    justify: Justify::SpaceBetween,
                    ..row()
                },
                &[],
                2,
            )]),
            ..FlexState::default()
        };
        let boxes = own_boxes(&placed(&state));
        let (row, first, last) = (boxes[0], boxes[1], boxes[2]);
        assert_eq!(first.x, row.x);
        assert_eq!(right(last), right(row));
        assert_eq!(first.width, measure(&state.boxes, &[0, 0, 0]).width);
        assert_eq!(last.width, measure(&state.boxes, &[0, 0, 1]).width);
    }

    #[test]
    fn a_space_between_row_with_one_box_keeps_it_its_width_at_the_left_edge() {
        let state = row_of_boxes(spreading(), &[], &["Hello"]);
        let boxes = own_boxes(&placed(&state));
        assert_eq!(boxes[1].x, boxes[0].x);
        assert_eq!(boxes[1].width, measure(&state.boxes, &[0, 0, 0]).width);
    }

    #[test]
    fn a_space_between_row_with_three_boxes_has_equal_gaps_when_the_free_space_divides_evenly() {
        let state = row_of_boxes(spreading(), &[], &["Hello", "Hello", "Hi"]);
        assert_eq!(free_space_of_row(&state, 3) % 2, 0);
        let boxes = own_boxes(&placed(&state));
        let (row, items) = (boxes[0], &boxes[1..]);
        assert_eq!(items[0].x, row.x);
        assert_eq!(right(items[2]), right(row));
        let gaps = gaps_between(items);
        assert_eq!(gaps[0], gaps[1]);
    }

    #[test]
    fn a_space_between_row_with_an_odd_remainder_keeps_both_edges_and_gaps_within_one() {
        let state = row_of_boxes(spreading(), &[], &["Hello", "Hello", "Hello"]);
        assert_eq!(free_space_of_row(&state, 3) % 2, 1);
        let boxes = own_boxes(&placed(&state));
        let (row, items) = (boxes[0], &boxes[1..]);
        assert_eq!(items[0].x, row.x);
        assert_eq!(right(items[2]), right(row));
        let gaps = gaps_between(items);
        assert!((gaps[0] - gaps[1]).abs() <= 1);
    }

    #[test]
    fn a_space_between_row_spreads_its_own_text_with_its_boxes() {
        let state = row_of_boxes(spreading(), &["Title"], &["Hello", "World"]);
        let placements = placed(&state);
        let boxes = own_boxes(&placements);
        let (row, items) = (boxes[0], &boxes[1..]);
        let label = the_label(&placements);
        assert_eq!(label.x, row.x);
        assert_eq!(right(items[1]), right(row));
        let gaps = [items[0].x - right(label), items[1].x - right(items[0])];
        assert!((gaps[0] - gaps[1]).abs() <= 1);
    }

    #[test]
    fn a_start_row_gives_each_of_two_boxes_half_of_its_width() {
        let state = row_of_boxes(FlexBox::default(), &[], &["Hello", "Hi"]);
        let boxes = own_boxes(&placed(&state));
        assert_eq!(
            boxes[1].width + FLEX_SPACE.width + boxes[2].width,
            boxes[0].width
        );
        assert!((boxes[1].width - boxes[2].width).abs() <= 1);
        assert_eq!(boxes[2].x, right(boxes[1]) + FLEX_SPACE.width);
    }

    #[test]
    fn pressing_s_twice_on_a_row_gives_the_boxes_their_equal_shares_again() {
        let state = FlexState {
            mode: FlexMode::Move,
            selected: vec![0, 0],
            ..row_of_boxes(FlexBox::default(), &[], &["Hello", "Hi"])
        };
        let before = placed(&state);
        let back = after(state, &["s", "s"]);
        assert_eq!(placed(&back), before);
    }

    #[test]
    fn an_inner_box_is_placed_like_an_empty_box() {
        let placements = placed(&with_inner_boxes("Hello", 1));
        assert_eq!(box_placements(&placements).len(), 3);
        assert!(!box_placements(&placements)[2].1);
    }

    #[test]
    fn the_last_inner_box_ends_at_the_outer_bottom_edge() {
        let boxes = own_boxes(&placed(&with_inner_boxes("Hello", 2)));
        assert_eq!(bottom(boxes[2]), bottom(boxes[0]));
    }

    #[test]
    fn an_inner_box_spans_the_outer_width() {
        for text in ["", "a"] {
            let boxes = own_boxes(&placed(&with_inner_boxes(text, 1)));
            assert_eq!((boxes[1].x, boxes[1].width), (boxes[0].x, boxes[0].width));
        }
    }

    #[test]
    fn a_following_outer_box_starts_one_gap_below_the_taller_one() {
        let state = FlexState {
            boxes: new_canvas(vec![
                outer(FlexBox::default(), &["Hello"], 2),
                outer(FlexBox::default(), &["Hello"], 0),
            ]),
            ..FlexState::default()
        };
        let boxes = own_boxes(&placed(&state));
        assert_eq!(boxes[3].y, bottom(boxes[0]) + FLEX_SPACE.height);
    }

    #[test]
    fn a_column_stretches_inner_boxes_across_the_parent() {
        let mut state = with_inner_boxes("Hello", 1);
        state.boxes.value_mut(&[0, 0]).direction = Direction::Column;
        let boxes = own_boxes(&placed(&state));
        assert_eq!((boxes[1].x, boxes[1].width), (boxes[0].x, boxes[0].width));
    }

    #[test]
    fn a_row_places_siblings_one_gap_apart_in_order() {
        let centred = FlexBox {
            justify: Justify::Center,
            ..row()
        };
        let state = FlexState {
            boxes: new_canvas(vec![Tree::new(
                centred,
                vec![text("Hello"), new_box(), text("World")],
            )]),
            ..FlexState::default()
        };
        let placements = placed(&state);
        let labels = label_rects(&placements);
        let inner = own_boxes(&placements)[1];
        assert_eq!(inner.x, right(labels[0]) + FLEX_SPACE.width);
        assert_eq!(labels[1].x, right(inner) + FLEX_SPACE.width);
    }

    fn selecting(mode: FlexMode, selected: &[usize], state: FlexState) -> FlexState {
        FlexState {
            mode,
            selected: selected.to_vec(),
            ..state
        }
    }

    #[test]
    fn in_move_the_selected_box_is_outlined_one_cell_outside_its_own_border() {
        let state = selecting(FlexMode::Move, &[0, 0], with_text("Hello"));
        let placements = placed(&state);
        let the_box = the_box(&placements);
        assert_eq!(
            outlines(&placements),
            vec![CellRect {
                x: the_box.x - OUTLINE_MARGIN,
                y: the_box.y - OUTLINE_MARGIN,
                width: the_box.width + 2 * OUTLINE_MARGIN,
                height: the_box.height + 2 * OUTLINE_MARGIN,
            }]
        );
    }

    #[test]
    fn in_move_a_selected_borderless_text_is_still_outlined() {
        let state = selecting(FlexMode::Move, &[0, 0, 0], with_text("Hello"));
        let placements = placed(&state);
        let the_box = the_box(&placements);
        let (label_x, label_y) = texts(&placements)[0].0;
        let [outline] = <[_; 1]>::try_from(outlines(&placements)).unwrap();
        assert_eq!(
            (outline.x, outline.y),
            (label_x - OUTLINE_MARGIN, label_y - OUTLINE_MARGIN)
        );
        assert_eq!(
            (outline.width, outline.height),
            (
                the_box.width + 2 * OUTLINE_MARGIN,
                the_box.height + 2 * OUTLINE_MARGIN
            )
        );
        assert_eq!(box_placements(&placements).len(), 2);
    }

    #[test]
    fn in_move_only_the_selected_box_is_outlined() {
        let state = selecting(FlexMode::Move, &[0, 1], stacked(&["Hello", "Hi", ""]));
        let placements = placed(&state);
        assert_eq!(outlines(&placements).len(), 1);
        assert_eq!(box_placements(&placements).len(), 4);
    }

    #[test]
    fn in_move_the_outline_follows_the_selection_to_another_box() {
        let state = selecting(FlexMode::Move, &[0, 2], stacked(&["Hello", "Hi", ""]));
        let placements = placed(&state);
        assert_eq!(
            own_boxes(&placements)[2].y - OUTLINE_MARGIN,
            outlines(&placements)[0].y
        );
    }

    #[test]
    fn in_write_the_selected_box_is_not_outlined() {
        let state = selecting(FlexMode::Write, &[0, 0], with_text("Hello"));
        assert!(outlines(&placed(&state)).is_empty());
    }

    #[test]
    fn the_outline_is_placed_after_everything_else() {
        let state = selecting(FlexMode::Move, &[0, 0], with_inner_boxes("Hello", 2));
        let placements = placed(&state);
        assert!(placements.len() > 1);
        let (last, rest) = placements.split_last().unwrap();
        assert!(matches!(last, Placement::Outline { .. }));
        assert!(rest
            .iter()
            .all(|placement| !matches!(placement, Placement::Outline { .. })));
    }

    #[test]
    fn the_outline_stays_last_when_a_box_painted_later_is_selected() {
        let state = selecting(FlexMode::Move, &[0, 0], stacked(&["Hello", "Hi"]));
        let placements = placed(&state);
        assert!(matches!(
            placements.last().unwrap(),
            Placement::Outline { .. }
        ));
    }

    fn in_replace(text: &str) -> FlexState {
        after(
            selecting(FlexMode::Move, &[0, 0, 0], with_text(text)),
            &["i"],
        )
    }

    #[test]
    fn pressing_i_on_a_box_with_text_highlights_the_label() {
        let placements = placed(&in_replace("Hello"));
        let label = the_label(&placements);
        assert_eq!(
            highlights(&placements),
            vec![CellRect {
                x: label.x,
                y: label.y,
                width: "Hello".chars().count() as i64,
                height: 1
            }]
        );
    }

    #[test]
    fn the_replace_highlight_comes_before_the_label() {
        let placements = placed(&in_replace("Hello"));
        let highlight = placements
            .iter()
            .position(|placement| matches!(placement, Placement::Highlight { .. }))
            .unwrap();
        let label = placements
            .iter()
            .position(|placement| matches!(placement, Placement::Text { .. }))
            .unwrap();
        assert!(highlight < label);
    }

    #[test]
    fn the_replace_highlight_width_counts_characters_not_bytes() {
        let placements = placed(&in_replace("café"));
        let width = highlights(&placements)[0].width;
        assert_eq!(width, "café".chars().count() as i64);
        assert_ne!(width, "café".len() as i64);
    }

    #[test]
    fn pressing_i_on_a_box_with_no_text_leaves_no_highlight() {
        assert!(highlights(&placed(&in_replace(""))).is_empty());
    }

    #[test]
    fn in_replace_the_selected_text_is_highlighted_and_not_outlined() {
        let placements = placed(&in_replace("Hello"));
        assert_eq!(highlights(&placements).len(), 1);
        assert!(outlines(&placements).is_empty());
    }

    #[test]
    fn after_the_first_replace_key_there_is_no_highlight() {
        let state = after(
            selecting(FlexMode::Move, &[0, 0, 0], with_text("Hello")),
            &["i", "W"],
        );
        assert!(highlights(&placed(&state)).is_empty());
    }

    fn writing(state: FlexState) -> FlexState {
        selecting(FlexMode::Write, &[0, 0, 0], state)
    }

    #[test]
    fn write_mode_puts_a_caret_one_cell_past_the_label() {
        let placements = placed(&writing(with_text("Hello")));
        let label = the_label(&placements);
        assert_eq!(
            carets(&placements),
            vec![(label.x + "Hello".chars().count() as i64, label.y)]
        );
    }

    #[test]
    fn an_empty_label_puts_the_caret_in_the_first_cell() {
        let placements = placed(&writing(with_text("")));
        let label = the_label(&placements);
        assert_eq!(carets(&placements), vec![(label.x, label.y)]);
    }

    #[test]
    fn typing_the_next_character_moves_the_caret_right_by_one_cell() {
        let before = writing(with_text("Hello"));
        let (before_x, before_y) = carets(&placed(&before))[0];
        let typed = after(before, &["x"]);
        assert_eq!(carets(&placed(&typed)), vec![(before_x + 1, before_y)]);
    }

    #[test]
    fn the_caret_counts_characters_not_bytes() {
        let placements = placed(&writing(with_text("café")));
        let label = the_label(&placements);
        assert_eq!(carets(&placements)[0].0, label.x + 4);
    }

    #[test]
    fn move_mode_places_no_caret() {
        assert!(carets(&placed(&with_text("Hello"))).is_empty());
    }

    #[test]
    fn a_text_measures_its_interior_by_one_row() {
        let tree = new_canvas(vec![Tree::new(FlexBox::default(), vec![text("Hello")])]);
        assert_eq!(
            measure(&tree, &[0, 0, 0]),
            Size {
                width: interior("Hello"),
                height: 1,
            }
        );
    }

    #[test]
    fn a_borderless_text_measures_its_padding() {
        let padding = 3_u16;
        let plain = measure(&text("Hi"), &[]);
        let padded = Tree::leaf(FlexBox {
            padding,
            ..text("Hi").value(&[]).clone()
        });
        let units = i64::from(padding);
        assert_eq!(
            measure(&padded, &[]),
            Size {
                width: plain.width + 2 * units * FLEX_SPACE.width,
                height: plain.height + 2 * units * FLEX_SPACE.height,
            }
        );
    }

    #[test]
    fn a_new_box_measures_with_no_padding() {
        assert_eq!(
            measure(&new_canvas(vec![new_box()]), &[0, 0]),
            Size {
                width: 0,
                height: 0
            }
        );
    }

    #[test]
    fn a_row_measures_its_children_side_by_side_with_gaps() {
        let tree = new_canvas(vec![Tree::new(
            row(),
            vec![text("Hello"), new_box(), text("World")],
        )]);
        assert_eq!(
            measure(&tree, &[0, 0]),
            Size {
                width: interior("Hello") + FLEX_SPACE.width + FLEX_SPACE.width + interior("World"),
                height: 1,
            }
        );
    }

    #[test]
    fn a_column_measures_its_children_stacked_with_gaps() {
        let tree = new_canvas(vec![Tree::new(
            FlexBox::default(),
            vec![text("Hello"), new_box(), text("World")],
        )]);
        assert_eq!(
            measure(&tree, &[0, 0]),
            Size {
                width: interior("Hello").max(interior("World")),
                height: 1 + FLEX_SPACE.height + FLEX_SPACE.height + 1,
            }
        );
    }

    #[test]
    fn start_places_children_a_gap_apart() {
        assert_eq!(
            distribute(&[2, 3, 4], 20, Justify::Start, FLEX_SPACE.width),
            vec![
                0,
                2 + FLEX_SPACE.width,
                2 + FLEX_SPACE.width + 3 + FLEX_SPACE.width
            ]
        );
    }

    #[test]
    fn start_places_no_children_for_an_empty_row() {
        assert_eq!(
            distribute(&[], 10, Justify::Start, FLEX_SPACE.width),
            Vec::<i64>::new()
        );
    }

    #[test]
    fn space_between_splits_free_space_evenly_between_children() {
        let gap = 4;
        let room = 2 + gap + 3 + gap + 4;
        assert_eq!(
            distribute(&[2, 3, 4], room, Justify::SpaceBetween, FLEX_SPACE.width),
            vec![0, 2 + gap, 2 + gap + 3 + gap]
        );
    }

    #[test]
    fn space_between_gives_the_remainder_to_the_last_gaps() {
        let gap = 2;
        let room = 2 + gap + 3 + (gap + 1) + 4 + (gap + 1) + 5;
        assert_eq!(
            distribute(&[2, 3, 4, 5], room, Justify::SpaceBetween, FLEX_SPACE.width),
            vec![
                0,
                2 + gap,
                2 + gap + 3 + gap + 1,
                2 + gap + 3 + gap + 1 + 4 + gap + 1
            ]
        );
    }

    #[test]
    fn space_between_places_a_single_child_at_the_row_start() {
        assert_eq!(
            distribute(&[5], 10, Justify::SpaceBetween, FLEX_SPACE.width),
            vec![0]
        );
    }

    #[test]
    fn center_splits_the_free_room_evenly_before_and_after_the_children() {
        let side = 4;
        let total = 2 + FLEX_SPACE.width + 3;
        assert_eq!(
            distribute(
                &[2, 3],
                side + total + side,
                Justify::Center,
                FLEX_SPACE.width
            ),
            vec![side, side + 2 + FLEX_SPACE.width]
        );
    }

    #[test]
    fn center_leans_odd_leftovers_left() {
        let side = 4;
        assert_eq!(
            distribute(&[5], side + 5 + side + 1, Justify::Center, FLEX_SPACE.width),
            vec![side]
        );
    }
}
