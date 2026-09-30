use std::borrow::Cow;
use std::collections::HashMap;

use types::Tree;

use crate::view::{self, Area, Label, Placement, PlacementNode, Rgb, Scene, ALL_SIDES};

use super::state::{FlexBox, FlexMode, FlexNode, FlexState, FlexWidth, Justify};

const FLEX_BORDER: i64 = 1;
const FLEX_GAP: i64 = 1;
#[cfg_attr(not(test), allow(dead_code))]
const TEXT_GAP: i64 = 1;
const FLEX_BORDER_COLOUR: Rgb = (0x2A, 0x2A, 0x2E);
const FLEX_SELECTED_COLOUR: Rgb = (0x8A, 0xB4, 0xF8);
const FLEX_TEXT_COLOUR: Rgb = (0xC9, 0xC9, 0xCF);
const FLEX_FILL_COLOUR: Rgb = (0x14, 0x14, 0x16);

#[cfg_attr(not(test), allow(dead_code))]
fn text_offsets(texts: &[&str], justify: Justify, inner_width: i64) -> Vec<i64> {
    let gaps = texts.len() as i64 - 1;
    let free = inner_width - texts.iter().map(|text| view::interior(text)).sum::<i64>();
    let gap_after = |index: i64| match justify {
        Justify::Start => TEXT_GAP,
        Justify::SpaceBetween => {
            let widened_from = gaps - free.rem_euclid(gaps);
            free.div_euclid(gaps) + i64::from(index >= widened_from)
        }
    };
    let mut next = 0;
    (0..)
        .zip(texts)
        .map(|(index, text)| {
            let offset = next;
            if index < gaps {
                next += view::interior(text) + gap_after(index);
            }
            offset
        })
        .collect()
}

#[cfg_attr(not(test), allow(dead_code))]
fn inner_box_width() -> i64 {
    view::interior("") + 2
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Size {
    width: i64,
    height: i64,
}

fn child_sizes(
    tree: &Tree<FlexNode>,
    sizes: &HashMap<Vec<usize>, Size>,
    path: &[usize],
) -> Vec<Size> {
    (0..)
        .map(|index| [path, &[index]].concat())
        .take_while(|child| tree.contains(child))
        .map(|child| sizes[&child])
        .collect()
}

fn measure(tree: &Tree<FlexNode>) -> HashMap<Vec<usize>, Size> {
    let mut sizes = HashMap::new();
    let nodes: Vec<_> = tree.walk().collect();
    for (path, node) in nodes.into_iter().rev() {
        let size = match node {
            FlexNode::Text(text) => Size {
                width: view::interior(text),
                height: 1,
            },
            FlexNode::Box(_) => {
                let children = child_sizes(tree, &sizes, &path);
                let gaps = FLEX_GAP * children.len().saturating_sub(1) as i64;
                let content_width = children.iter().map(|child| child.width).sum::<i64>() + gaps;
                let content_height = children.iter().map(|child| child.height).max().unwrap_or(0);
                Size {
                    width: content_width + 2 * FLEX_BORDER,
                    height: content_height + 2 * FLEX_BORDER,
                }
            }
        };
        sizes.insert(path, size);
    }
    sizes
}

struct Frame {
    y: i64,
    inner_height: i64,
    cursor: i64,
    gap: i64,
    widened_from: i64,
}

impl Frame {
    fn new(
        width: FlexWidth,
        justify: Justify,
        x: i64,
        y: i64,
        size: Size,
        children: &[Size],
    ) -> Self {
        let inner_width = size.width - 2 * FLEX_BORDER;
        let gaps = children.len().saturating_sub(1) as i64;
        let children_width: i64 = children.iter().map(|child| child.width).sum();
        let (gap, widened_from) = match justify {
            Justify::SpaceBetween if gaps > 0 => {
                let free = inner_width - children_width;
                (free.div_euclid(gaps), gaps - free.rem_euclid(gaps))
            }
            _ => (FLEX_GAP, gaps),
        };
        let row_width = children_width + gap * gaps + (gaps - widened_from);
        let row_shift = match width {
            FlexWidth::Fit => (inner_width - row_width).div_euclid(2),
            FlexWidth::Full => 0,
        };
        Frame {
            y,
            inner_height: size.height - 2 * FLEX_BORDER,
            cursor: x + FLEX_BORDER + row_shift,
            gap,
            widened_from,
        }
    }

    fn gap_after(&self, index: usize) -> i64 {
        self.gap + i64::from(index as i64 >= self.widened_from)
    }
}

fn place<'a>(state: &'a FlexState, window: Area) -> Vec<Placement<'a>> {
    let sizes = measure(&state.boxes);
    let mut frames: HashMap<Vec<usize>, Frame> = HashMap::new();
    let mut placements = Vec::with_capacity(sizes.len());
    let mut next_outer_y = 0;
    for (path, node) in state.boxes.walk() {
        let measured = sizes[&path];
        let (x, y, size) = match path.split_last() {
            Some((_, [])) => {
                let width = match node {
                    FlexNode::Box(FlexBox {
                        width: FlexWidth::Full,
                        ..
                    }) => window.cols,
                    _ => measured.width,
                };
                let y = next_outer_y;
                next_outer_y += measured.height + FLEX_GAP;
                (-width.div_euclid(2), y, Size { width, ..measured })
            }
            Some((&index, parent)) => {
                let frame = frames
                    .get_mut(parent)
                    .expect("a parent is placed before its children");
                let x = frame.cursor;
                frame.cursor += measured.width + frame.gap_after(index);
                let y = frame.y + FLEX_BORDER + (frame.inner_height - measured.height) / 2;
                (x, y, measured)
            }
            None => unreachable!("walk never yields the root"),
        };
        match node {
            FlexNode::Text(text) => placements.push(Placement {
                node: PlacementNode::Label(Label {
                    text: Cow::Borrowed(text),
                    colour: FLEX_TEXT_COLOUR,
                    bold: false,
                }),
                x,
                y,
                width: size.width,
                height: size.height,
            }),
            FlexNode::Box(flex_box) => {
                let outer = path.len() == 1;
                let colour = if outer && state.mode == FlexMode::Move && state.selected == path {
                    FLEX_SELECTED_COLOUR
                } else {
                    FLEX_BORDER_COLOUR
                };
                placements.push(Placement {
                    node: PlacementNode::Box {
                        colour,
                        fill: None,
                        opacity: None,
                        solid_fill: (outer && flex_box.filled).then_some(FLEX_FILL_COLOUR),
                        rounded: false,
                        sides: ALL_SIDES,
                        border: FLEX_BORDER,
                    },
                    x,
                    y,
                    width: size.width,
                    height: size.height,
                });
                let width = if outer {
                    flex_box.width
                } else {
                    FlexWidth::Fit
                };
                let children = child_sizes(&state.boxes, &sizes, &path);
                let frame = Frame::new(width, flex_box.justify, x, y, size, &children);
                frames.insert(path, frame);
            }
        }
    }
    placements
}

pub(crate) fn scene(state: &FlexState, window: Area) -> Scene<'_> {
    let mut placements = place(state, window);
    // view::centre assumes content starts at x = 0.
    let left = placements
        .iter()
        .map(|placement| placement.x)
        .min()
        .unwrap_or(0);
    for placement in &mut placements {
        placement.x -= left;
    }
    vec![(window, view::centre(placements, window))]
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
    fn an_empty_box_is_three_cells_wide() {
        let state = with_text("");
        let placements = placements(&state);
        let the_box = the_box(&placements);
        assert_eq!(the_box.width, 3);
        assert_eq!(the_box.height, BOX_HEIGHT);
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
        assert_eq!(the_box(&placements).width, 7);
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
        let before = with_text("Hellp");
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
        let state = with_text("");
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
        let grown = with_text("Hello");
        let (shrunk, _) = reduce(with_text("Hellp"), "\x7f");
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
        assert_eq!(the_box.width, 7);
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
                width: FlexWidth::Full,
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
    fn a_full_box_starts_its_label_just_inside_the_border() {
        let state = full("Hello");
        let placements = placements(&state);
        assert_eq!(the_label(&placements).x, the_box(&placements).x + 1);
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
        assert_eq!(boxes[1].y, boxes[0].y + BOX_HEIGHT + FLEX_GAP);
    }

    #[test]
    fn stacked_boxes_line_up_by_their_centres() {
        let state = stacked(&["Hello", ""]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        assert_eq!((boxes[0].width, boxes[1].width), (7, 3));
        assert_eq!(
            2 * boxes[0].x + boxes[0].width,
            2 * boxes[1].x + boxes[1].width
        );
    }

    #[test]
    fn odd_widths_lean_right() {
        let state = stacked(&["Hi", ""]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        assert_eq!((boxes[0].width, boxes[1].width), (4, 3));
        assert_eq!(boxes[1].x, boxes[0].x + 1);
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
    fn each_text_starts_after_the_previous_one_and_a_gap() {
        let texts = ["Hello", "World", ""];
        let first = view::interior("Hello") + TEXT_GAP;
        let second = first + view::interior("World") + TEXT_GAP;
        assert_eq!(
            text_offsets(&texts, Justify::Start, 0),
            vec![0, first, second]
        );
    }

    #[test]
    fn a_fit_box_is_as_wide_as_its_texts_and_gaps() {
        for texts in [&["Hello", ""][..], &["Hello", "World"][..]] {
            let state = beside(texts);
            let placements = placements(&state);
            let interiors: i64 = texts.iter().map(|text| view::interior(text)).sum();
            let gaps = TEXT_GAP * (texts.len() as i64 - 1);
            assert_eq!(the_box(&placements).width, interiors + gaps + 2);
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
            labels[0].x + view::interior("Hello") + TEXT_GAP
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
                width: FlexWidth::Full,
                ..FlexBox::default()
            },
            &["Hello", "World"],
        );
        let placements = placements(&state);
        let the_box = the_box(&placements);
        assert_eq!(the_box.width, WINDOW.cols);
        assert_eq!(all_labels(&placements)[0].x, the_box.x + 1);
    }

    #[test]
    fn start_ignores_the_inner_width() {
        let texts = ["Hello", "World"];
        assert_eq!(text_offsets(&texts, Justify::Start, 78), vec![0, 6]);
    }

    #[test]
    fn space_between_puts_the_last_text_at_the_inner_right_edge() {
        let texts = ["Hello", "World"];
        assert_eq!(text_offsets(&texts, Justify::SpaceBetween, 78), vec![0, 73]);
    }

    #[test]
    fn space_between_splits_the_free_space_evenly() {
        let texts = ["a", "b", "c"];
        assert_eq!(
            text_offsets(&texts, Justify::SpaceBetween, 9),
            vec![0, 4, 8]
        );
    }

    #[test]
    fn space_between_gives_the_extra_cells_to_the_right_gaps() {
        assert_eq!(
            text_offsets(&["a", "b", "c"], Justify::SpaceBetween, 10),
            vec![0, 4, 9]
        );
        assert_eq!(
            text_offsets(&["a", "b", "c", "d"], Justify::SpaceBetween, 12),
            vec![0, 3, 7, 11]
        );
    }

    #[test]
    fn space_between_with_one_text_sits_at_the_start() {
        assert_eq!(text_offsets(&["Hello"], Justify::SpaceBetween, 78), vec![0]);
    }

    #[test]
    fn space_between_in_the_packed_width_matches_start() {
        let texts = ["Hello", "World", ""];
        let interiors: i64 = texts.iter().map(|text| view::interior(text)).sum();
        let packed = interiors + TEXT_GAP * (texts.len() as i64 - 1);
        assert_eq!(
            text_offsets(&texts, Justify::SpaceBetween, packed),
            text_offsets(&texts, Justify::Start, packed)
        );
    }

    fn full_beside(texts: &[&str], justify: Justify) -> FlexState {
        holding(
            FlexBox {
                width: FlexWidth::Full,
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
        assert_eq!(labels[0].x, the_box.x + 1);
        assert_eq!(last.x + last.width, the_box.x + the_box.width - 1);
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
        assert_eq!(labels[0].x, the_box.x + 1);
        assert_eq!(labels[1].x, labels[0].x + labels[0].width + TEXT_GAP);
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
        let state = with_inner_boxes("Hello", 1);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        assert_eq!(boxes[1].width, inner_box_width());
        assert_eq!(boxes[1].height, BOX_HEIGHT);
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
    fn the_last_inner_box_ends_on_the_row_above_the_outer_bottom_border() {
        let state = with_inner_boxes("Hello", 2);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        assert_eq!(
            boxes[2].y + boxes[2].height + 1,
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
            outer.width = FlexWidth::Full;
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
        assert_eq!(second.y, first.y + first.height + FLEX_GAP);
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

    fn sizes_of(children: Vec<Tree<FlexNode>>) -> HashMap<Vec<usize>, Size> {
        measure(&Tree::root(vec![Tree::new(
            FlexNode::Box(FlexBox::default()),
            children,
        )]))
    }

    fn text(text: &str) -> Tree<FlexNode> {
        Tree::leaf(FlexNode::Text(text.to_string()))
    }

    #[test]
    fn a_text_measures_its_interior_by_one_row() {
        let sizes = sizes_of(vec![text("Hello")]);
        assert_eq!(
            sizes[&vec![0, 0]],
            Size {
                width: view::interior("Hello"),
                height: 1,
            }
        );
    }

    #[test]
    fn a_new_box_measures_around_its_empty_text() {
        let sizes = measure(&Tree::root(vec![new_box()]));
        assert_eq!(
            sizes[&vec![0]],
            Size {
                width: view::interior("") + 2 * FLEX_BORDER,
                height: 1 + 2 * FLEX_BORDER,
            }
        );
    }

    #[test]
    fn a_row_measures_its_children_side_by_side_with_gaps() {
        let sizes = sizes_of(vec![text("Hello"), new_box(), text("World")]);
        assert_eq!(
            sizes[&vec![0]],
            Size {
                width: view::interior("Hello")
                    + FLEX_GAP
                    + inner_box_width()
                    + FLEX_GAP
                    + view::interior("World")
                    + 2 * FLEX_BORDER,
                height: BOX_HEIGHT + 2 * FLEX_BORDER,
            }
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
        assert_eq!(inner.x, hello.x + hello.width + FLEX_GAP);
        assert_eq!(world.x, inner.x + inner.width + FLEX_GAP);
    }

    #[test]
    fn texts_are_centred_vertically_on_the_tallest_sibling() {
        let state = hello_box_world();
        let placements = placements(&state);
        let inner = all_boxes(&placements)[1];
        let middle = inner.y + inner.height / 2;
        assert_eq!(label_showing(&placements, "Hello").y, middle);
        assert_eq!(label_showing(&placements, "World").y, middle);
    }

    #[test]
    fn the_outer_box_grows_to_fit_the_row_and_stays_centred() {
        let state = hello_box_world();
        let placements = placements(&state);
        let outer = all_boxes(&placements)[0];
        let measured = measure(&state.boxes)[&vec![0]];
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
        assert_eq!(inner.x, hello.x + hello.width + FLEX_GAP);
        assert_eq!(world.x, inner.x + inner.width + FLEX_GAP);
    }

    const EVEN_SPREAD_WINDOW: Area = Area { cols: 81, ..WINDOW };

    fn spread(children: Vec<Tree<FlexNode>>) -> FlexState {
        FlexState {
            boxes: Tree::root(vec![Tree::new(
                FlexNode::Box(FlexBox {
                    width: FlexWidth::Full,
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
        window.cols - 2 * FLEX_BORDER - row.iter().map(|sibling| sibling.width).sum::<i64>()
    }

    fn assert_spread_edge_to_edge(placements: &[Placement<'_>], row: &[&Placement<'_>]) {
        let outer = &placements[0];
        let (first, last) = (row[0], row[row.len() - 1]);
        assert_eq!(first.x, outer.x + FLEX_BORDER);
        assert_eq!(last.x + last.width, outer.x + outer.width - FLEX_BORDER);
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
        let placements = placements(&state);
        let row = row_of(&placements, &state);
        assert_spread_edge_to_edge(&placements, &row);
        let base = free_space(WINDOW, &row).div_euclid(2);
        assert_eq!(gaps_between(&row), vec![base, base + 1]);
    }

    #[test]
    fn a_full_space_between_box_widens_the_last_gaps_first() {
        let state = spread(vec![text("a"), new_box(), text("b"), new_box()]);
        for (window, widened) in [(WINDOW, 1), (EVEN_SPREAD_WINDOW, 2)] {
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
            "H", "e", "l", "l", "o", "\r", "A", "s", "W", "o", "r", "l", "d", "\r", "w", "g",
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
        assert_eq!(hello.x, outer.x + FLEX_BORDER);
        assert_eq!(world.x + world.width, outer.x + outer.width - FLEX_BORDER);
        assert_eq!(
            inner.x - (hello.x + hello.width),
            world.x - (inner.x + inner.width)
        );
    }
}
