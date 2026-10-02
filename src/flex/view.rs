use std::borrow::Cow;
use std::collections::HashSet;

use crate::style::FOREGROUND;
use crate::view::{Label, Placement, PlacementNode, Rgb, Scene, ALL_SIDES, NO_SIDES};

use super::layout::{Arranged, Layout, Rect, FLEX_BORDER};
use super::state::{FlexBox, FlexMode, FlexState};

const FLEX_BORDER_COLOUR: Rgb = (0x2A, 0x2A, 0x2E);
const FLEX_SELECTED_COLOUR: Rgb = (0x8A, 0xB4, 0xF8);
const FLEX_TEXT_COLOUR: Rgb = (0xC9, 0xC9, 0xCF);
const FLEX_FILL_COLOUR: Rgb = (0x14, 0x14, 0x16);
const FLEX_REPLACE_OPACITY: f64 = 0.55;

fn paint<'a>(
    state: &FlexState,
    new: &HashSet<Vec<usize>>,
    flex_box: &'a FlexBox,
    arranged: &Arranged,
) -> Vec<Placement<'a>> {
    let path = &arranged.path;
    let selected = state.mode == FlexMode::Move && &state.selected == path;
    let depth = path.len().saturating_sub(1) as u8;
    let placement = |node, rect: Rect| Placement {
        node,
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        depth,
    };
    let border = flex_box.border.then(|| {
        let outer = path.len() == 1;
        placement(
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
                grow: new.contains(path),
            },
            arranged.rect,
        )
    });
    let highlight = if state.mode == FlexMode::Replace && &state.selected == path {
        flex_box
            .text
            .as_ref()
            .zip(arranged.text)
            .map(|(text, rect)| {
                placement(
                    PlacementNode::Box {
                        colour: FLEX_SELECTED_COLOUR,
                        fill: Some(FOREGROUND),
                        opacity: Some(FLEX_REPLACE_OPACITY),
                        solid_fill: None,
                        rounded: false,
                        sides: NO_SIDES,
                        border: FLEX_BORDER,
                        grow: false,
                    },
                    Rect {
                        x: rect.x,
                        y: rect.y,
                        width: text.chars().count() as i64,
                        height: 1,
                    },
                )
            })
    } else {
        None
    };
    let label = flex_box
        .text
        .as_ref()
        .zip(arranged.text)
        .map(|(text, rect)| {
            placement(
                PlacementNode::Label(Label {
                    text: Cow::Borrowed(text),
                    colour: if selected && !flex_box.border {
                        FLEX_SELECTED_COLOUR
                    } else {
                        FLEX_TEXT_COLOUR
                    },
                    bold: false,
                }),
                rect,
            )
        });
    let caret = (state.mode == FlexMode::Write && &state.selected == path)
        .then(|| {
            flex_box
                .text
                .as_ref()
                .zip(arranged.text)
                .map(|(text, rect)| {
                    placement(
                        PlacementNode::TypingCaret {
                            colour: FLEX_TEXT_COLOUR,
                            bold: false,
                        },
                        Rect {
                            x: rect.x + text.chars().count() as i64,
                            y: rect.y,
                            width: 1,
                            height: 1,
                        },
                    )
                })
        })
        .flatten();
    border
        .into_iter()
        .chain(highlight)
        .chain(label)
        .chain(caret)
        .collect()
}

pub(crate) fn scene<'a>(
    state: &'a FlexState,
    layout: &Layout,
    new: &HashSet<Vec<usize>>,
) -> Scene<'a> {
    let placements = layout
        .arranged()
        .iter()
        .flat_map(|arranged| paint(state, new, state.boxes.value(&arranged.path), arranged))
        .collect();
    vec![(layout.area(), placements)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flex::layout::*;
    use crate::flex::state::{new_box, new_window, Direction, FlexBox, FlexEffect, Justify};
    use crate::view::{self, Area, BOX_HEIGHT};
    use types::Tree;

    const WINDOW: Area = Area {
        col: 0,
        row: 0,
        cols: 80,
        rows: 24,
    };

    fn reduce(state: FlexState, key: &str) -> (FlexState, Option<FlexEffect>) {
        let layout = Layout::arrange(&state.boxes, WINDOW);
        crate::flex::state::reduce(state, key, &layout)
    }

    fn row() -> FlexBox {
        FlexBox {
            direction: Direction::Row,
            ..FlexBox::default()
        }
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
            boxes: new_window(vec![outer(the_box, texts, 0)]),
            ..FlexState::default()
        }
    }

    fn with_text(text: &str) -> FlexState {
        holding(FlexBox::default(), &[text])
    }

    fn with_text_row(text: &str) -> FlexState {
        holding(row(), &[text])
    }

    fn outer_box_mut(state: &mut FlexState, index: usize) -> &mut FlexBox {
        state.boxes.value_mut(&[index])
    }

    fn placements(state: &FlexState) -> Vec<Placement<'_>> {
        placements_with_new(state, &HashSet::new())
    }

    fn placements_with_new<'a>(
        state: &'a FlexState,
        new: &HashSet<Vec<usize>>,
    ) -> Vec<Placement<'a>> {
        let layout = Layout::arrange(&state.boxes, WINDOW);
        let [(area, placements)] = <[_; 1]>::try_from(scene(state, &layout, new)).unwrap();
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
    fn an_empty_box_spans_the_window_and_is_as_high_as_its_padding() {
        let state = FlexState::default();
        let placements = placements(&state);
        let the_box = the_box(&placements);
        assert_eq!(the_box.width, WINDOW.cols);
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
    fn the_box_measures_to_fit_the_text() {
        let state = with_text("Hello");
        assert_eq!(
            measure(&state.boxes, &[0]).width,
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
        assert_eq!(
            Some(text.as_ref()),
            state.boxes.value(&[0, 0]).text.as_deref()
        );
        assert!(label.x > the_box.x);
        assert!(label.x + label.width < the_box.x + the_box.width);
    }

    #[test]
    fn the_box_measures_narrower_after_backspace() {
        let before = FlexState {
            selected: vec![0, 0],
            mode: FlexMode::Write,
            ..with_text("Hellp")
        };
        let (after, _) = reduce(before.clone(), "\x7f");
        let before_width = measure(&before.boxes, &[0]).width;
        let after_width = measure(&after.boxes, &[0]).width;
        assert!(after_width < before_width);
        assert_eq!(after_width, measure(&with_text("Hell").boxes, &[0]).width);
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
        let (state, _) = reduce(FlexState::default(), "o");
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
        assert_eq!(
            measure(&state.boxes, &[0]).width,
            view::interior("Hello") + 2 * FLEX_SPACE.width
        );
        assert_eq!(sides_and_border(the_box(&placements)).1, FLEX_BORDER);
    }

    #[test]
    fn the_border_stays_thin_after_backspace() {
        let (after, _) = reduce(with_text("Hellp"), "\x7f");
        let placements = placements(&after);
        assert_eq!(sides_and_border(the_box(&placements)).1, FLEX_BORDER);
    }

    #[test]
    fn an_outer_box_with_text_spans_the_window() {
        let state = with_text("Hello");
        let placements = placements(&state);
        let the_box = the_box(&placements);
        assert_eq!(the_box.x, 0);
        assert_eq!(the_box.width, WINDOW.cols);
    }

    #[test]
    fn an_outer_box_starts_its_label_one_space_in_from_its_edge() {
        let state = with_text("Hello");
        let placements = placements(&state);
        assert_eq!(
            the_label(&placements).x,
            the_box(&placements).x + FLEX_SPACE.width
        );
    }

    #[test]
    fn an_outer_box_follows_the_window() {
        let narrow = Area { cols: 40, ..WINDOW };
        let state = with_text("Hello");
        let layout = Layout::arrange(&state.boxes, narrow);
        let [(_, placements)] =
            <[_; 1]>::try_from(scene(&state, &layout, &HashSet::new())).unwrap();
        assert_eq!(the_box(&placements).width, narrow.cols);
    }

    fn grows(placement: &Placement) -> bool {
        let PlacementNode::Box { grow, .. } = placement.node else {
            panic!("not a box")
        };
        grow
    }

    #[test]
    fn only_a_new_box_grows() {
        let state = FlexState {
            boxes: new_window(vec![outer(FlexBox::default(), &[], 2)]),
            ..FlexState::default()
        };
        let new = HashSet::from([vec![0, 1]]);
        let grown: Vec<bool> = placements_with_new(&state, &new)
            .iter()
            .map(grows)
            .collect();
        assert_eq!(grown, [false, false, true]);
    }

    fn stacked(texts: &[&str]) -> FlexState {
        FlexState {
            boxes: new_window(
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
        assert_eq!((boxes[0].width, boxes[1].width), (WINDOW.cols, WINDOW.cols));
        assert_eq!(
            2 * boxes[0].x + boxes[0].width,
            2 * boxes[1].x + boxes[1].width
        );
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

    fn beside_row(texts: &[&str]) -> FlexState {
        holding(row(), texts)
    }

    fn margins(placements: &[Placement<'_>]) -> (i64, i64) {
        let the_box = the_box(placements);
        (
            the_box.x - WINDOW.col,
            WINDOW.col + WINDOW.cols - (the_box.x + the_box.width),
        )
    }

    #[test]
    fn a_box_measures_as_wide_as_its_texts_and_gaps() {
        for texts in [&["Hello", ""][..], &["Hello", "World"][..]] {
            let state = beside_row(texts);
            let interiors: i64 = texts.iter().map(|text| view::interior(text)).sum();
            let gaps = FLEX_SPACE.width * (texts.len() as i64 - 1);
            assert_eq!(
                measure(&state.boxes, &[0]).width,
                interiors + gaps + 2 * FLEX_SPACE.width
            );
        }
    }

    #[test]
    fn a_box_with_two_texts_places_one_box_and_two_labels() {
        let state = beside_row(&["Hello", "World"]);
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
        let state = beside_row(&["Hello", "World"]);
        let placements = placements(&state);
        let labels = all_labels(&placements);
        assert_eq!(
            labels[1].x,
            labels[0].x + labels[0].width + FLEX_SPACE.width
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
        let (grown, _) = reduce(with_text("Hello"), "o");
        let (left, right) = margins(&placements(&grown));
        assert!((left - right).abs() <= 1);
    }

    #[test]
    fn an_outer_box_with_two_texts_spans_the_window() {
        let state = beside(&["Hello", "World"]);
        let placements = placements(&state);
        let the_box = the_box(&placements);
        assert_eq!(the_box.width, WINDOW.cols);
        assert_eq!(all_labels(&placements)[0].x, the_box.x + FLEX_SPACE.width);
    }

    fn justified_beside(texts: &[&str], justify: Justify) -> FlexState {
        holding(
            FlexBox {
                justify,
                ..FlexBox::default()
            },
            texts,
        )
    }

    fn justified_beside_row(texts: &[&str], justify: Justify) -> FlexState {
        holding(FlexBox { justify, ..row() }, texts)
    }

    #[test]
    fn a_space_between_outer_box_spreads_its_texts_to_the_borders() {
        let state = justified_beside(&["Hello", "World"], Justify::SpaceBetween);
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
    fn a_space_between_outer_box_with_one_text_starts_it_one_space_in_from_its_edge() {
        let state = justified_beside(&["Hello"], Justify::SpaceBetween);
        let placements = placements(&state);
        assert_eq!(
            the_label(&placements).x,
            the_box(&placements).x + FLEX_SPACE.width
        );
    }

    #[test]
    fn a_space_between_row_spreads_two_boxes_to_its_inner_edges() {
        let state = FlexState {
            boxes: new_window(vec![outer(
                FlexBox {
                    justify: Justify::SpaceBetween,
                    ..row()
                },
                &[],
                2,
            )]),
            ..FlexState::default()
        };
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        let (row, first, last) = (boxes[0], boxes[1], boxes[2]);
        assert_eq!(first.x, row.x + FLEX_SPACE.width);
        assert_eq!(last.x + last.width, row.x + row.width - FLEX_SPACE.width);
        assert_eq!(first.width, measure(&state.boxes, &[0, 0]).width);
        assert_eq!(last.width, measure(&state.boxes, &[0, 1]).width);
    }

    fn row_of_boxes(row: FlexBox, own_text: &[&str], box_texts: &[&str]) -> FlexState {
        let row = FlexBox {
            direction: Direction::Row,
            ..row
        };
        let boxes: Vec<_> = box_texts
            .iter()
            .map(|label| outer(FlexBox::default(), &[label], 0))
            .collect();
        FlexState {
            boxes: new_window(vec![Tree::new(
                row,
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

    fn free_space_of_row(state: &FlexState, box_count: usize) -> i64 {
        let placements = placements(state);
        let row = all_boxes(&placements)[0];
        let widths: i64 = (0..box_count)
            .map(|index| measure(&state.boxes, &[0, index]).width)
            .sum();
        row.width - 2 * FLEX_SPACE.width - widths
    }

    #[test]
    fn a_space_between_row_with_one_box_keeps_it_its_width_at_the_left_edge() {
        let state = row_of_boxes(spreading(), &[], &["Hello"]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        assert_eq!(boxes[1].x, boxes[0].x + FLEX_SPACE.width);
        assert_eq!(boxes[1].width, measure(&state.boxes, &[0, 0]).width);
    }

    #[test]
    fn a_space_between_row_with_three_boxes_has_equal_gaps_when_the_free_space_divides_evenly() {
        let state = row_of_boxes(spreading(), &[], &["Hello", "Hello", "Hi"]);
        assert_eq!(free_space_of_row(&state, 3) % 2, 0);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        let (row, items) = (boxes[0], &boxes[1..]);
        assert_eq!(items[0].x, row.x + FLEX_SPACE.width);
        assert_eq!(
            items[2].x + items[2].width,
            row.x + row.width - FLEX_SPACE.width
        );
        let gaps = gaps_between(items);
        assert_eq!(gaps[0], gaps[1]);
    }

    #[test]
    fn a_space_between_row_with_an_odd_remainder_keeps_both_edges_and_gaps_within_one() {
        let state = row_of_boxes(spreading(), &[], &["Hello", "Hello", "Hello"]);
        assert_eq!(free_space_of_row(&state, 3) % 2, 1);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        let (row, items) = (boxes[0], &boxes[1..]);
        assert_eq!(items[0].x, row.x + FLEX_SPACE.width);
        assert_eq!(
            items[2].x + items[2].width,
            row.x + row.width - FLEX_SPACE.width
        );
        let gaps = gaps_between(items);
        assert!((gaps[0] - gaps[1]).abs() <= 1);
    }

    #[test]
    fn a_space_between_row_spreads_its_own_text_with_its_boxes() {
        let state = row_of_boxes(spreading(), &["Title"], &["Hello", "World"]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        let (row, items) = (boxes[0], &boxes[1..]);
        let label = the_label(&placements);
        assert_eq!(label.x, row.x + FLEX_SPACE.width);
        assert_eq!(
            items[1].x + items[1].width,
            row.x + row.width - FLEX_SPACE.width
        );
        let gaps = [
            items[0].x - (label.x + label.width),
            items[1].x - (items[0].x + items[0].width),
        ];
        assert!((gaps[0] - gaps[1]).abs() <= 1);
    }

    #[test]
    fn a_start_row_gives_each_of_two_boxes_half_of_its_width() {
        let state = row_of_boxes(FlexBox::default(), &[], &["Hello", "Hi"]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        let inner_width = boxes[0].width - 2 * FLEX_SPACE.width;
        assert_eq!(
            boxes[1].width + FLEX_SPACE.width + boxes[2].width,
            inner_width
        );
        assert!((boxes[1].width - boxes[2].width).abs() <= 1);
        assert_eq!(boxes[2].x, boxes[1].x + boxes[1].width + FLEX_SPACE.width);
    }

    #[test]
    fn pressing_s_twice_on_a_row_gives_the_boxes_their_equal_shares_again() {
        let state = FlexState {
            mode: FlexMode::Move,
            selected: vec![0],
            ..row_of_boxes(FlexBox::default(), &[], &["Hello", "Hi"])
        };
        let before = placements(&state)
            .iter()
            .map(|placement| (placement.x, placement.width))
            .collect::<Vec<_>>();
        let (spread, _) = reduce(state, "s");
        let (back, _) = reduce(spread, "s");
        let after = placements(&back)
            .iter()
            .map(|placement| (placement.x, placement.width))
            .collect::<Vec<_>>();
        assert_eq!(after, before);
    }

    #[test]
    fn a_start_outer_box_keeps_its_texts_one_gap_apart() {
        let state = justified_beside_row(&["Hello", "World"], Justify::Start);
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
            boxes: new_window(vec![outer(FlexBox::default(), &[text], count)]),
            ..FlexState::default()
        }
    }

    fn with_inner_boxes_row(text: &str, count: usize) -> FlexState {
        FlexState {
            boxes: new_window(vec![outer(row(), &[text], count)]),
            ..FlexState::default()
        }
    }

    fn outer_boxes_with_inner_counts(counts: &[usize]) -> FlexState {
        FlexState {
            boxes: new_window(
                counts
                    .iter()
                    .map(|&count| outer(FlexBox::default(), &["Hello"], count))
                    .collect(),
            ),
            ..FlexState::default()
        }
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
            ..
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

    fn box_of(children: Vec<Tree<FlexBox>>) -> Tree<FlexBox> {
        new_window(vec![Tree::new(FlexBox::default(), children)])
    }

    fn row_box_of(children: Vec<Tree<FlexBox>>) -> Tree<FlexBox> {
        new_window(vec![Tree::new(row(), children)])
    }

    fn text(text: &str) -> Tree<FlexBox> {
        Tree::leaf(FlexBox {
            border: false,
            text: Some(text.to_string()),
            ..FlexBox::default()
        })
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
    fn a_borderless_text_ignores_its_padding() {
        let padded = Tree::leaf(FlexBox {
            padding: 3,
            ..text("Hi").value(&[]).clone()
        });
        assert_eq!(measure(&padded, &[]), measure(&text("Hi"), &[]));
    }

    #[test]
    fn a_new_box_measures_its_padding_on_both_sides() {
        assert_eq!(
            measure(&new_window(vec![new_box()]), &[0]),
            Size {
                width: 2 * FLEX_SPACE.width,
                height: 2 * FLEX_SPACE.height,
            }
        );
    }

    #[test]
    fn a_row_measures_its_children_side_by_side_with_gaps() {
        let tree = row_box_of(vec![text("Hello"), new_box(), text("World")]);
        let empty_box = measure(&new_window(vec![new_box()]), &[0]);
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
            boxes: new_window(vec![Tree::new(
                row(),
                vec![text("Hello"), new_box(), text("World")],
            )]),
            ..FlexState::default()
        }
    }

    #[test]
    fn a_column_measures_its_children_stacked_with_gaps() {
        let mut state = hello_box_world();
        state.boxes.value_mut(&[0]).direction = Direction::Column;
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
    fn the_outer_box_grows_to_fit_the_row_in_height_and_stays_centred() {
        let state = hello_box_world();
        let placements = placements(&state);
        let outer = all_boxes(&placements)[0];
        let measured = measure(&state.boxes, &[0]);
        assert_eq!((outer.width, outer.height), (WINDOW.cols, measured.height));
        let left = outer.x - WINDOW.col;
        let right = WINDOW.col + WINDOW.cols - (outer.x + outer.width);
        let top = outer.y - WINDOW.row;
        let bottom = WINDOW.row + WINDOW.rows - (outer.y + outer.height);
        assert!((left - right).abs() <= 1);
        assert!((top - bottom).abs() <= 1);
    }

    #[test]
    fn in_move_a_box_then_a_text_are_added_to_the_right_in_order() {
        let state = ["A", "o", "W", "o", "r", "l", "d"]
            .into_iter()
            .fold(with_text_row("Hello"), |state, key| reduce(state, key).0);
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

    fn from_row_after(keys: &[&str]) -> FlexState {
        let state = FlexState {
            boxes: new_window(vec![Tree::new(row(), vec![])]),
            ..FlexState::default()
        };
        keys.iter().fold(state, |state, key| reduce(state, key).0)
    }

    fn empty_box_size() -> Size {
        measure(&new_window(vec![new_box()]), &[0])
    }

    #[test]
    fn one_a_places_an_empty_inner_box_one_space_inside_the_outer_box() {
        let state = from_default_after(&["A"]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        let (outer, inner) = (boxes[0], boxes[1]);
        let empty = empty_box_size();
        assert_eq!(
            (outer.width, outer.height),
            (WINDOW.cols, empty.height + 2 * FLEX_SPACE.height)
        );
        assert_eq!(
            (inner.x, inner.y, inner.width, inner.height),
            (
                outer.x + FLEX_SPACE.width,
                outer.y + FLEX_SPACE.height,
                outer.width - 2 * FLEX_SPACE.width,
                empty.height
            )
        );
    }

    #[test]
    fn a_second_a_adds_an_inner_box_one_gap_to_the_right_and_stays_centred() {
        let state = from_row_after(&["A", "A"]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        let (outer, first, second) = (boxes[0], boxes[1], boxes[2]);
        let empty = empty_box_size();
        assert_eq!(
            (outer.width, outer.height),
            (WINDOW.cols, empty.height + 2 * FLEX_SPACE.height)
        );
        assert_eq!(
            (second.x, second.y),
            (first.x + first.width + FLEX_SPACE.width, first.y)
        );
        let left = outer.x - WINDOW.col;
        let right = WINDOW.col + WINDOW.cols - (outer.x + outer.width);
        let top = outer.y - WINDOW.row;
        let bottom = WINDOW.row + WINDOW.rows - (outer.y + outer.height);
        assert!((left - right).abs() <= 1);
        assert!((top - bottom).abs() <= 1);
    }

    const EVEN_SPREAD_WINDOW: Area = Area { cols: 82, ..WINDOW };

    fn spread(children: Vec<Tree<FlexBox>>) -> FlexState {
        FlexState {
            boxes: new_window(vec![Tree::new(
                FlexBox {
                    justify: Justify::SpaceBetween,
                    ..FlexBox::default()
                },
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
    fn a_space_between_outer_box_spreads_mixed_siblings_edge_to_edge_with_equal_gaps() {
        let state = spread(vec![text("Hello"), new_box(), text("World")]);
        let layout = Layout::arrange(&state.boxes, EVEN_SPREAD_WINDOW);
        let [(_, placements)] =
            <[_; 1]>::try_from(scene(&state, &layout, &HashSet::new())).unwrap();
        let row = row_of(&placements, &state);
        assert_spread_edge_to_edge(&placements, &row);
        let gap = free_space(EVEN_SPREAD_WINDOW, &row) / 2;
        assert_eq!(gaps_between(&row), vec![gap, gap]);
    }

    #[test]
    fn s_spreads_hello_the_inner_box_and_world_across_the_outer_box() {
        let keys = [
            "o", "H", "e", "l", "l", "o", "\r", "\r", "A", "o", "W", "o", "r", "l", "d", "\r",
            "\r", "s",
        ];
        let state = keys
            .into_iter()
            .fold(FlexState::default(), |state, key| reduce(state, key).0);
        let layout = Layout::arrange(&state.boxes, EVEN_SPREAD_WINDOW);
        let [(_, placements)] =
            <[_; 1]>::try_from(scene(&state, &layout, &HashSet::new())).unwrap();
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

    fn assert_stretched_across(outer: &Placement<'_>, siblings: &[&Placement<'_>]) {
        for sibling in siblings {
            assert_eq!(
                (sibling.x, sibling.width),
                (
                    outer.x + FLEX_SPACE.width,
                    outer.width - 2 * FLEX_SPACE.width
                )
            );
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
    fn in_column_siblings_stretch_across_the_box() {
        let state = in_column(hello_box_world());
        let placements = placements(&state);
        let column = row_of(&placements, &state);
        assert_stretched_across(&placements[0], &column);
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
    fn an_outer_column_stretches_its_siblings_across_the_window() {
        let state = in_column(spread(vec![text("Hello"), new_box(), text("World")]));
        let placements = placements(&state);
        let outer = &placements[0];
        assert_eq!((outer.x, outer.width), (WINDOW.col, WINDOW.cols));
        assert_stretched_across(outer, &row_of(&placements, &state));
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

    const HELLO_WITH_AN_INNER_BOX: [&str; 9] = ["o", "H", "e", "l", "l", "o", "\r", "\r", "A"];

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
        let state = pressed(&["o", "H", "e", "l", "l", "o", "\r", "\r", "f", "A"]);
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

    #[test]
    fn a_selected_borderless_leaf_paints_its_label_selected_and_no_box_of_its_own() {
        let state = world_selected(FlexMode::Move);
        let placements = placements(&state);
        assert_eq!(all_boxes(&placements).len(), 2);
        assert_eq!(all_labels(&placements).len(), 2);
        assert_eq!(
            text_colours(&placements, &["World"]),
            vec![FLEX_SELECTED_COLOUR]
        );
    }

    #[test]
    fn a_selected_bordered_box_with_text_paints_a_selected_border_and_a_text_coloured_label() {
        let state = FlexState {
            mode: FlexMode::Move,
            selected: vec![0],
            boxes: new_window(vec![Tree::leaf(FlexBox {
                text: Some("Title".to_string()),
                ..FlexBox::default()
            })]),
            ..FlexState::default()
        };
        let placements = placements(&state);
        assert_eq!(borders(&placements), vec![FLEX_SELECTED_COLOUR]);
        assert_eq!(
            text_colours(&placements, &["Title"]),
            vec![FLEX_TEXT_COLOUR]
        );
    }

    #[test]
    fn the_window_root_paints_nothing() {
        let state = FlexState {
            mode: FlexMode::Move,
            selected: vec![],
            boxes: new_window(vec![]),
            ..FlexState::default()
        };
        assert!(placements(&state).is_empty());
    }

    fn title_and_hi_in(direction: Direction) -> FlexState {
        let parent = FlexBox {
            direction,
            ..FlexBox::default()
        };
        let inner = Tree::new(FlexBox::default(), vec![text("Hi")]);
        FlexState {
            mode: FlexMode::Move,
            selected: vec![0, 1],
            boxes: new_window(vec![Tree::new(parent, vec![text("Title"), inner])]),
            ..FlexState::default()
        }
    }

    fn after(state: FlexState, keys: &[&str]) -> FlexState {
        keys.iter().fold(state, |state, key| reduce(state, key).0)
    }

    fn outer_and_inner<'a>(
        placements: &'a [Placement<'a>],
    ) -> (&'a Placement<'a>, &'a Placement<'a>) {
        let [outer, inner] = <[_; 2]>::try_from(all_boxes(placements)).unwrap();
        (outer, inner)
    }

    #[test]
    fn with_no_key_pressed_the_first_outer_box_spans_the_window() {
        let state = FlexState::default();
        let placements = placements(&state);
        let the_box = the_box(&placements);
        assert_eq!((the_box.x, the_box.width), (WINDOW.col, WINDOW.cols));
    }

    #[test]
    fn a_second_outer_box_added_with_a_spans_the_window_too() {
        let state = pressed(&["a"]);
        let placements = placements(&state);
        let boxes = all_boxes(&placements);
        assert_eq!(boxes.len(), 2);
        for the_box in boxes {
            assert_eq!((the_box.x, the_box.width), (WINDOW.col, WINDOW.cols));
        }
    }

    #[test]
    fn in_a_column_an_inner_box_stretches_to_the_parent_inside_width() {
        let state = in_column(with_inner_boxes("Hello", 1));
        let placements = placements(&state);
        let (outer, inner) = outer_and_inner(&placements);
        assert_eq!(inner.x, outer.x + FLEX_SPACE.width);
        assert_eq!(inner.width, outer.width - 2 * FLEX_SPACE.width);
    }

    #[test]
    fn in_a_column_a_text_stretches_to_the_parent_inside_width_from_its_inside_left_edge() {
        let state = title_and_hi_in(Direction::Column);
        let placements = placements(&state);
        let (outer, _) = outer_and_inner(&placements);
        let title = label_showing(&placements, "Title");
        assert_eq!(
            (title.x, title.width),
            (
                outer.x + FLEX_SPACE.width,
                outer.width - 2 * FLEX_SPACE.width
            )
        );
    }

    fn inside_width(outer: &Placement<'_>) -> i64 {
        outer.width - 2 * FLEX_SPACE.width
    }

    fn shared_widths(room: i64, count: i64) -> (i64, i64) {
        let free = room - FLEX_SPACE.width * (count - 1);
        (free.div_euclid(count), free.rem_euclid(count))
    }

    fn assert_fills_the_inside_with_space_gaps(outer: &Placement<'_>, row: &[&Placement<'_>]) {
        let last = row[row.len() - 1];
        assert_eq!(row[0].x, outer.x + FLEX_SPACE.width);
        assert_eq!(gaps_between(row), vec![FLEX_SPACE.width; row.len() - 1]);
        assert_eq!(
            last.x + last.width,
            outer.x + outer.width - FLEX_SPACE.width
        );
    }

    #[test]
    fn in_a_row_children_share_the_inside_width_equally_with_space_gaps() {
        let state = hello_box_world();
        let placements = placements(&state);
        let row = row_of(&placements, &state);
        let (share, remainder) = shared_widths(inside_width(&placements[0]), row.len() as i64);
        assert_eq!(remainder, 0);
        assert_eq!(
            row.iter().map(|child| child.width).collect::<Vec<_>>(),
            vec![share; row.len()]
        );
        assert_fills_the_inside_with_space_gaps(&placements[0], &row);
    }

    #[test]
    fn in_a_row_the_leftover_columns_go_to_the_leftmost_children_one_each() {
        let state = with_inner_boxes_row("Hello", 3);
        let placements = placements(&state);
        let row = row_of(&placements, &state);
        let (share, remainder) = shared_widths(inside_width(&placements[0]), row.len() as i64);
        assert!(remainder > 1);
        let mut expected = vec![share + 1; remainder as usize];
        expected.resize(row.len(), share);
        assert_eq!(
            row.iter().map(|child| child.width).collect::<Vec<_>>(),
            expected
        );
        assert_fills_the_inside_with_space_gaps(&placements[0], &row);
    }

    #[test]
    fn in_a_row_children_keep_their_measured_height_and_are_centred_vertically() {
        let state = title_and_hi_in(Direction::Row);
        let placements = placements(&state);
        let (outer, inner) = outer_and_inner(&placements);
        let title = label_showing(&placements, "Title");
        assert_eq!(
            (title.height, inner.height),
            (
                measure(&state.boxes, &[0, 0]).height,
                measure(&state.boxes, &[0, 1]).height
            )
        );
        let inside_top = outer.y + FLEX_SPACE.height;
        let inside_height = outer.height - 2 * FLEX_SPACE.height;
        for child in [title, inner] {
            assert_eq!(child.y - inside_top, (inside_height - child.height) / 2);
        }
    }

    #[test]
    fn r_on_the_parent_of_an_inner_box_in_a_row_stretches_it_to_the_parent_inside_width() {
        let state = after(title_and_hi_in(Direction::Row), &["\x7f", "r"]);
        let placements = placements(&state);
        let (outer, inner) = outer_and_inner(&placements);
        assert_eq!(inner.x, outer.x + FLEX_SPACE.width);
        assert_eq!(
            inner.x + inner.width,
            outer.x + outer.width - FLEX_SPACE.width
        );
    }

    #[test]
    fn close_bracket_adds_one_space_width_of_padding_on_the_left_and_right_of_the_box() {
        let state = after(with_text("Hello"), &["]"]);
        let placements = placements(&state);
        let the_box = the_box(&placements);
        let label = the_label(&placements);
        assert_eq!(label.x, the_box.x + 2 * FLEX_SPACE.width);
        assert_eq!(
            label.x + label.width,
            the_box.x + the_box.width - 2 * FLEX_SPACE.width
        );
        assert_eq!(label.y, the_box.y + FLEX_SPACE.height);
        assert_eq!(
            label.y + label.height,
            the_box.y + the_box.height - FLEX_SPACE.height
        );
    }

    fn the_box_width(placements: &[Placement<'_>]) -> i64 {
        the_box(placements).width
    }

    #[test]
    fn close_bracket_twice_adds_two_space_widths_of_padding_on_each_side() {
        let before = with_text("Hello");
        let state = after(before.clone(), &["]", "]"]);
        assert_eq!(
            measure(&state.boxes, &[0]).width,
            measure(&before.boxes, &[0]).width + 4 * FLEX_SPACE.width
        );
    }

    #[test]
    fn close_bracket_on_an_inner_box_widens_it_and_its_parent() {
        let before = title_and_hi_in(Direction::Row);
        let state = after(before.clone(), &["]"]);
        assert_eq!(
            measure(&state.boxes, &[0, 1]).width,
            measure(&before.boxes, &[0, 1]).width + 2 * FLEX_SPACE.width
        );
        assert_eq!(
            measure(&state.boxes, &[0]).width,
            measure(&before.boxes, &[0]).width + 2 * FLEX_SPACE.width
        );
    }

    #[test]
    fn close_bracket_on_an_outer_box_keeps_its_width_and_moves_its_text_in() {
        let before = with_text("Hello");
        let state = after(before.clone(), &["]"]);
        let before_width = the_box_width(&placements(&before));
        let placements = placements(&state);
        let the_box = the_box(&placements);
        assert_eq!(the_box.width, before_width);
        assert_eq!(the_label(&placements).x, the_box.x + 2 * FLEX_SPACE.width);
    }

    fn titled(direction: Direction, title: Option<&str>) -> FlexState {
        let parent = FlexBox {
            direction,
            text: title.map(str::to_string),
            ..FlexBox::default()
        };
        FlexState {
            boxes: new_window(vec![Tree::new(parent, vec![new_box(), new_box()])]),
            ..FlexState::default()
        }
    }

    fn the_label_and_inner_boxes<'a>(
        placements: &'a [Placement<'a>],
    ) -> (&'a Placement<'a>, Vec<&'a Placement<'a>>) {
        let inner = all_boxes(placements).into_iter().skip(1).collect();
        (the_label(placements), inner)
    }

    #[test]
    fn in_a_column_an_own_text_comes_first_one_gap_above_the_inner_boxes() {
        let state = titled(Direction::Column, Some("Title"));
        let placements = placements(&state);
        let (label, inner) = the_label_and_inner_boxes(&placements);
        assert_eq!(label.height, 1);
        assert_eq!(label.y + label.height + FLEX_SPACE.height, inner[0].y);
        assert_eq!(label.x, inner[0].x);
        assert_eq!(label.width, inner[0].width);
    }

    #[test]
    fn in_a_row_an_own_text_comes_first_one_gap_before_the_inner_boxes() {
        let state = titled(Direction::Row, Some("Title"));
        let placements = placements(&state);
        let (label, inner) = the_label_and_inner_boxes(&placements);
        assert_eq!(label.x + label.width + FLEX_SPACE.width, inner[0].x);
        assert!(inner[0].x < inner[1].x);
    }

    #[test]
    fn a_box_with_an_own_text_measures_taller_in_a_column_by_a_row_and_a_gap() {
        let without = measure(&titled(Direction::Column, None).boxes, &[0]);
        let with = measure(&titled(Direction::Column, Some("Title")).boxes, &[0]);
        assert_eq!(with.height, without.height + 1 + FLEX_SPACE.height);
    }

    #[test]
    fn a_box_with_no_text_places_only_its_borders() {
        let state = titled(Direction::Column, None);
        let placements = placements(&state);
        assert_eq!(placements.len(), all_boxes(&placements).len());
        assert_eq!(placements.len(), 3);
    }

    #[test]
    fn a_box_with_an_empty_own_text_keeps_a_slot_for_it() {
        let empty = measure(&titled(Direction::Column, Some("")).boxes, &[0]);
        let none = measure(&titled(Direction::Column, None).boxes, &[0]);
        assert_eq!(empty.height, none.height + 1 + FLEX_SPACE.height);
    }

    fn with_selected_text(text: &str) -> FlexState {
        FlexState {
            selected: vec![0, 0],
            ..with_text(text)
        }
    }

    fn in_replace(text: &str) -> FlexState {
        after(with_selected_text(text), &["i"])
    }

    fn fill(placement: &Placement<'_>) -> Option<u8> {
        let PlacementNode::Box { fill, .. } = placement.node else {
            unreachable!()
        };
        fill
    }

    fn opacity(placement: &Placement<'_>) -> Option<f64> {
        let PlacementNode::Box { opacity, .. } = placement.node else {
            unreachable!()
        };
        opacity
    }

    fn is_highlight(placement: &Placement<'_>) -> bool {
        matches!(
            &placement.node,
            PlacementNode::Box { fill: Some(colour), opacity, .. }
                if *colour == FOREGROUND && *opacity == Some(FLEX_REPLACE_OPACITY)
        )
    }

    fn highlight<'a>(placements: &'a [Placement<'a>]) -> Option<&'a Placement<'a>> {
        placements.iter().find(|placement| is_highlight(placement))
    }

    #[test]
    fn pressing_i_on_a_box_with_text_highlights_the_label() {
        let state = in_replace("Hello");
        let placements = placements(&state);
        let highlight = highlight(&placements).unwrap();
        let label = the_label(&placements);
        assert_eq!(
            (fill(highlight), opacity(highlight)),
            (Some(FOREGROUND), Some(FLEX_REPLACE_OPACITY))
        );
        assert_eq!(
            (highlight.x, highlight.y, highlight.width, highlight.height),
            (label.x, label.y, "Hello".chars().count() as i64, 1)
        );
    }

    #[test]
    fn the_replace_highlight_comes_before_the_label() {
        let state = in_replace("Hello");
        let placements = placements(&state);
        let highlight = placements
            .iter()
            .position(|placement| is_highlight(placement))
            .unwrap();
        let label = placements
            .iter()
            .position(|placement| matches!(placement.node, PlacementNode::Label(_)))
            .unwrap();
        assert!(highlight < label);
    }

    #[test]
    fn the_replace_highlight_width_counts_characters_not_bytes() {
        let state = in_replace("café");
        let placements = placements(&state);
        let highlight = highlight(&placements).unwrap();
        assert_eq!(highlight.width, "café".chars().count() as i64);
        assert_ne!(highlight.width, "café".len() as i64);
    }

    #[test]
    fn pressing_i_on_a_box_with_no_text_leaves_no_highlight() {
        let state = after(with_text(""), &["i"]);
        assert!(highlight(&placements(&state)).is_none());
    }

    #[test]
    fn the_o_flow_leaves_no_replace_highlight() {
        let state = after(with_text("Hello"), &["o"]);
        assert!(highlight(&placements(&state)).is_none());
    }

    #[test]
    fn the_enter_adds_a_box_flow_leaves_no_replace_highlight() {
        let state = FlexState {
            mode: FlexMode::Write,
            selected: vec![0, 0],
            ..with_text("Hello")
        };
        let state = after(state, &["\r"]);
        assert!(highlight(&placements(&state)).is_none());
    }

    #[test]
    fn after_the_first_replace_key_there_is_no_highlight() {
        let state = after(with_selected_text("Hello"), &["i", "W"]);
        assert!(highlight(&placements(&state)).is_none());
    }

    #[test]
    fn backspace_as_the_first_replace_key_leaves_no_highlight() {
        let state = after(with_selected_text("Hello"), &["i", "\x7f"]);
        assert!(highlight(&placements(&state)).is_none());
    }

    #[test]
    fn enter_as_the_first_replace_key_leaves_no_highlight() {
        let state = after(with_selected_text("Hello"), &["i", "\r"]);
        assert!(highlight(&placements(&state)).is_none());
    }

    fn writing(state: FlexState) -> FlexState {
        FlexState {
            mode: FlexMode::Write,
            selected: vec![0, 0],
            ..state
        }
    }

    fn the_typing_caret<'a>(placements: &'a [Placement<'a>]) -> &'a Placement<'a> {
        placements
            .iter()
            .find(|placement| matches!(placement.node, PlacementNode::TypingCaret { .. }))
            .unwrap()
    }

    fn typing_caret_at(x: i64, y: i64) -> Placement<'static> {
        Placement {
            node: PlacementNode::TypingCaret {
                colour: FLEX_TEXT_COLOUR,
                bold: false,
            },
            x,
            y,
            width: 1,
            height: 1,
            depth: 0,
        }
    }

    #[test]
    fn write_mode_puts_a_typing_caret_one_cell_past_the_label() {
        let state = writing(with_text("Hello"));
        let placements = placements(&state);
        let label = the_label(&placements);
        let caret = the_typing_caret(&placements);
        let PlacementNode::Label(Label { text, .. }) = &label.node else {
            unreachable!()
        };
        assert_eq!(caret.x, label.x + text.chars().count() as i64);
        assert_eq!(caret.y, label.y);
        assert_eq!((caret.width, caret.height), (1, 1));
        let PlacementNode::TypingCaret { colour, bold } = caret.node else {
            unreachable!()
        };
        assert_eq!(colour, FLEX_TEXT_COLOUR);
        assert!(!bold);
    }

    #[test]
    fn an_empty_label_puts_the_caret_in_the_first_cell() {
        let state = writing(with_text(""));
        let placements = placements(&state);
        assert_eq!(the_typing_caret(&placements).x, the_label(&placements).x);
    }

    #[test]
    fn typing_the_next_character_moves_the_caret_right_by_one_cell() {
        let before_state = writing(with_text("Hello"));
        let before_caret = the_typing_caret(&placements(&before_state)).x;
        let before_label = the_label(&placements(&before_state)).x;
        let (typed, _) = reduce(before_state, "x");
        let after = placements(&typed);
        assert_eq!(the_typing_caret(&after).x, before_caret + 1);
        assert_eq!(the_label(&after).x, before_label);
    }

    #[test]
    fn backspacing_to_empty_leaves_the_caret_in_the_first_cell() {
        let (emptied, _) = reduce(writing(with_text("x")), "\x7f");
        let placements = placements(&emptied);
        assert_eq!(the_typing_caret(&placements).x, the_label(&placements).x);
    }

    #[test]
    fn move_mode_draws_no_typing_caret() {
        let state = with_text("Hello");
        let placements = placements(&state);
        assert!(!placements
            .iter()
            .any(|placement| matches!(placement.node, PlacementNode::TypingCaret { .. })));
    }

    #[test]
    fn the_caret_placement_is_one_cell_wide_and_does_not_widen_the_label() {
        let plain_state = with_text_row("Hello");
        let writing_state = writing(with_text_row("Hello"));
        let plain = placements(&plain_state);
        let writing = placements(&writing_state);
        let caret = the_typing_caret(&writing);
        assert_eq!((caret.width, caret.height), (1, 1));
        assert_eq!(the_label(&writing).width, the_label(&plain).width);
    }

    #[test]
    fn the_caret_is_a_decoration_and_stays_out_of_the_bounding_box() {
        assert!(PlacementNode::TypingCaret {
            colour: FLEX_TEXT_COLOUR,
            bold: false,
        }
        .is_decoration());
        let content_state = with_text("Hello");
        let content = placements(&content_state);
        let content_only = view::centre(content.clone(), WINDOW);
        let mut decorated = content.clone();
        decorated.push(typing_caret_at(
            content[0].x + content[0].width + 5,
            content[0].y + content[0].height + 5,
        ));
        let placed = view::centre(decorated, WINDOW);
        assert_eq!(&placed[..content_only.len()], &content_only[..]);
    }

    #[test]
    fn a_bordered_box_insets_its_children_by_at_least_one_row() {
        let state = FlexState {
            boxes: new_window(vec![Tree::new(FlexBox::default(), vec![new_box()])]),
            ..FlexState::default()
        };
        let layout = Layout::arrange(&state.boxes, WINDOW);
        let parent = layout.rect(&[0]).unwrap();
        let child = layout.rect(&[0, 0]).unwrap();
        assert!(child.y > parent.y);
    }

    #[test]
    fn a_borderless_box_lays_its_first_child_on_its_own_rect() {
        let state = FlexState {
            boxes: new_window(vec![Tree::new(
                FlexBox {
                    border: false,
                    ..FlexBox::default()
                },
                vec![new_box()],
            )]),
            ..FlexState::default()
        };
        let layout = Layout::arrange(&state.boxes, WINDOW);
        let parent = layout.rect(&[0]).unwrap();
        let child = layout.rect(&[0, 0]).unwrap();
        assert_eq!(child, parent);
    }
}
