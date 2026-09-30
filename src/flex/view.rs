use std::borrow::Cow;

use crate::view::{self, Area, Label, Placement, PlacementNode, Rgb, Scene, ALL_SIDES, BOX_HEIGHT};

use super::state::{FlexMode, FlexState, FlexWidth, Justify};

const FLEX_BORDER: i64 = 1;
const FLEX_GAP: i64 = 1;
const TEXT_GAP: i64 = 1;
const FLEX_BORDER_COLOUR: Rgb = (0x2A, 0x2A, 0x2E);
const FLEX_SELECTED_COLOUR: Rgb = (0x8A, 0xB4, 0xF8);
const FLEX_TEXT_COLOUR: Rgb = (0xC9, 0xC9, 0xCF);
const FLEX_FILL_COLOUR: Rgb = (0x14, 0x14, 0x16);

fn text_offsets(texts: &[String], justify: Justify, inner_width: i64) -> Vec<i64> {
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

pub(crate) fn scene(state: &FlexState, window: Area) -> Scene<'_> {
    let mut placements = Vec::with_capacity(state.outer_boxes().count() * 2);
    for (i, flex_box) in (0..).zip(state.outer_boxes()) {
        let width = match flex_box.width {
            FlexWidth::Fit => {
                let interiors: i64 = flex_box.texts.iter().map(|text| view::interior(text)).sum();
                interiors + TEXT_GAP * (flex_box.texts.len() as i64 - 1) + 2
            }
            FlexWidth::Full => window.cols,
        };
        let offsets = text_offsets(&flex_box.texts, flex_box.justify, width - 2);
        let x = -width.div_euclid(2);
        let y = i * (BOX_HEIGHT + FLEX_GAP);
        let colour = if state.mode == FlexMode::Move && state.selected == [i as usize] {
            FLEX_SELECTED_COLOUR
        } else {
            FLEX_BORDER_COLOUR
        };
        placements.push(Placement {
            node: PlacementNode::Box {
                colour,
                fill: None,
                opacity: None,
                solid_fill: flex_box.filled.then_some(FLEX_FILL_COLOUR),
                rounded: false,
                sides: ALL_SIDES,
                border: FLEX_BORDER,
            },
            x,
            y,
            width,
            height: BOX_HEIGHT,
        });
        for (text, offset) in flex_box.texts.iter().zip(offsets) {
            placements.push(Placement {
                node: PlacementNode::Label(Label {
                    text: Cow::Borrowed(text.as_str()),
                    colour: FLEX_TEXT_COLOUR,
                    bold: false,
                }),
                x: x + 1 + offset,
                y: y + BOX_HEIGHT / 2,
                width: view::interior(text),
                height: 1,
            });
        }
    }
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
    use crate::flex::state::{reduce, FlexBox, Justify};
    use types::Tree;

    const WINDOW: Area = Area {
        col: 0,
        row: 0,
        cols: 80,
        rows: 24,
    };

    fn holding(the_box: FlexBox) -> FlexState {
        FlexState {
            boxes: Tree::root(vec![Tree::leaf(the_box)]),
            ..FlexState::default()
        }
    }

    fn a_box(text: &str) -> FlexBox {
        FlexBox {
            texts: vec![text.to_string()],
            ..FlexBox::default()
        }
    }

    fn with_text(text: &str) -> FlexState {
        holding(a_box(text))
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
        assert_eq!(text, &state.outer_boxes().last().unwrap().texts[0]);
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
        holding(FlexBox {
            width: FlexWidth::Full,
            ..a_box(text)
        })
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
            boxes: Tree::root(texts.iter().map(|text| Tree::leaf(a_box(text))).collect()),
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
        let state = holding(FlexBox {
            filled: true,
            ..a_box("Hello")
        });
        let placements = placements(&state);
        assert_eq!(solid_fill(the_box(&placements)), Some(FLEX_FILL_COLOUR));
    }

    fn beside(texts: &[&str]) -> FlexState {
        holding(FlexBox {
            texts: texts.iter().map(|text| text.to_string()).collect(),
            ..FlexBox::default()
        })
    }

    fn margins(placements: &[Placement<'_>]) -> (i64, i64) {
        let the_box = the_box(placements);
        (
            the_box.x - WINDOW.col,
            WINDOW.col + WINDOW.cols - (the_box.x + the_box.width),
        )
    }

    fn owned(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| text.to_string()).collect()
    }

    #[test]
    fn each_text_starts_after_the_previous_one_and_a_gap() {
        let texts = owned(&["Hello", "World", ""]);
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
        let state = holding(FlexBox {
            width: FlexWidth::Full,
            ..FlexBox {
                texts: owned(&["Hello", "World"]),
                ..FlexBox::default()
            }
        });
        let placements = placements(&state);
        let the_box = the_box(&placements);
        assert_eq!(the_box.width, WINDOW.cols);
        assert_eq!(all_labels(&placements)[0].x, the_box.x + 1);
    }

    #[test]
    fn start_ignores_the_inner_width() {
        let texts = owned(&["Hello", "World"]);
        assert_eq!(text_offsets(&texts, Justify::Start, 78), vec![0, 6]);
    }

    #[test]
    fn space_between_puts_the_last_text_at_the_inner_right_edge() {
        let texts = owned(&["Hello", "World"]);
        assert_eq!(text_offsets(&texts, Justify::SpaceBetween, 78), vec![0, 73]);
    }

    #[test]
    fn space_between_splits_the_free_space_evenly() {
        let texts = owned(&["a", "b", "c"]);
        assert_eq!(
            text_offsets(&texts, Justify::SpaceBetween, 9),
            vec![0, 4, 8]
        );
    }

    #[test]
    fn space_between_gives_the_extra_cells_to_the_right_gaps() {
        assert_eq!(
            text_offsets(&owned(&["a", "b", "c"]), Justify::SpaceBetween, 10),
            vec![0, 4, 9]
        );
        assert_eq!(
            text_offsets(&owned(&["a", "b", "c", "d"]), Justify::SpaceBetween, 12),
            vec![0, 3, 7, 11]
        );
    }

    #[test]
    fn space_between_with_one_text_sits_at_the_start() {
        assert_eq!(
            text_offsets(&owned(&["Hello"]), Justify::SpaceBetween, 78),
            vec![0]
        );
    }

    #[test]
    fn space_between_in_the_packed_width_matches_start() {
        let texts = owned(&["Hello", "World", ""]);
        let interiors: i64 = texts.iter().map(|text| view::interior(text)).sum();
        let packed = interiors + TEXT_GAP * (texts.len() as i64 - 1);
        assert_eq!(
            text_offsets(&texts, Justify::SpaceBetween, packed),
            text_offsets(&texts, Justify::Start, packed)
        );
    }

    fn full_beside(texts: &[&str], justify: Justify) -> FlexState {
        holding(FlexBox {
            width: FlexWidth::Full,
            justify,
            texts: owned(texts),
            ..FlexBox::default()
        })
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
        let spread = holding(FlexBox {
            justify: Justify::SpaceBetween,
            ..start.outer_boxes().next().unwrap().clone()
        });
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
}
