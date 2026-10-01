use std::borrow::Cow;

use types::Tree;

use crate::view::{self, Area, Label, Placement, PlacementNode, Rgb, Scene, ALL_SIDES};

use super::state::{Direction, FlexBox, FlexMode, FlexNode, FlexSize, FlexState, Justify};

const FLEX_BORDER: i64 = 1;
const FLEX_SPACE: Size = Size {
    width: 2,
    height: 1,
};
const FLEX_BORDER_COLOUR: Rgb = (0x2A, 0x2A, 0x2E);
const FLEX_SELECTED_COLOUR: Rgb = (0x8A, 0xB4, 0xF8);
const FLEX_TEXT_COLOUR: Rgb = (0xC9, 0xC9, 0xCF);
const FLEX_FILL_COLOUR: Rgb = (0x14, 0x14, 0x16);

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
    if flex_box.bare {
        Size {
            width: 0,
            height: 0,
        }
    } else {
        FLEX_SPACE
    }
}

fn is_full_across(node: &FlexNode, direction: Direction) -> bool {
    let FlexNode::Box(flex_box) = node else {
        return false;
    };
    let size = match direction {
        Direction::Row => flex_box.height,
        Direction::Column => flex_box.width,
    };
    size == FlexSize::Full
}

fn measure(tree: &Tree<FlexNode>, path: &[usize]) -> Size {
    match tree.value(path) {
        FlexNode::Text(text) => Size {
            width: view::interior(text),
            height: 1,
        },
        FlexNode::Box(flex_box) => {
            let direction = flex_box.direction;
            let children: Vec<Size> = tree
                .children(path)
                .iter()
                .map(|child| measure(tree, child))
                .collect();
            let gaps = FLEX_SPACE.main(direction) * children.len().saturating_sub(1) as i64;
            let main = children
                .iter()
                .map(|child| child.main(direction))
                .sum::<i64>()
                + gaps;
            let cross = children
                .iter()
                .map(|child| child.cross(direction))
                .max()
                .unwrap_or(0);
            let content = Size::along(direction, main, cross);
            let padding = padding(flex_box);
            Size {
                width: content.width + 2 * padding.width,
                height: content.height + 2 * padding.height,
            }
        }
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

fn arrange(tree: &Tree<FlexNode>, path: &[usize], rect: Rect, out: &mut Vec<(Vec<usize>, Rect)>) {
    out.push((path.to_vec(), rect));
    let FlexNode::Box(flex_box) = tree.value(path) else {
        return;
    };
    let direction = flex_box.direction;
    let children = tree.children(path);
    let sizes: Vec<Size> = children.iter().map(|child| measure(tree, child)).collect();
    let mains: Vec<i64> = sizes.iter().map(|size| size.main(direction)).collect();
    let inner = rect.inner(padding(flex_box));
    let inner_cross = inner.size().cross(direction);
    let offsets = distribute(
        &mains,
        inner.size().main(direction),
        flex_box.justify,
        FLEX_SPACE.main(direction),
    );
    for ((child, size), offset) in children.iter().zip(sizes).zip(offsets) {
        let (cross, cross_offset) = if is_full_across(tree.value(child), direction) {
            (inner_cross, 0)
        } else {
            let cross = size.cross(direction);
            (cross, (inner_cross - cross) / 2)
        };
        let child_rect = Rect::along(
            direction,
            inner.main_start(direction) + offset,
            inner.cross_start(direction) + cross_offset,
            Size::along(direction, size.main(direction), cross),
        );
        arrange(tree, child, child_rect, out);
    }
}

fn paint<'a>(state: &FlexState, path: &[usize], node: &'a FlexNode, rect: Rect) -> Placement<'a> {
    let selected = state.mode == FlexMode::Move && state.selected == path;
    let node = match node {
        FlexNode::Text(text) => PlacementNode::Label(Label {
            text: Cow::Borrowed(text),
            colour: if selected {
                FLEX_SELECTED_COLOUR
            } else {
                FLEX_TEXT_COLOUR
            },
            bold: false,
        }),
        FlexNode::Box(flex_box) => {
            let outer = path.len() == 1;
            PlacementNode::Box {
                colour: if selected {
                    FLEX_SELECTED_COLOUR
                } else {
                    FLEX_BORDER_COLOUR
                },
                fill: None,
                opacity: None,
                solid_fill: (outer && flex_box.filled).then_some(FLEX_FILL_COLOUR),
                rounded: false,
                sides: ALL_SIDES,
                border: FLEX_BORDER,
            }
        }
    };
    Placement {
        node,
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        depth: (path.len() - 1) as u8,
    }
}

pub(crate) fn scene(state: &FlexState, window: Area) -> Scene<'_> {
    let window_rect = Rect {
        x: window.col,
        y: window.row,
        width: window.cols,
        height: window.rows,
    };
    let mut rects = Vec::new();
    arrange(&state.boxes, &[], window_rect, &mut rects);
    let placements = rects
        .into_iter()
        .filter_map(|(path, rect)| match state.boxes.value(&path) {
            FlexNode::Box(FlexBox { bare: true, .. }) => None,
            node => Some(paint(state, &path, node, rect)),
        })
        .collect();
    vec![(window, placements)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flex::state::{new_box, reduce, FlexBox, Justify};
    use crate::view::BOX_HEIGHT;
    use types::Tree;

    const WINDOW: Area = Area {
        col: 0,
        row: 0,
        cols: 80,
        rows: 24,
    };

    fn outer(the_box: FlexBox, texts: &[&str], inner_boxes: usize) -> Tree<FlexNode> {
        Tree::new(
            FlexNode::Box(the_box),
            texts
                .iter()
                .map(|text| Tree::leaf(FlexNode::Text(text.to_string())))
                .chain((0..inner_boxes).map(|_| new_box()))
                .collect(),
        )
    }

    fn holding(the_box: FlexBox, texts: &[&str]) -> FlexState {
        FlexState {
            boxes: Tree::root(vec![outer(the_box, texts, 0)]),
            ..FlexState::default()
        }
    }

    fn with_text(text: &str) -> FlexState {
        holding(FlexBox::default(), &[text])
    }

    fn outer_box_mut(state: &mut FlexState, index: usize) -> &mut FlexBox {
        let FlexNode::Box(flex_box) = state.boxes.value_mut(&[index]) else {
            panic!("no box at {index}")
        };
        flex_box
    }

    fn placements(state: &FlexState) -> Vec<Placement<'_>> {
        let [(area, placements)] = <[_; 1]>::try_from(scene(state, WINDOW)).unwrap();
        assert_eq!(area, WINDOW);
        placements
    }

    fn the_box<'a>(placements: &'a [Placement<'a>]) -> &'a Placement<'a> {
        placements
            .iter()
            .find(|placement| matches!(placement.node, PlacementNode::Box { .. }))
            .unwrap()
    }

    fn the_label<'a>(placements: &'a [Placement<'a>]) -> &'a Placement<'a> {
        placements
            .iter()
            .find(|placement| matches!(placement.node, PlacementNode::Label(_)))
            .unwrap()
    }

    #[test]
    fn an_empty_box_is_as_wide_and_high_as_its_padding() {
        let state = FlexState::default();
        let placements = placements(&state);
        let the_box = the_box(&placements);
        assert_eq!(the_box.width, 2 * FLEX_SPACE.width);
        assert_eq!(the_box.height, 2 * FLEX_SPACE.height);
    }

    #[test]
    fn a_box_pads_its_content_by_the_space_width_on_the_left_and_right() {
        let state = with_text("Hello");
        let placements = placements(&state);
        let the_box = the_box(&placements);
        let label = the_label(&placements);
        assert_eq!(label.x, the_box.x + FLEX_SPACE.width);
        assert_eq!(
            label.x + label.width,
            the_box.x + the_box.width - FLEX_SPACE.width
        );
    }

    #[test]
    fn a_box_pads_its_content_by_the_space_height_above_and_below() {
        let state = with_text("Hello");
        let placements = placements(&state);
        let the_box = the_box(&placements);
        let label = the_label(&placements);
        assert_eq!(label.y, the_box.y + FLEX_SPACE.height);
        assert_eq!(
            label.y + label.height,
            the_box.y + the_box.height - FLEX_SPACE.height
        );
    }

    #[test]
    fn the_label_sits_inside_the_box() {
        let state = with_text("");
        let placements = placements(&state);
        let the_box = the_box(&placements);
        let label = the_label(&placements);
        assert!(label.x > the_box.x);
        assert!(label.x + label.width < the_box.x + the_box.width);
        assert!(label.y > the_box.y);
        assert!(label.y + label.height < the_box.y + the_box.height);
    }

    #[test]
    fn only_the_box_and_its_label_are_placed() {
        let state = with_text("");
        let placements = placements(&state);
        let boxes = placements
            .iter()
            .filter(|placement| matches!(placement.node, PlacementNode::Box { .. }))
            .count();
        let labels = placements
            .iter()
            .filter(|placement| matches!(placement.node, PlacementNode::Label(_)))
            .count();
        assert_eq!((boxes, labels, placements.len()), (1, 1, 2));
    }

    #[test]
    fn the_box_grows_to_fit_the_text() {
        let state = with_text("Hello");
        let placements = placements(&state);
        assert_eq!(
            the_box(&placements).width,
            view::interior("Hello") + 2 * FLEX_SPACE.width
        );
    }

    #[test]
    fn the_label_shows_the_text_inside_the_box() {
        let state = with_text("Hello");
        let placements = placements(&state);
        let the_box = the_box(&placements);
        let label = the_label(&placements);
        let PlacementNode::Label(Label { text, .. }) = &label.node else {
            unreachable!()
        };
        assert_eq!(text, state.texts_of(&[state.outer_boxes().count() - 1])[0]);
        assert!(label.x > the_box.x);
        assert!(label.x + label.width < the_box.x + the_box.width);
    }

    #[test]
    fn the_box_shrinks_after_backspace() {
        let (before, _) = reduce(with_text("Hellp"), "i");
        let (after, _) = reduce(before.clone(), "\x7f");
        let before_width = the_box(&placements(&before)).width;
        let after_width = the_box(&placements(&after)).width;
        assert!(after_width < before_width);
        assert_eq!(after_width, the_box(&placements(&with_text("Hell"))).width);
    }

    fn colours(placements: &[Placement<'_>]) -> (Rgb, Rgb) {
        let PlacementNode::Box { colour: border, .. } = the_box(placements).node else {
            unreachable!()
        };
        let PlacementNode::Label(Label { colour: text, .. }) = &the_label(placements).node else {
            unreachable!()
        };
        (border, *text)
    }

    #[test]
    fn an_empty_box_wears_its_own_colours() {
        let (state, _) = reduce(FlexState::default(), "i");
        assert_eq!(
            colours(&placements(&state)),
            ((0x2A, 0x2A, 0x2E), (0xC9, 0xC9, 0xCF))
        );
    }

    fn sides_and_border(placement: &Placement<'_>) -> (view::Sides, i64) {
        let PlacementNode::Box { sides, border, .. } = placement.node else {
            unreachable!()
        };
        (sides, border)
    }

    #[test]
    fn an_empty_box_has_a_thin_border_on_all_sides() {
        let state = with_text("");
        let placements = placements(&state);
        assert_eq!(
            sides_and_border(the_box(&placements)),
            (ALL_SIDES, FLEX_BORDER)
        );
    }

    #[test]
    fn the_colours_stay_while_the_box_grows_and_shrinks() {
        let (grown, _) = reduce(with_text("Hello"), "i");
        let (shrunk, _) = reduce(reduce(with_text("Hellp"), "i").0, "\x7f");
        for state in [&grown, &shrunk] {
            assert_eq!(
                colours(&placements(state)),
                ((0x2A, 0x2A, 0x2E), (0xC9, 0xC9, 0xCF))
            );
        }
    }

    #[test]
    fn the_border_stays_thin_as_the_box_grows() {
        let state = with_text("Hello");
        let placements = placements(&state);
        let the_box = the_box(&placements);
        assert_eq!(
            the_box.width,
            view::interior("Hello") + 2 * FLEX_SPACE.width
        );
        assert_eq!(sides_and_border(the_box).1, FLEX_BORDER);
    }

    #[test]
    fn the_border_stays_thin_after_backspace() {
        let (after, _) = reduce(with_text("Hellp"), "\x7f");
        let placements = placements(&after);
        assert_eq!(sides_and_border(the_box(&placements)).1, FLEX_BORDER);
    }

    fn full(text: &str) -> FlexState {
        holding(
            FlexBox {
                width: FlexSize::Full,
                ..FlexBox::default()
            },
            &[text],
        )
    }

    #[test]
    fn a_full_box_spans_the_window() {
        let state = full("Hello");
        let placements = placements(&state);
        let the_box = the_box(&placements);
        assert_eq!(the_box.x, 0);
        assert_eq!(the_box.width, WINDOW.cols);
    }

    #[test]
    fn a_full_box_starts_its_label_one_space_in_from_its_edge() {
        let state = full("Hello");
        let placements = placements(&state);
        assert_eq!(
            the_label(&placements).x,
            the_box(&placements).x + FLEX_SPACE.width
        );
    }

    #[test]
    fn a_full_box_follows_the_window() {
        let narrow = Area { cols: 40, ..WINDOW };
        let state = full("Hello");
        let [(_, placements)] = <[_; 1]>::try_from(scene(&state, narrow)).unwrap();
        assert_eq!(the_box(&placements).width, narrow.cols);
    }

    fn stacked(texts: &[&str]) -> FlexState {
        FlexState {
            boxes: Tree::root(
                texts
                    .iter()
                    .map(|text| outer(FlexBox::default(), &[text], 0))
                    .collect(),
            ),
            ..FlexState::default()
        }
    }

    fn all_boxes<'a>(placements: &'a [Placement<'a>]) -> Vec<&'a Placement<'a>> {
        placements
            .iter()
            .filter(|placement| matches!(placement.node, PlacementNode::Box { .. }))
            .collect()
    }

    fn all_labels<'a>(placements: &'a [Placement<'a>]) -> Vec<&'a Placement<'a>> {
        placements
            .iter()
            .filter(|placement| matches!(placement.node, PlacementNode::Label(_)))
            .collect()
    }

    #[test]
    fn two_boxes_place_two_boxes_and_two_labels() {
        let state = stacked(&["Hello", ""]);
        let placements = placements(&state);
        assert_eq!(
            (all_boxes(&placements).len(), all_labels(&placements).len()),
            (2, 2)
        );
    }

    #[test]
    fn the_second_box_sits_one_gap_below_the_first() {
        let state = stacked(&["Hello", ""]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        assert_eq!(boxes[1].y, boxes[0].y + BOX_HEIGHT + FLEX_SPACE.height);
    }

    #[test]
    fn stacked_boxes_line_up_by_their_centres() {
        let state = stacked(&["Hello", ""]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        assert_eq!(
            (boxes[0].width, boxes[1].width),
            (
                view::interior("Hello") + 2 * FLEX_SPACE.width,
                view::interior("") + 2 * FLEX_SPACE.width
            )
        );
        assert_eq!(
            2 * boxes[0].x + boxes[0].width,
            2 * boxes[1].x + boxes[1].width
        );
    }

    #[test]
    fn odd_widths_lean_left() {
        let state = stacked(&["Hi", ""]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        assert_eq!(
            (boxes[0].width, boxes[1].width),
            (
                view::interior("Hi") + 2 * FLEX_SPACE.width,
                view::interior("") + 2 * FLEX_SPACE.width
            )
        );
        for the_box in boxes {
            let left_margin = the_box.x - WINDOW.col;
            let right_margin = WINDOW.col + WINDOW.cols - (the_box.x + the_box.width);
            assert!(
                (0..=1).contains(&(right_margin - left_margin)),
                "{left_margin} {right_margin}"
            );
        }
    }

    #[test]
    fn the_stack_is_centred_in_the_window() {
        let state = stacked(&["Hello", "", ""]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        let left = boxes.iter().map(|b| b.x).min().unwrap();
        let right = boxes.iter().map(|b| b.x + b.width).max().unwrap();
        let top = boxes.iter().map(|b| b.y).min().unwrap();
        let bottom = boxes.iter().map(|b| b.y + b.height).max().unwrap();
        let left_margin = left - WINDOW.col;
        let right_margin = WINDOW.col + WINDOW.cols - right;
        let top_margin = top - WINDOW.row;
        let bottom_margin = WINDOW.row + WINDOW.rows - bottom;
        assert!((left_margin - right_margin).abs() <= 1);
        assert!((top_margin - bottom_margin).abs() <= 1);
    }

    #[test]
    fn each_label_sits_inside_its_own_box() {
        let state = stacked(&["Hello", "", "Hi"]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        let labels = all_labels(&placements);
        assert_eq!(boxes.len(), labels.len());
        for (the_box, label) in boxes.iter().zip(&labels) {
            assert!(label.x > the_box.x);
            assert!(label.x + label.width < the_box.x + the_box.width);
            assert!(label.y > the_box.y);
            assert!(label.y + label.height < the_box.y + the_box.height);
        }
    }

    fn solid_fill(placement: &Placement<'_>) -> Option<Rgb> {
        let PlacementNode::Box { solid_fill, .. } = placement.node else {
            unreachable!()
        };
        solid_fill
    }

    #[test]
    fn an_unfilled_box_has_no_solid_fill() {
        let state = with_text("Hello");
        let placements = placements(&state);
        assert_eq!(solid_fill(the_box(&placements)), None);
    }

    #[test]
    fn a_filled_box_is_solid_with_the_flex_fill_colour() {
        let state = holding(
            FlexBox {
                filled: true,
                ..FlexBox::default()
            },
            &["Hello"],
        );
        let placements = placements(&state);
        assert_eq!(solid_fill(the_box(&placements)), Some(FLEX_FILL_COLOUR));
    }

    fn beside(texts: &[&str]) -> FlexState {
        holding(FlexBox::default(), texts)
    }

    fn margins(placements: &[Placement<'_>]) -> (i64, i64) {
        let the_box = the_box(placements);
        (
            the_box.x - WINDOW.col,
            WINDOW.col + WINDOW.cols - (the_box.x + the_box.width),
        )
    }

    #[test]
    fn a_fit_box_is_as_wide_as_its_texts_and_gaps() {
        for texts in [&["Hello", ""][..], &["Hello", "World"][..]] {
            let state = beside(texts);
            let placements = placements(&state);
            let interiors: i64 = texts.iter().map(|text| view::interior(text)).sum();
            let gaps = FLEX_SPACE.width * (texts.len() as i64 - 1);
            assert_eq!(
                the_box(&placements).width,
                interiors + gaps + 2 * FLEX_SPACE.width
            );
        }
    }

    #[test]
    fn a_box_with_two_texts_places_one_box_and_two_labels() {
        let state = beside(&["Hello", "World"]);
        let placements = placements(&state);
        assert_eq!(
            (all_boxes(&placements).len(), all_labels(&placements).len()),
            (1, 2)
        );
        let the_box = the_box(&placements);
        for label in all_labels(&placements) {
            assert!(label.x > the_box.x);
            assert!(label.x + label.width < the_box.x + the_box.width);
            assert_eq!(label.y, the_box.y + BOX_HEIGHT / 2);
        }
    }

    #[test]
    fn the_second_label_is_one_gap_after_the_first() {
        let state = beside(&["Hello", "World"]);
        let placements = placements(&state);
        let labels = all_labels(&placements);
        assert_eq!(
            labels[1].x,
            labels[0].x + view::interior("Hello") + FLEX_SPACE.width
        );
    }

    #[test]
    fn a_box_with_two_texts_stays_centred() {
        let state = beside(&["Hello", "World"]);
        let (left, right) = margins(&placements(&state));
        assert!((left - right).abs() <= 1);
    }

    #[test]
    fn the_box_is_centred_again_after_a_text_is_added() {
        let (grown, _) = reduce(with_text("Hello"), "s");
        let (left, right) = margins(&placements(&grown));
        assert!((left - right).abs() <= 1);
    }

    #[test]
    fn a_full_box_with_two_texts_spans_the_window() {
        let state = holding(
            FlexBox {
                width: FlexSize::Full,
                ..FlexBox::default()
            },
            &["Hello", "World"],
        );
        let placements = placements(&state);
        let the_box = the_box(&placements);
        assert_eq!(the_box.width, WINDOW.cols);
        assert_eq!(all_labels(&placements)[0].x, the_box.x + FLEX_SPACE.width);
    }

    fn full_beside(texts: &[&str], justify: Justify) -> FlexState {
        holding(
            FlexBox {
                width: FlexSize::Full,
                justify,
                ..FlexBox::default()
            },
            texts,
        )
    }

    #[test]
    fn a_full_space_between_box_spreads_its_texts_to_the_borders() {
        let state = full_beside(&["Hello", "World"], Justify::SpaceBetween);
        let placements = placements(&state);
        let the_box = the_box(&placements);
        let labels = all_labels(&placements);
        let last = labels.last().unwrap();
        assert_eq!(labels[0].x, the_box.x + FLEX_SPACE.width);
        assert_eq!(
            last.x + last.width,
            the_box.x + the_box.width - FLEX_SPACE.width
        );
    }

    #[test]
    fn a_full_space_between_box_with_one_text_starts_it_one_space_in_from_its_edge() {
        let state = full_beside(&["Hello"], Justify::SpaceBetween);
        let placements = placements(&state);
        assert_eq!(
            the_label(&placements).x,
            the_box(&placements).x + FLEX_SPACE.width
        );
    }

    #[test]
    fn a_fit_space_between_box_places_the_same_as_a_fit_start_box() {
        let texts = &["Hello", "World"];
        let start = beside(texts);
        let spread = holding(
            FlexBox {
                justify: Justify::SpaceBetween,
                ..start.outer_boxes().next().unwrap().clone()
            },
            texts,
        );
        assert_eq!(placements(&spread), placements(&start));
    }

    #[test]
    fn a_full_start_box_keeps_its_texts_one_gap_apart() {
        let state = full_beside(&["Hello", "World"], Justify::Start);
        let placements = placements(&state);
        let the_box = the_box(&placements);
        let labels = all_labels(&placements);
        assert_eq!(labels[0].x, the_box.x + FLEX_SPACE.width);
        assert_eq!(
            labels[1].x,
            labels[0].x + labels[0].width + FLEX_SPACE.width
        );
    }

    fn borders(placements: &[Placement<'_>]) -> Vec<Rgb> {
        all_boxes(placements)
            .iter()
            .map(|placement| {
                let PlacementNode::Box { colour, .. } = placement.node else {
                    unreachable!()
                };
                colour
            })
            .collect()
    }

    fn selecting(mode: FlexMode, selected: usize) -> FlexState {
        FlexState {
            mode,
            selected: vec![selected],
            ..stacked(&["Hello", "Hi", ""])
        }
    }

    #[test]
    fn in_move_the_selected_box_has_the_selected_border() {
        let state = selecting(FlexMode::Move, 1);
        assert_eq!(
            borders(&placements(&state)),
            vec![FLEX_BORDER_COLOUR, FLEX_SELECTED_COLOUR, FLEX_BORDER_COLOUR]
        );
    }

    #[test]
    fn in_write_no_box_has_the_selected_border() {
        let state = selecting(FlexMode::Write, 1);
        assert_eq!(borders(&placements(&state)), vec![FLEX_BORDER_COLOUR; 3]);
    }

    fn with_inner_boxes(text: &str, count: usize) -> FlexState {
        FlexState {
            boxes: Tree::root(vec![outer(FlexBox::default(), &[text], count)]),
            ..FlexState::default()
        }
    }

    fn outer_boxes_with_inner_counts(counts: &[usize]) -> FlexState {
        FlexState {
            boxes: Tree::root(
                counts
                    .iter()
                    .map(|&count| outer(FlexBox::default(), &["Hello"], count))
                    .collect(),
            ),
            ..FlexState::default()
        }
    }

    #[test]
    fn an_inner_box_is_as_wide_as_an_empty_box() {
        let empty_state = FlexState::default();
        let empty = placements(&empty_state);
        let state = with_inner_boxes("Hello", 1);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        assert_eq!(boxes[1].width, the_box(&empty).width);
        assert_eq!(boxes[1].height, the_box(&empty).height);
    }

    #[test]
    fn an_inner_box_looks_like_an_empty_box() {
        let state = with_inner_boxes("Hello", 1);
        let placements = placements(&state);
        let inner = all_boxes(&placements)[1];
        let PlacementNode::Box {
            colour,
            fill,
            opacity,
            solid_fill,
            rounded,
            sides,
            border,
        } = inner.node
        else {
            unreachable!()
        };
        assert_eq!(colour, FLEX_BORDER_COLOUR);
        assert_eq!((fill, opacity, solid_fill), (None, None, None));
        assert!(!rounded);
        assert_eq!((sides, border), (ALL_SIDES, FLEX_BORDER));
    }

    #[test]
    fn an_inner_box_never_wears_the_fill_of_its_outer_box() {
        let mut state = with_inner_boxes("Hello", 1);
        outer_box_mut(&mut state, 0).filled = true;
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        assert_eq!(solid_fill(boxes[0]), Some(FLEX_FILL_COLOUR));
        assert_eq!(solid_fill(boxes[1]), None);
    }

    #[test]
    fn the_last_inner_box_ends_one_space_above_the_outer_bottom_edge() {
        let state = with_inner_boxes("Hello", 2);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        assert_eq!(
            boxes[2].y + boxes[2].height + FLEX_SPACE.height,
            boxes[0].y + boxes[0].height
        );
    }

    fn assert_inner_box_strictly_inside(state: &FlexState) {
        let placements = placements(state);
        let boxes = all_boxes(&placements);
        let (outer, inner) = (boxes[0], boxes[1]);
        assert!(inner.x > outer.x);
        assert!(inner.x + inner.width < outer.x + outer.width);
    }

    #[test]
    fn an_inner_box_in_a_fit_box_with_empty_text_stays_inside_the_outer_edges() {
        assert_inner_box_strictly_inside(&with_inner_boxes("", 1));
    }

    #[test]
    fn an_inner_box_in_a_fit_box_with_short_text_stays_inside_the_outer_edges() {
        assert_inner_box_strictly_inside(&with_inner_boxes("a", 1));
    }

    #[test]
    fn an_inner_box_in_a_full_start_box_with_short_text_stays_inside_the_outer_edges() {
        for text in ["", "a"] {
            let mut state = with_inner_boxes(text, 1);
            let outer = outer_box_mut(&mut state, 0);
            outer.width = FlexSize::Full;
            outer.justify = Justify::Start;
            assert_inner_box_strictly_inside(&state);
        }
    }

    #[test]
    fn a_following_outer_box_starts_one_gap_below_the_taller_one() {
        let state = outer_boxes_with_inner_counts(&[2, 0]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        let (first, second) = (boxes[0], boxes[3]);
        assert_eq!(second.y, first.y + first.height + FLEX_SPACE.height);
    }

    #[test]
    fn a_scene_with_inner_boxes_is_centred_in_the_window() {
        let state = outer_boxes_with_inner_counts(&[2, 1, 0]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        let left = boxes.iter().map(|b| b.x).min().unwrap();
        let right = boxes.iter().map(|b| b.x + b.width).max().unwrap();
        let top = boxes.iter().map(|b| b.y).min().unwrap();
        let bottom = boxes.iter().map(|b| b.y + b.height).max().unwrap();
        assert!(((left - WINDOW.col) - (WINDOW.col + WINDOW.cols - right)).abs() <= 1);
        assert!(((top - WINDOW.row) - (WINDOW.row + WINDOW.rows - bottom)).abs() <= 1);
    }

    #[test]
    fn in_move_an_inner_box_keeps_the_border_colour_when_its_outer_box_is_selected() {
        let state = FlexState {
            mode: FlexMode::Move,
            ..with_inner_boxes("Hello", 1)
        };
        assert_eq!(
            borders(&placements(&state)),
            vec![FLEX_SELECTED_COLOUR, FLEX_BORDER_COLOUR]
        );
    }

    #[test]
    fn in_move_a_selected_inner_box_has_the_selected_border() {
        let state = FlexState {
            mode: FlexMode::Move,
            selected: vec![0, 2],
            ..with_inner_boxes("Hello", 2)
        };
        assert_eq!(
            borders(&placements(&state)),
            vec![FLEX_BORDER_COLOUR, FLEX_BORDER_COLOUR, FLEX_SELECTED_COLOUR]
        );
    }

    #[test]
    fn in_write_a_selected_inner_box_keeps_the_border_colour() {
        let state = FlexState {
            mode: FlexMode::Write,
            selected: vec![0, 2],
            ..with_inner_boxes("Hello", 2)
        };
        assert_eq!(borders(&placements(&state)), vec![FLEX_BORDER_COLOUR; 3]);
    }

    fn box_of(children: Vec<Tree<FlexNode>>) -> Tree<FlexNode> {
        Tree::root(vec![Tree::new(FlexNode::Box(FlexBox::default()), children)])
    }

    fn text(text: &str) -> Tree<FlexNode> {
        Tree::leaf(FlexNode::Text(text.to_string()))
    }

    #[test]
    fn a_text_measures_its_interior_by_one_row() {
        let tree = box_of(vec![text("Hello")]);
        assert_eq!(
            measure(&tree, &[0, 0]),
            Size {
                width: view::interior("Hello"),
                height: 1,
            }
        );
    }

    #[test]
    fn a_new_box_measures_its_padding_on_both_sides() {
        assert_eq!(
            measure(&Tree::root(vec![new_box()]), &[0]),
            Size {
                width: 2 * FLEX_SPACE.width,
                height: 2 * FLEX_SPACE.height,
            }
        );
    }

    #[test]
    fn a_row_measures_its_children_side_by_side_with_gaps() {
        let tree = box_of(vec![text("Hello"), new_box(), text("World")]);
        let empty_box = measure(&Tree::root(vec![new_box()]), &[0]);
        assert_eq!(
            measure(&tree, &[0]),
            Size {
                width: view::interior("Hello")
                    + FLEX_SPACE.width
                    + empty_box.width
                    + FLEX_SPACE.width
                    + view::interior("World")
                    + 2 * FLEX_SPACE.width,
                height: empty_box.height + 2 * FLEX_SPACE.height,
            }
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
    fn start_places_a_single_child_at_the_row_start() {
        assert_eq!(
            distribute(&[5], 10, Justify::Start, FLEX_SPACE.width),
            vec![0]
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
    fn center_splits_the_free_room_evenly_before_and_after_the_children() {
        let lengths = [2, 3];
        let total = 2 + FLEX_SPACE.width + 3;
        let side = 4;
        assert_eq!(
            distribute(
                &lengths,
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

    #[test]
    fn space_between_places_a_single_child_at_the_row_start() {
        assert_eq!(
            distribute(&[5], 10, Justify::SpaceBetween, FLEX_SPACE.width),
            vec![0]
        );
    }

    fn hello_box_world() -> FlexState {
        FlexState {
            boxes: Tree::root(vec![Tree::new(
                FlexNode::Box(FlexBox::default()),
                vec![text("Hello"), new_box(), text("World")],
            )]),
            ..FlexState::default()
        }
    }

    #[test]
    fn a_column_measures_its_children_stacked_with_gaps() {
        let mut state = hello_box_world();
        if let FlexNode::Box(the_box) = state.boxes.value_mut(&[0]) {
            the_box.direction = Direction::Column;
        }
        let children: Vec<Size> = state
            .boxes
            .children(&[0])
            .iter()
            .map(|child| measure(&state.boxes, child))
            .collect();
        let widest = children.iter().map(|child| child.width).max().unwrap();
        let heights = children.iter().map(|child| child.height).sum::<i64>();
        assert_eq!(
            measure(&state.boxes, &[0]),
            Size {
                width: widest + 2 * FLEX_SPACE.width,
                height: heights
                    + FLEX_SPACE.height * (children.len() as i64 - 1)
                    + 2 * FLEX_SPACE.height,
            }
        );
    }

    fn label_showing<'a>(placements: &'a [Placement<'a>], shown: &str) -> &'a Placement<'a> {
        placements
            .iter()
            .find(|placement| {
                matches!(&placement.node, PlacementNode::Label(Label { text, .. }) if text == shown)
            })
            .unwrap()
    }

    #[test]
    fn siblings_sit_in_one_row_in_child_order_one_gap_apart() {
        let state = hello_box_world();
        let placements = placements(&state);
        let hello = label_showing(&placements, "Hello");
        let inner = all_boxes(&placements)[1];
        let world = label_showing(&placements, "World");
        assert_eq!(inner.x, hello.x + hello.width + FLEX_SPACE.width);
        assert_eq!(world.x, inner.x + inner.width + FLEX_SPACE.width);
    }

    #[test]
    fn texts_are_centred_vertically_on_the_tallest_sibling() {
        let state = hello_box_world();
        let placements = placements(&state);
        let inner = all_boxes(&placements)[1];
        for shown in ["Hello", "World"] {
            let label = label_showing(&placements, shown);
            assert_eq!(label.y, inner.y + (inner.height - label.height) / 2);
        }
    }

    #[test]
    fn the_outer_box_grows_to_fit_the_row_and_stays_centred() {
        let state = hello_box_world();
        let placements = placements(&state);
        let outer = all_boxes(&placements)[0];
        let measured = measure(&state.boxes, &[0]);
        assert_eq!(
            (outer.width, outer.height),
            (measured.width, measured.height)
        );
        let left = outer.x - WINDOW.col;
        let right = WINDOW.col + WINDOW.cols - (outer.x + outer.width);
        let top = outer.y - WINDOW.row;
        let bottom = WINDOW.row + WINDOW.rows - (outer.y + outer.height);
        assert!((left - right).abs() <= 1);
        assert!((top - bottom).abs() <= 1);
    }

    #[test]
    fn in_move_a_box_then_a_text_are_added_to_the_right_in_order() {
        let state = ["\r", "A", "s", "W", "o", "r", "l", "d"]
            .into_iter()
            .fold(with_text("Hello"), |state, key| reduce(state, key).0);
        let placements = placements(&state);
        let hello = label_showing(&placements, "Hello");
        let inner = all_boxes(&placements)[1];
        let world = label_showing(&placements, "World");
        assert_eq!(inner.x, hello.x + hello.width + FLEX_SPACE.width);
        assert_eq!(world.x, inner.x + inner.width + FLEX_SPACE.width);
    }

    fn from_default_after(keys: &[&str]) -> FlexState {
        keys.iter()
            .fold(FlexState::default(), |state, key| reduce(state, key).0)
    }

    fn empty_box_size() -> Size {
        measure(&Tree::root(vec![new_box()]), &[0])
    }

    #[test]
    fn one_a_centres_an_empty_inner_box_with_one_space_on_every_side() {
        let state = from_default_after(&["A"]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        let (outer, inner) = (boxes[0], boxes[1]);
        let empty = empty_box_size();
        assert_eq!(
            (outer.width, outer.height),
            (
                empty.width + 2 * FLEX_SPACE.width,
                empty.height + 2 * FLEX_SPACE.height
            )
        );
        assert_eq!(
            (inner.x, inner.y),
            (outer.x + FLEX_SPACE.width, outer.y + FLEX_SPACE.height)
        );
        assert_eq!(
            (inner.x + inner.width, inner.y + inner.height),
            (
                outer.x + outer.width - FLEX_SPACE.width,
                outer.y + outer.height - FLEX_SPACE.height
            )
        );
    }

    #[test]
    fn a_second_a_adds_an_inner_box_one_gap_to_the_right_and_stays_centred() {
        let state = from_default_after(&["A", "A"]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        let (outer, first, second) = (boxes[0], boxes[1], boxes[2]);
        let empty = empty_box_size();
        assert_eq!(
            (outer.width, outer.height),
            (
                2 * empty.width + FLEX_SPACE.width + 2 * FLEX_SPACE.width,
                empty.height + 2 * FLEX_SPACE.height
            )
        );
        assert_eq!(
            (second.x, second.y),
            (first.x + empty.width + FLEX_SPACE.width, first.y)
        );
        let left = outer.x - WINDOW.col;
        let right = WINDOW.col + WINDOW.cols - (outer.x + outer.width);
        let top = outer.y - WINDOW.row;
        let bottom = WINDOW.row + WINDOW.rows - (outer.y + outer.height);
        assert!((left - right).abs() <= 1);
        assert!((top - bottom).abs() <= 1);
    }

    const EVEN_SPREAD_WINDOW: Area = Area { cols: 82, ..WINDOW };
    const UNEVEN_SPREAD_WINDOW: Area = Area { cols: 81, ..WINDOW };

    fn spread(children: Vec<Tree<FlexNode>>) -> FlexState {
        FlexState {
            boxes: Tree::root(vec![Tree::new(
                FlexNode::Box(FlexBox {
                    width: FlexSize::Full,
                    justify: Justify::SpaceBetween,
                    ..FlexBox::default()
                }),
                children,
            )]),
            ..FlexState::default()
        }
    }

    fn row_of<'a>(placements: &'a [Placement<'a>], state: &FlexState) -> Vec<&'a Placement<'a>> {
        placements
            .iter()
            .zip(state.boxes.walk())
            .filter(|(_, (path, _))| path.len() == 2)
            .map(|(placement, _)| placement)
            .collect()
    }

    fn gaps_between(row: &[&Placement<'_>]) -> Vec<i64> {
        row.windows(2)
            .map(|pair| pair[1].x - (pair[0].x + pair[0].width))
            .collect()
    }

    fn free_space(window: Area, row: &[&Placement<'_>]) -> i64 {
        window.cols - 2 * FLEX_SPACE.width - row.iter().map(|sibling| sibling.width).sum::<i64>()
    }

    fn assert_spread_edge_to_edge(placements: &[Placement<'_>], row: &[&Placement<'_>]) {
        let outer = &placements[0];
        let (first, last) = (row[0], row[row.len() - 1]);
        assert_eq!(first.x, outer.x + FLEX_SPACE.width);
        assert_eq!(
            last.x + last.width,
            outer.x + outer.width - FLEX_SPACE.width
        );
    }

    #[test]
    fn a_full_space_between_box_spreads_mixed_siblings_edge_to_edge_with_equal_gaps() {
        let state = spread(vec![text("Hello"), new_box(), text("World")]);
        let [(_, placements)] = <[_; 1]>::try_from(scene(&state, EVEN_SPREAD_WINDOW)).unwrap();
        let row = row_of(&placements, &state);
        assert_spread_edge_to_edge(&placements, &row);
        let gap = free_space(EVEN_SPREAD_WINDOW, &row) / 2;
        assert_eq!(gaps_between(&row), vec![gap, gap]);
    }

    #[test]
    fn a_full_space_between_box_gives_the_extra_cells_to_the_last_gaps() {
        let state = spread(vec![text("Hello"), new_box(), text("World")]);
        let [(_, placements)] = <[_; 1]>::try_from(scene(&state, UNEVEN_SPREAD_WINDOW)).unwrap();
        let row = row_of(&placements, &state);
        assert_spread_edge_to_edge(&placements, &row);
        let base = free_space(UNEVEN_SPREAD_WINDOW, &row).div_euclid(2);
        assert_eq!(gaps_between(&row), vec![base, base + 1]);
    }

    #[test]
    fn a_full_space_between_box_widens_the_last_gaps_first() {
        let state = spread(vec![text("a"), new_box(), text("b"), new_box()]);
        for (window, widened) in [(UNEVEN_SPREAD_WINDOW, 1), (EVEN_SPREAD_WINDOW, 2)] {
            let [(_, placements)] = <[_; 1]>::try_from(scene(&state, window)).unwrap();
            let row = row_of(&placements, &state);
            assert_spread_edge_to_edge(&placements, &row);
            let base = free_space(window, &row).div_euclid(3);
            let mut expected = vec![base; 3 - widened];
            expected.extend(vec![base + 1; widened]);
            assert_eq!(gaps_between(&row), expected);
        }
    }

    #[test]
    fn full_and_g_spread_hello_the_inner_box_and_world_across_the_outer_box() {
        let keys = [
            "i", "H", "e", "l", "l", "o", "\r", "A", "s", "W", "o", "r", "l", "d", "\r", "w", "g",
        ];
        let state = keys
            .into_iter()
            .fold(FlexState::default(), |state, key| reduce(state, key).0);
        let [(_, placements)] = <[_; 1]>::try_from(scene(&state, EVEN_SPREAD_WINDOW)).unwrap();
        let outer = &placements[0];
        let hello = label_showing(&placements, "Hello");
        let inner = all_boxes(&placements)[1];
        let world = label_showing(&placements, "World");
        assert_eq!(outer.width, EVEN_SPREAD_WINDOW.cols);
        assert_eq!(hello.x, outer.x + FLEX_SPACE.width);
        assert_eq!(
            world.x + world.width,
            outer.x + outer.width - FLEX_SPACE.width
        );
        assert_eq!(
            inner.x - (hello.x + hello.width),
            world.x - (inner.x + inner.width)
        );
    }

    fn in_column(mut state: FlexState) -> FlexState {
        outer_box_mut(&mut state, 0).direction = Direction::Column;
        state
    }

    fn geometry(siblings: &[&Placement<'_>]) -> Vec<(i64, i64, i64, i64)> {
        siblings
            .iter()
            .map(|sibling| (sibling.x, sibling.y, sibling.width, sibling.height))
            .collect()
    }

    fn assert_centred_across(outer: &Placement<'_>, siblings: &[&Placement<'_>]) {
        let inner_left = outer.x + FLEX_SPACE.width;
        let inner_width = outer.width - 2 * FLEX_SPACE.width;
        for sibling in siblings {
            assert_eq!(sibling.x - inner_left, (inner_width - sibling.width) / 2);
        }
    }

    #[test]
    fn in_column_siblings_stack_top_to_bottom_in_order_one_gap_apart() {
        let state = in_column(hello_box_world());
        let placements = placements(&state);
        let outer = &placements[0];
        let hello = label_showing(&placements, "Hello");
        let inner = all_boxes(&placements)[1];
        let world = label_showing(&placements, "World");
        assert_eq!(hello.y, outer.y + FLEX_SPACE.height);
        assert_eq!(inner.y, hello.y + hello.height + FLEX_SPACE.height);
        assert_eq!(world.y, inner.y + inner.height + FLEX_SPACE.height);
        assert_eq!(
            world.y + world.height,
            outer.y + outer.height - FLEX_SPACE.height
        );
    }

    #[test]
    fn siblings_in_a_row_are_the_space_width_apart() {
        let state = hello_box_world();
        let placements = placements(&state);
        let row = row_of(&placements, &state);
        assert_eq!(gaps_between(&row), vec![FLEX_SPACE.width; 2]);
    }

    #[test]
    fn siblings_in_a_column_are_the_space_height_apart() {
        let state = in_column(hello_box_world());
        let placements = placements(&state);
        let column = row_of(&placements, &state);
        let gaps: Vec<i64> = column
            .windows(2)
            .map(|pair| pair[1].y - (pair[0].y + pair[0].height))
            .collect();
        assert_eq!(gaps, vec![FLEX_SPACE.height; 2]);
    }

    #[test]
    fn stacked_outer_boxes_are_the_space_height_apart() {
        let state = stacked(&["Hello", "", "Hi"]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        let gaps: Vec<i64> = boxes
            .windows(2)
            .map(|pair| pair[1].y - (pair[0].y + pair[0].height))
            .collect();
        assert_eq!(gaps, vec![FLEX_SPACE.height; 2]);
    }

    #[test]
    fn in_column_siblings_are_centred_horizontally_in_the_box() {
        let state = in_column(hello_box_world());
        let placements = placements(&state);
        let column = row_of(&placements, &state);
        assert_centred_across(&placements[0], &column);
    }

    #[test]
    fn a_space_between_column_places_its_siblings_as_a_start_column_does() {
        let spread_column = in_column(spread(vec![text("Hello"), new_box(), text("World")]));
        let mut start_column = spread_column.clone();
        outer_box_mut(&mut start_column, 0).justify = Justify::Start;
        let spread_placements = placements(&spread_column);
        let start_placements = placements(&start_column);
        assert_eq!(
            geometry(&row_of(&spread_placements, &spread_column)),
            geometry(&row_of(&start_placements, &start_column))
        );
    }

    #[test]
    fn a_full_column_centres_its_siblings_across_the_window() {
        let state = in_column(spread(vec![text("Hello"), new_box(), text("World")]));
        let placements = placements(&state);
        let outer = &placements[0];
        assert_eq!((outer.x, outer.width), (WINDOW.col, WINDOW.cols));
        assert_centred_across(outer, &row_of(&placements, &state));
    }

    fn pressed(keys: &[&str]) -> FlexState {
        keys.iter()
            .fold(FlexState::default(), |state, key| reduce(state, key).0)
    }

    fn outer_hello_and_inner_depths(placements: &[Placement<'_>]) -> (u8, u8, u8) {
        let [outer, inner] = <[_; 2]>::try_from(all_boxes(placements)).unwrap();
        (
            outer.depth,
            label_showing(placements, "Hello").depth,
            inner.depth,
        )
    }

    const HELLO_WITH_AN_INNER_BOX: [&str; 8] = ["i", "H", "e", "l", "l", "o", "\r", "A"];

    #[test]
    fn the_outer_box_is_depth_zero_and_its_texts_and_inner_box_are_depth_one() {
        let state = pressed(&HELLO_WITH_AN_INNER_BOX);
        let placements = placements(&state);
        assert_eq!(outer_hello_and_inner_depths(&placements), (0, 1, 1));
    }

    #[test]
    fn filling_the_outer_box_keeps_its_texts_and_inner_box_one_deeper() {
        let state = pressed(&[&HELLO_WITH_AN_INNER_BOX[..], &["f"]].concat());
        let placements = placements(&state);
        assert_eq!(solid_fill(&placements[0]), Some(FLEX_FILL_COLOUR));
        assert_eq!(outer_hello_and_inner_depths(&placements), (0, 1, 1));
    }

    #[test]
    fn an_inner_box_added_after_filling_is_one_deeper_than_the_outer_box() {
        let state = pressed(&["i", "H", "e", "l", "l", "o", "\r", "f", "A"]);
        let placements = placements(&state);
        assert_eq!(solid_fill(&placements[0]), Some(FLEX_FILL_COLOUR));
        assert_eq!(outer_hello_and_inner_depths(&placements), (0, 1, 1));
    }

    fn text_colours(placements: &[Placement<'_>], shown: &[&str]) -> Vec<Rgb> {
        shown
            .iter()
            .map(|shown| {
                let PlacementNode::Label(Label { colour, .. }) =
                    label_showing(placements, shown).node
                else {
                    unreachable!()
                };
                colour
            })
            .collect()
    }

    fn world_selected(mode: FlexMode) -> FlexState {
        FlexState {
            mode,
            selected: vec![0, 2],
            ..hello_box_world()
        }
    }

    #[test]
    fn in_move_the_selected_text_has_the_selected_colour() {
        let state = world_selected(FlexMode::Move);
        assert_eq!(
            text_colours(&placements(&state), &["Hello", "World"]),
            vec![FLEX_TEXT_COLOUR, FLEX_SELECTED_COLOUR]
        );
    }

    #[test]
    fn in_write_the_selected_text_keeps_the_text_colour() {
        let state = world_selected(FlexMode::Write);
        assert_eq!(
            text_colours(&placements(&state), &["Hello", "World"]),
            vec![FLEX_TEXT_COLOUR; 2]
        );
    }

    #[test]
    fn in_move_the_box_around_a_selected_text_keeps_the_border_colour() {
        let state = world_selected(FlexMode::Move);
        assert_eq!(borders(&placements(&state)), vec![FLEX_BORDER_COLOUR; 2]);
    }

    #[test]
    fn in_move_a_selected_inner_box_beside_texts_has_the_selected_border() {
        let state = FlexState {
            mode: FlexMode::Move,
            selected: vec![0, 1],
            ..hello_box_world()
        };
        let placements = placements(&state);
        assert_eq!(
            borders(&placements),
            vec![FLEX_BORDER_COLOUR, FLEX_SELECTED_COLOUR]
        );
        assert_eq!(
            text_colours(&placements, &["Hello", "World"]),
            vec![FLEX_TEXT_COLOUR; 2]
        );
    }
}
