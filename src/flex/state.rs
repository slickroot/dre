use super::history::{self, Snapshot};
use types::Tree;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlexMode {
    Write,
    Move,
    Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlexEffect {
    Quit,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Justify {
    #[default]
    Start,
    SpaceBetween,
    #[allow(dead_code)]
    Center,
}

impl Justify {
    pub(crate) fn toggle(self) -> Self {
        match self {
            Justify::Start => Justify::SpaceBetween,
            Justify::SpaceBetween | Justify::Center => Justify::Start,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Direction {
    Row,
    #[default]
    Column,
}

impl Direction {
    pub(crate) fn toggle(self) -> Self {
        match self {
            Direction::Row => Direction::Column,
            Direction::Column => Direction::Row,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlexBox {
    pub(crate) text: Option<String>,
    pub(crate) border: bool,
    pub(crate) justify: Justify,
    pub(crate) direction: Direction,
    pub(crate) filled: bool,
    pub(crate) padding: u16,
}

impl Default for FlexBox {
    fn default() -> Self {
        Self {
            text: None,
            border: true,
            justify: Justify::default(),
            direction: Direction::default(),
            filled: false,
            padding: 0,
        }
    }
}

impl FlexBox {
    pub(crate) fn window() -> Self {
        Self {
            border: false,
            justify: Justify::Start,
            text: None,
            ..Self::default()
        }
    }
}

pub(crate) fn new_box() -> Tree<FlexBox> {
    Tree::new(FlexBox::default(), vec![])
}

pub(crate) fn new_window(children: Vec<Tree<FlexBox>>) -> Tree<FlexBox> {
    Tree::new(FlexBox::window(), children)
}

#[derive(Debug, Clone)]
pub(crate) struct FlexState {
    pub(crate) boxes: Tree<FlexBox>,
    pub(crate) selected: Vec<usize>,
    pub(crate) mode: FlexMode,
    pub(crate) clipboard: Option<Tree<FlexBox>>,
    pub(crate) history: Vec<Snapshot>,
}

impl PartialEq for FlexState {
    fn eq(&self, other: &Self) -> bool {
        self.boxes == other.boxes && self.selected == other.selected && self.mode == other.mode
    }
}

impl Default for FlexState {
    fn default() -> Self {
        Self {
            boxes: new_window(vec![new_box()]),
            selected: vec![0],
            mode: FlexMode::Move,
            clipboard: None,
            history: Vec::new(),
        }
    }
}

impl FlexState {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn outer_boxes(&self) -> impl Iterator<Item = &FlexBox> {
        self.boxes
            .walk()
            .filter(|(path, _)| path.len() == 1)
            .map(|(_, flex_box)| flex_box)
    }

    fn selected_mut(&mut self) -> &mut FlexBox {
        self.boxes.value_mut(&self.selected)
    }
}

pub(crate) fn reduce(state: FlexState, key: &str) -> (FlexState, Option<FlexEffect>) {
    match (key, state.mode) {
        ("\x03", _) => (state, Some(FlexEffect::Quit)),
        ("\r", FlexMode::Write) => history::recorded(state, key, |s| write_key(s, key)),
        (_, FlexMode::Write) => write_key(state, key),
        (_, FlexMode::Replace) => write_key(state, key),
        (_, FlexMode::Move) => history::recorded(state, key, |s| move_key(s, key)),
    }
}

fn is_droppable(b: &FlexBox, children: &[Vec<usize>]) -> bool {
    !b.border && children.is_empty()
}

fn sibling_or_else_parent(boxes: &Tree<FlexBox>, path: &[usize]) -> Vec<usize> {
    if path.last() == Some(&0) {
        boxes.parent(path)
    } else {
        boxes.previous(path)
    }
}

fn drop_empty_text(mut state: FlexState) -> FlexState {
    let children = state.boxes.children(&state.selected);
    if is_droppable(state.boxes.value(&state.selected), &children) {
        let parent = state.boxes.parent(&state.selected);
        state.boxes.remove(&state.selected);
        state.selected = parent;
    } else {
        state.selected_mut().text = None;
    }
    state
}

fn exit_write_mode(mut state: FlexState) -> FlexState {
    if state.selected_mut().text.as_deref() == Some("") {
        state = drop_empty_text(state);
    }
    state.mode = FlexMode::Move;
    state
}

fn write_key(mut state: FlexState, key: &str) -> (FlexState, Option<FlexEffect>) {
    if state.mode == FlexMode::Replace {
        match key {
            "\x7f" => {
                return history::recorded(state, key, |mut state| {
                    state.selected_mut().text = Some(String::new());
                    state.mode = FlexMode::Write;
                    (state, None)
                });
            }
            "\r" => state.mode = FlexMode::Write,
            _ if printable_char(key).is_some() => {
                state = history::recorded(state, key, |mut state| {
                    state.selected_mut().text = Some(String::new());
                    state.mode = FlexMode::Write;
                    (state, None)
                })
                .0;
            }
            _ => {}
        }
    }
    match key {
        "\r" => {
            if state.selected_mut().text.as_deref() != Some("") {
                let new_box = copied_box(state.boxes.value(&state.selected));
                let parent = parent_path(&state.selected);
                let index = state.selected.last().copied().unwrap() + 1;
                state.selected = state
                    .boxes
                    .insert(&parent, index, Tree::new(new_box, vec![]));
                return (state, None);
            }
            state = exit_write_mode(state);
        }
        "\x1b" => state = exit_write_mode(state),
        "\x7f" => {
            let children = state.boxes.children(&state.selected);
            if state.selected_mut().text.as_deref() == Some("")
                && is_droppable(state.boxes.value(&state.selected), &children)
            {
                history::record(&mut state);
                let next = sibling_or_else_parent(&state.boxes, &state.selected);
                state.boxes.remove(&state.selected);
                state.selected = next;
                state.mode = FlexMode::Move;
            } else if let Some(text) = state.selected_mut().text.as_mut() {
                text.pop();
            }
        }
        _ => {
            if let (Some(c), Some(text)) = (printable_char(key), state.selected_mut().text.as_mut())
            {
                text.push(c);
            }
        }
    }
    (state, None)
}

fn copied_box(b: &FlexBox) -> FlexBox {
    FlexBox {
        text: Some(String::new()),
        ..b.clone()
    }
}

fn move_key(mut state: FlexState, key: &str) -> (FlexState, Option<FlexEffect>) {
    match key {
        "i" => {
            let has_text = state
                .selected_mut()
                .text
                .as_deref()
                .is_some_and(|text| !text.is_empty());
            if has_text {
                state.mode = FlexMode::Replace;
            } else {
                state.selected_mut().text.get_or_insert_with(String::new);
                state.mode = FlexMode::Write;
            }
        }
        "a" => state.selected = state.boxes.push(&[], new_box()),
        "A" => {
            state.boxes.push(&state.selected, new_box());
        }
        "\r" => state.selected = state.boxes.child(&state.selected),
        "\x7f" => state.selected = state.boxes.parent(&state.selected),
        "j" | "k" | "h" | "l" => {
            let parent = parent_path(&state.selected);
            let axis = state.boxes.value(&parent).direction;
            state.selected = match (axis, key) {
                (Direction::Column, "j") => state.boxes.next(&state.selected),
                (Direction::Column, "k") => state.boxes.previous(&state.selected),
                (Direction::Row, "l") => state.boxes.next(&state.selected),
                (Direction::Row, "h") => state.boxes.previous(&state.selected),
                _ => state.selected.clone(),
            };
        }
        "o" | "O" => {
            let source = if state.selected.is_empty() {
                state.boxes.value(&[])
            } else {
                state.boxes.value(&state.selected)
            };
            let new_box = copied_box(source);
            let (parent, index) = if state.selected.is_empty() {
                (Vec::new(), state.boxes.children(&[]).len())
            } else {
                (
                    parent_path(&state.selected),
                    state.selected.last().copied().unwrap() + usize::from(key == "o"),
                )
            };
            state.selected = state
                .boxes
                .insert(&parent, index, Tree::new(new_box, vec![]));
            state.mode = FlexMode::Write;
        }
        "s" => {
            let selected = state.selected_mut();
            selected.justify = selected.justify.toggle();
        }
        "r" => {
            let selected = state.selected_mut();
            selected.direction = selected.direction.toggle();
        }
        "f" => {
            let selected = state.selected_mut();
            selected.filled = !selected.filled;
        }
        "]" => {
            let selected = state.selected_mut();
            selected.padding = selected.padding.saturating_add(1);
        }
        "u" => state = history::undo(state),
        "d" => {
            if !state.selected.is_empty() {
                state = cut_selected(state);
            }
        }
        "q" => return (state, Some(FlexEffect::Quit)),
        _ => {}
    }
    (state, None)
}

fn cut_selected(mut state: FlexState) -> FlexState {
    state.clipboard = Some(state.boxes.remove(&state.selected));
    state.selected = parent_path(&state.selected);
    state
}

fn parent_path(path: &[usize]) -> Vec<usize> {
    if path.len() == 1 {
        Vec::new()
    } else {
        path[..path.len() - 1].to_vec()
    }
}

pub(super) fn printable_char(key: &str) -> Option<char> {
    let mut chars = key.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if !c.is_control() => Some(c),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tty;

    fn box_at<'a>(state: &'a FlexState, path: &[usize]) -> &'a FlexBox {
        state.boxes.value(path)
    }

    fn text_of<'a>(state: &'a FlexState, path: &[usize]) -> Option<&'a str> {
        state.boxes.value(path).text.as_deref()
    }

    fn selected_box(state: &FlexState) -> &FlexBox {
        box_at(state, &state.selected)
    }

    fn box_with(text: &str) -> Tree<FlexBox> {
        Tree::leaf(FlexBox {
            text: Some(text.to_string()),
            ..FlexBox::default()
        })
    }

    fn holding(text: &str, mode: FlexMode) -> FlexState {
        FlexState {
            boxes: new_window(vec![box_with(text)]),
            mode,
            ..FlexState::default()
        }
    }

    #[test]
    fn starts_in_move_with_one_box_and_no_text() {
        let state = FlexState::default();
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(state.outer_boxes().count(), 1);
        assert_eq!(state.selected, [0]);
        assert_eq!(state.boxes.children(&state.selected).len(), 0);
    }

    #[test]
    fn a_new_box_has_no_children() {
        let boxes = new_window(vec![new_box()]);
        assert_eq!(
            boxes.walk().collect::<Vec<_>>(),
            [(vec![0], &FlexBox::default())]
        );
    }

    #[test]
    fn is_droppable_when_borderless_with_no_children() {
        let b = FlexBox {
            border: false,
            ..FlexBox::default()
        };
        assert!(is_droppable(&b, &[]));
    }

    #[test]
    fn is_not_droppable_when_bordered() {
        let b = FlexBox::default();
        assert!(!is_droppable(&b, &[]));
    }

    #[test]
    fn is_not_droppable_when_borderless_with_children() {
        let b = FlexBox {
            border: false,
            ..FlexBox::default()
        };
        assert!(!is_droppable(&b, &[vec![0]]));
    }

    #[test]
    fn sibling_or_else_parent_returns_previous_sibling_when_not_first_child() {
        let boxes = new_window(vec![new_box(), new_box()]);
        assert_eq!(sibling_or_else_parent(&boxes, &[1]), vec![0]);
    }

    #[test]
    fn sibling_or_else_parent_returns_parent_when_first_child() {
        let boxes = new_window(vec![new_window(vec![new_box()])]);
        assert_eq!(sibling_or_else_parent(&boxes, &[0, 0]), vec![0]);
    }

    #[test]
    fn o_in_move_mode_adds_a_copy_after_and_switches_to_write() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "o");
        assert_eq!(state.selected, [1]);
        assert_eq!(text_of(&state, &state.selected), Some(""));
        assert!(box_at(&state, &state.selected).border);
        assert_eq!(text_of(&state, &[0]), Some("Hello"));
        assert_eq!(state.outer_boxes().count(), 2);
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(effect, None);
    }

    #[test]
    fn typing_after_o_goes_into_the_new_copy() {
        let state = ["o", "W", "o", "r", "l", "d"]
            .iter()
            .fold(hello_in(FlexMode::Move), |state, key| reduce(state, key).0);
        assert_eq!(text_of(&state, &[1]), Some("World"));
        assert_eq!(text_of(&state, &[0]), Some("Hello"));
    }

    #[test]
    fn o_after_enter_on_non_empty_text_types_into_the_new_sibling() {
        let state = ["o", "W", "o", "r", "l", "d", "\r", "o"]
            .iter()
            .fold(hello_in(FlexMode::Move), |state, key| reduce(state, key).0);
        assert_eq!(state.selected, [2]);
        assert_eq!(text_of(&state, &state.selected), Some("o"));
        assert_eq!(text_of(&state, &[1]), Some("World"));
        assert_eq!(state.mode, FlexMode::Write);
    }

    #[test]
    fn o_and_capital_o_in_write_mode_type_themselves_and_add_no_box() {
        for (key, text) in [("o", "Helloo"), ("O", "HelloO")] {
            let (state, effect) = reduce(hello_in(FlexMode::Write), key);
            assert_eq!(text_of(&state, &state.selected), Some(text), "{key}");
            assert_eq!(state.mode, FlexMode::Write, "{key}");
            assert_eq!(effect, None, "{key}");
        }
    }

    #[test]
    fn backspace_empties_a_borderless_copy_and_then_removes_it() {
        let before = world_selected();
        let state = moved(before.clone(), &["o", "W", "\x7f", "\x7f"]);
        assert_eq!(state.boxes, before.boxes);
        assert_eq!(state.selected, [0, 2]);
        assert_eq!(state.mode, FlexMode::Move);
    }

    #[test]
    fn backspace_on_an_empty_borderless_box_with_a_previous_sibling_selects_it() {
        let before = world_selected();
        let state = moved(before.clone(), &["o", "\x7f"]);
        assert_eq!(state.boxes, before.boxes);
        assert_eq!(state.selected, [0, 2]);
        assert_eq!(state.mode, FlexMode::Move);
    }

    #[test]
    fn backspace_on_a_bordered_box_with_empty_text_leaves_it_in_place() {
        let before = moved(FlexState::default(), &["i"]);
        assert!(selected_box(&before).border);
        assert_eq!(text_of(&before, &before.selected), Some(""));
        assert_eq!(reduce(before.clone(), "\x7f"), (before, None));
    }

    #[test]
    fn o_inserts_a_copy_after_the_selected_box_among_its_siblings() {
        let (state, _) = reduce(middle_selected(), "o");
        assert_eq!(
            texts(&state),
            [Some("Top"), Some("Middle"), Some(""), Some("Bottom")]
        );
        assert_eq!(state.selected, [2]);
    }

    #[test]
    fn capital_o_inserts_a_copy_before_the_selected_box_among_its_siblings() {
        let (state, _) = reduce(middle_selected(), "O");
        assert_eq!(
            texts(&state),
            [Some("Top"), Some(""), Some("Middle"), Some("Bottom")]
        );
        assert_eq!(state.selected, [1]);
    }

    #[test]
    fn capital_o_on_a_top_level_box_inserts_before_the_selected_top_level_box() {
        let (state, _) = reduce(FlexState::default(), "O");
        assert_eq!(state.outer_boxes().count(), 2);
        assert_eq!(state.selected, [0]);
        assert_eq!(text_of(&state, &[0]), Some(""));
        assert_eq!(text_of(&state, &[1]), None);
    }

    #[test]
    fn o_on_a_top_level_box_inserts_after_the_selected_top_level_box() {
        let before = FlexState::default();
        let (state, _) = reduce(before, "o");
        assert_eq!(state.outer_boxes().count(), 2);
        assert_eq!(state.selected, [1]);
        assert_eq!(text_of(&state, &[0]), None);
    }

    #[test]
    fn o_and_capital_o_enter_write_mode_so_the_next_keys_fill_the_new_box() {
        for (key, filled, other) in [("o", 1usize, 0usize), ("O", 0usize, 1usize)] {
            let state = moved(FlexState::default(), &[key, "H", "i"]);
            assert_eq!(state.mode, FlexMode::Write, "{key}");
            assert_eq!(text_of(&state, &[filled]), Some("Hi"), "{key}");
            assert_eq!(text_of(&state, &[other]), None, "{key}");
        }
    }

    #[test]
    fn o_and_capital_o_copy_border_fill_padding_justify_and_direction() {
        let styled = moved(middle_selected(), &["f", "]", "r", "s"]);
        for (key, copy_index) in [("o", 2usize), ("O", 1usize)] {
            let (state, _) = reduce(styled.clone(), key);
            let source = box_at(&styled, &[1]);
            let copy = box_at(&state, &[copy_index]);
            assert_eq!(copy.text.as_deref(), Some(""));
            assert_eq!(copy.border, source.border);
            assert_eq!(copy.filled, source.filled);
            assert_eq!(copy.padding, source.padding);
            assert_eq!(copy.justify, source.justify);
            assert_eq!(copy.direction, source.direction);
            assert!(state.boxes.children(&state.selected).is_empty());
            assert_eq!(state.selected, [copy_index], "{key}");
        }
    }

    #[test]
    fn o_and_capital_o_on_a_nested_box_insert_a_sibling_within_that_parent() {
        for (key, copy_index) in [("o", 2usize), ("O", 1usize)] {
            let before = moved(hello_box_world(), &["\r", "j"]);
            let (state, _) = reduce(before, key);
            assert_eq!(state.selected, [0, copy_index], "{key}");
            assert_eq!(state.selected.len(), 2);
            assert_eq!(state.boxes.parent(&state.selected), [0]);
            assert_eq!(state.boxes.children(&[0]).len(), 4);
        }
    }

    #[test]
    fn one_u_after_o_removes_the_copy_and_restores_the_selection() {
        let before = middle_selected();
        let state = moved(before.clone(), &["o", "\x1b", "u"]);
        assert_eq!(state.boxes, before.boxes);
        assert_eq!(state.selected, before.selected);
        assert_eq!(state.mode, FlexMode::Move);
    }

    #[test]
    fn one_u_after_capital_o_removes_the_copy_and_restores_the_selection() {
        let before = middle_selected();
        let state = moved(before.clone(), &["O", "\x1b", "u"]);
        assert_eq!(state.boxes, before.boxes);
        assert_eq!(state.selected, before.selected);
        assert_eq!(state.mode, FlexMode::Move);
    }

    #[test]
    fn starts_unfilled() {
        assert!(!selected_box(&FlexState::default()).filled);
    }

    fn hello_in(mode: FlexMode) -> FlexState {
        holding("Hello", mode)
    }

    #[test]
    fn ctrl_c_in_write_mode_quits_and_leaves_the_state_unchanged() {
        let before = hello_in(FlexMode::Write);
        assert_eq!(
            reduce(before.clone(), "\x03"),
            (before, Some(FlexEffect::Quit))
        );
    }

    #[test]
    fn ctrl_c_in_move_mode_quits_and_leaves_the_state_unchanged() {
        let before = hello_in(FlexMode::Move);
        assert_eq!(
            reduce(before.clone(), "\x03"),
            (before, Some(FlexEffect::Quit))
        );
    }

    fn typed(keys: &[&str]) -> (FlexState, Vec<FlexEffect>) {
        keys.iter().fold(
            (FlexState::default(), Vec::new()),
            |(state, mut effects), key| {
                let (state, effect) = reduce(state, key);
                effects.extend(effect);
                (state, effects)
            },
        )
    }

    #[test]
    fn typing_appends_each_character() {
        let (state, effects) = typed(&["i", "H", "e", "l", "l", "o"]);
        assert_eq!(text_of(&state, &state.selected), Some("Hello"));
        assert!(effects.is_empty());
    }

    #[test]
    fn typing_a_multibyte_character_appends_it() {
        let (state, effects) = typed(&["i", "c", "a", "f", "é"]);
        assert_eq!(text_of(&state, &state.selected), Some("café"));
        assert!(effects.is_empty());
    }

    #[test]
    fn backspace_removes_the_last_character() {
        let (state, effects) = typed(&["i", "H", "e", "l", "l", "p", "\x7f"]);
        assert_eq!(text_of(&state, &state.selected), Some("Hell"));
        assert!(effects.is_empty());
    }

    #[test]
    fn backspace_removes_a_whole_multibyte_character() {
        let (state, effects) = typed(&["i", "c", "a", "f", "é", "\x7f"]);
        assert_eq!(text_of(&state, &state.selected), Some("caf"));
        assert!(effects.is_empty());
    }

    #[test]
    fn backspace_on_an_empty_box_leaves_the_state_unchanged() {
        let (before, _) = typed(&["i"]);
        assert_eq!(reduce(before.clone(), "\x7f"), (before, None));
    }

    #[test]
    fn keys_that_are_not_a_single_printable_char_leave_the_state_unchanged() {
        let before = holding("Hi", FlexMode::Write);
        for key in ["\x1b[A", "\x01", "\t", "", "ab", tty::RESIZE] {
            assert_eq!(
                reduce(before.clone(), key),
                (before.clone(), None),
                "key {key:?}"
            );
        }
    }

    #[test]
    fn enter_on_non_empty_text_adds_a_sibling_and_stays_in_write() {
        let before = hello_in(FlexMode::Write);
        let (state, effect) = reduce(before.clone(), "\r");
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(state.selected, [1]);
        assert_eq!(text_of(&state, &[1]), Some(""));
        assert_eq!(text_of(&state, &[0]), Some("Hello"));
        assert_eq!(effect, None);
    }

    #[test]
    fn enter_on_non_empty_text_inserts_the_new_box_right_after_the_current_one() {
        let before = moved(
            stacked(&["Top", "Middle", "Bottom"], FlexMode::Move),
            &["k", "i"],
        );
        let (state, _) = reduce(before, "\r");
        assert_eq!(
            texts(&state),
            [Some("Top"), Some("Middle"), Some(""), Some("Bottom")]
        );
        assert_eq!(state.selected, [2]);
    }

    #[test]
    fn enter_on_non_empty_text_copies_the_boxs_fields_onto_the_new_sibling() {
        let before = moved(hello_in(FlexMode::Move), &["r", "f", "]", "i"]);
        let (state, _) = reduce(before.clone(), "\r");
        let original = box_at(&state, &[0]);
        let new_box = box_at(&state, &[1]);
        assert_eq!(new_box.direction, original.direction);
        assert_eq!(new_box.border, original.border);
        assert_eq!(new_box.filled, original.filled);
        assert_eq!(new_box.padding, original.padding);
        assert_eq!(new_box.justify, original.justify);
        assert_eq!(new_box.text, Some(String::new()));
    }

    #[test]
    fn line_feed_in_write_mode_leaves_the_state_unchanged() {
        let before = hello_in(FlexMode::Write);
        assert_eq!(reduce(before.clone(), "\n"), (before, None));
    }

    #[test]
    fn typing_in_move_mode_leaves_the_state_unchanged() {
        let before = hello_in(FlexMode::Move);
        assert_eq!(reduce(before.clone(), "x"), (before, None));
    }

    #[test]
    fn backspace_in_move_mode_leaves_the_state_unchanged() {
        let before = hello_in(FlexMode::Move);
        assert_eq!(reduce(before.clone(), "\x7f"), (before, None));
    }

    #[test]
    fn i_in_move_mode_switches_to_replace_and_keeps_the_text() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "i");
        assert_eq!(state.mode, FlexMode::Replace);
        assert_eq!(text_of(&state, &state.selected), Some("Hello"));
        assert_eq!(effect, None);
    }

    #[test]
    fn the_first_printable_key_in_replace_mode_replaces_the_whole_text() {
        let state = moved(hello_in(FlexMode::Move), &["i", "W"]);
        assert_eq!(text_of(&state, &state.selected), Some("W"));
        assert_eq!(state.mode, FlexMode::Write);
    }

    #[test]
    fn keys_after_the_first_replace_append() {
        let state = moved(hello_in(FlexMode::Move), &["i", "W", "o", "r", "l", "d"]);
        assert_eq!(text_of(&state, &state.selected), Some("World"));
    }

    #[test]
    fn a_multibyte_first_key_in_replace_mode_replaces_the_whole_text() {
        let state = moved(hello_in(FlexMode::Move), &["i", "é"]);
        assert_eq!(text_of(&state, &state.selected), Some("é"));
        assert_eq!(state.mode, FlexMode::Write);
    }

    #[test]
    fn backspace_as_the_first_key_in_replace_mode_empties_the_text() {
        let before = moved(hello_in(FlexMode::Move), &["i"]);
        let (state, effect) = reduce(before, "\x7f");
        assert_eq!(text_of(&state, &state.selected), Some(""));
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(effect, None);
    }

    #[test]
    fn enter_as_the_first_key_in_replace_mode_adds_an_empty_sibling() {
        let before = moved(hello_in(FlexMode::Move), &["i"]);
        let (state, effect) = reduce(before, "\r");
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(text_of(&state, &[0]), Some("Hello"));
        assert_eq!(text_of(&state, &[1]), Some(""));
        assert_eq!(state.selected, [1]);
        assert_eq!(effect, None);
    }

    #[test]
    fn undo_after_the_first_replace_restores_the_whole_original_label_in_one_step() {
        let state = moved(hello_in(FlexMode::Move), &["i", "W", "\x1b", "u"]);
        assert_eq!(text_of(&state, &state.selected), Some("Hello"));
    }

    #[test]
    fn undo_after_backspace_as_the_first_replace_key_restores_the_original_label() {
        let state = moved(hello_in(FlexMode::Move), &["i", "\x7f", "\x1b", "u"]);
        assert_eq!(text_of(&state, &state.selected), Some("Hello"));
    }

    #[test]
    fn i_on_a_box_with_empty_text_switches_to_write() {
        let (state, _) = reduce(holding("", FlexMode::Move), "i");
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(text_of(&state, &state.selected), Some(""));
    }

    #[test]
    fn o_and_the_enter_add_flow_never_produce_replace() {
        let (after_o, _) = reduce(hello_in(FlexMode::Move), "o");
        assert_eq!(after_o.mode, FlexMode::Write);
        let (after_enter, _) = reduce(moved(hello_in(FlexMode::Move), &["i"]), "\r");
        assert_eq!(after_enter.mode, FlexMode::Write);
    }

    #[test]
    fn i_on_a_box_with_no_text_adds_one_empty_text_and_switches_to_write() {
        let (state, effect) = reduce(FlexState::default(), "i");
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(text_of(&state, &state.selected), Some(""));
        assert_eq!(effect, None);
    }

    #[test]
    fn i_on_a_bordered_box_writes_into_its_own_text_and_creates_no_child() {
        let before = FlexState::default();
        let state = moved(before.clone(), &["i", "H", "i"]);
        assert!(selected_box(&state).border);
        assert_eq!(text_of(&state, &state.selected), Some("Hi"));
        assert!(state.boxes.children(&state.selected).is_empty());
        assert_eq!(state.selected, before.selected);
    }

    #[test]
    fn o_then_enter_keeps_the_empty_copied_box_and_returns_to_move() {
        let state = moved(FlexState::default(), &["o", "\r"]);
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(state.outer_boxes().count(), 2);
        assert_eq!(text_of(&state, &[1]), None);
        assert!(state.boxes.children(&[1]).is_empty());
    }

    #[test]
    fn o_then_enter_on_an_inner_box_leaves_an_empty_copy_next_to_it() {
        let before = moved(hello_box_world(), &["\r", "j"]);
        let state = moved(before, &["o", "\r"]);
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(state.selected, [0, 2]);
        assert_eq!(text_of(&state, &[0, 2]), None);
        assert!(box_at(&state, &[0, 2]).border);
    }

    #[test]
    fn i_then_enter_on_a_bordered_box_leaves_no_text() {
        let before = FlexState::default();
        let state = moved(before.clone(), &["i", "\r"]);
        assert_eq!(state, before);
        assert_eq!(selected_box(&state).text, None);
    }

    #[test]
    fn enter_on_empty_text_that_has_children_keeps_the_box_without_text() {
        let nested = FlexState {
            boxes: new_window(vec![Tree::new(
                FlexBox::default(),
                vec![Tree::new(
                    FlexBox {
                        text: Some("x".to_string()),
                        border: false,
                        ..FlexBox::default()
                    },
                    vec![new_box(), new_box()],
                )],
            )]),
            selected: vec![0, 0],
            ..FlexState::default()
        };
        let state = moved(nested, &["i", "\x7f", "\r"]);
        assert_eq!(text_of(&state, &state.selected), None);
        assert_eq!(state.boxes.children(&state.selected).len(), 2);
    }

    #[test]
    fn enter_on_an_empty_borderless_copy_drops_it_and_returns_to_move() {
        let before = world_selected();
        let (state, effect) = reduce(moved(before.clone(), &["o"]), "\r");
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(state.boxes, before.boxes);
        assert_eq!(state.selected, [0]);
        assert_eq!(effect, None);
    }

    #[test]
    fn esc_with_non_empty_text_returns_to_move_and_adds_no_sibling() {
        let before = moved(middle_selected(), &["i", "!", "?"]);
        assert_eq!(text_of(&before, &before.selected), Some("!?"));
        let (state, effect) = reduce(before.clone(), "\x1b");
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(texts(&state), [Some("Top"), Some("!?"), Some("Bottom")]);
        assert_eq!(state.outer_boxes().count(), before.outer_boxes().count());
        assert_eq!(state.selected, before.selected);
        assert_eq!(effect, None);
    }

    #[test]
    fn esc_with_empty_text_leaves_the_same_state_as_enter() {
        let hello = hello_in(FlexMode::Move);
        let before = moved(hello.clone(), &["o"]);
        assert_eq!(text_of(&before, &before.selected), Some(""));
        let escaped = reduce(before.clone(), "\x1b");
        assert_eq!(escaped, reduce(before, "\r"));
        let (state, effect) = escaped;
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(state.outer_boxes().count(), 2);
        assert_eq!(selected_box(&state).text, None);
        assert_eq!(effect, None);
    }

    #[test]
    fn esc_with_empty_text_on_a_bordered_box_leaves_the_same_state_as_enter() {
        let before = moved(FlexState::default(), &["i"]);
        assert!(selected_box(&before).border);
        let escaped = reduce(before.clone(), "\x1b");
        assert_eq!(escaped, reduce(before, "\r"));
        let (state, effect) = escaped;
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(selected_box(&state).text, None);
        assert_eq!(effect, None);
    }

    #[test]
    fn esc_after_i_returns_to_move_and_keeps_the_typed_text() {
        let before = moved(hello_in(FlexMode::Move), &["i", "!", "?"]);
        let state = reduce(before, "\x1b").0;
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(texts(&state), [Some("!?")]);
        assert_eq!(state.boxes.children(&[0]).len(), 0);
    }

    #[test]
    fn esc_after_o_with_text_keeps_the_new_text_and_returns_to_move() {
        let before = moved(hello_in(FlexMode::Move), &["o", "W", "o", "r", "l", "d"]);
        let state = reduce(before.clone(), "\x1b").0;
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(state.selected, before.selected);
        assert_eq!(text_of(&state, &state.selected), Some("World"));
        assert_eq!(state.outer_boxes().count(), 2);
    }

    #[test]
    fn esc_in_the_sibling_enter_just_spawned_keeps_its_text_and_returns_to_move() {
        let before = moved(hello_in(FlexMode::Move), &["i", "!", "\r", "?"]);
        assert_eq!(text_of(&before, &before.selected), Some("?"));
        let state = reduce(before, "\x1b").0;
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(texts(&state), [Some("!"), Some("?")]);
        assert_eq!(state.selected, [1]);
    }

    #[test]
    fn o_adds_a_copy_after_and_selects_it() {
        let state = moved(FlexState::default(), &["o"]);
        assert_eq!(state.outer_boxes().count(), 2);
        assert_eq!(state.selected, [1]);
        assert!(selected_box(&state).border);
        assert!(state.boxes.children(&state.selected).is_empty());
    }

    #[test]
    fn capital_a_on_a_text_leaf_nests_a_bordered_box_inside_the_leaf() {
        let before = FlexState {
            boxes: new_window(vec![Tree::new(FlexBox::default(), vec![text("x")])]),
            selected: vec![0, 0],
            ..FlexState::default()
        };
        let state = moved(before.clone(), &["A"]);
        let children = state.boxes.children(&before.selected);
        assert_eq!(children.len(), 1);
        let nested = box_at(&state, &children[0]);
        assert!(nested.border);
        assert_eq!(nested.text, None);
        assert_eq!(text_of(&state, &before.selected), Some("x"));
    }

    #[test]
    fn typing_after_i_on_a_box_with_no_text_goes_into_the_new_text() {
        let state = moved(FlexState::default(), &["i", "H", "i"]);
        assert_eq!(text_of(&state, &state.selected), Some("Hi"));
    }

    #[test]
    fn q_in_move_mode_quits_and_leaves_the_state_unchanged() {
        let before = hello_in(FlexMode::Move);
        assert_eq!(
            reduce(before.clone(), "q"),
            (before, Some(FlexEffect::Quit))
        );
    }

    #[test]
    fn q_in_write_mode_is_typed_into_the_box() {
        let (state, effect) = reduce(hello_in(FlexMode::Write), "q");
        assert_eq!(text_of(&state, &state.selected), Some("Helloq"));
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(effect, None);
    }

    #[test]
    fn w_in_move_mode_leaves_the_boxes_and_selection_unchanged() {
        let before = hello_in(FlexMode::Move);
        let (state, effect) = reduce(before.clone(), "w");
        assert_eq!(state.boxes, before.boxes);
        assert_eq!(state.selected, before.selected);
        assert_eq!(effect, None);
    }

    #[test]
    fn f_in_move_mode_fills_the_box_and_keeps_the_text_and_mode() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "f");
        assert!(selected_box(&state).filled);
        assert_eq!(text_of(&state, &state.selected), Some("Hello"));
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn f_twice_in_move_mode_unfills_the_box() {
        let (state, _) = reduce(hello_in(FlexMode::Move), "f");
        let (state, effect) = reduce(state, "f");
        assert_eq!(state, hello_in(FlexMode::Move));
        assert_eq!(effect, None);
    }

    #[test]
    fn f_in_write_mode_is_typed_into_the_box_and_leaves_the_fill_alone() {
        let (state, effect) = reduce(hello_in(FlexMode::Write), "f");
        assert_eq!(text_of(&state, &state.selected), Some("Hellof"));
        assert!(!selected_box(&state).filled);
        assert_eq!(effect, None);
    }

    #[test]
    fn the_fill_survives_switching_back_to_write_mode() {
        let (state, effects) = typed(&["i", "\r", "f", "i"]);
        assert_eq!(state.mode, FlexMode::Write);
        assert!(selected_box(&state).filled);
        assert!(effects.is_empty());
    }

    fn stacked(texts: &[&str], mode: FlexMode) -> FlexState {
        FlexState {
            boxes: new_window(texts.iter().map(|text| box_with(text)).collect()),
            selected: vec![texts.len() - 1],
            mode,
            ..FlexState::default()
        }
    }

    fn texts(state: &FlexState) -> Vec<Option<&str>> {
        (0..state.outer_boxes().count())
            .map(|index| text_of(state, &[index]))
            .collect()
    }

    #[test]
    fn a_in_move_mode_adds_an_empty_box_below_and_stays_in_move() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "a");
        assert_eq!(texts(&state), [Some("Hello"), None]);
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn a_adds_an_outer_box_with_no_children() {
        let state = moved(hello_in(FlexMode::Move), &["a"]);
        assert_eq!(subtree(&state, 1), [(vec![1], &FlexBox::default())]);
    }

    #[test]
    fn a_second_a_in_move_mode_adds_a_third_box() {
        let (state, _) = reduce(hello_in(FlexMode::Move), "a");
        let (state, effect) = reduce(state, "a");
        assert_eq!(texts(&state), [Some("Hello"), None, None]);
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn a_in_write_mode_is_typed_into_the_box_and_adds_no_box() {
        let (state, effect) = reduce(hello_in(FlexMode::Write), "a");
        assert_eq!(texts(&state), [Some("Helloa")]);
        assert_eq!(effect, None);
    }

    #[test]
    fn typing_goes_into_the_last_box() {
        let state = ["i", "H", "i"]
            .iter()
            .fold(stacked(&["Hello", ""], FlexMode::Move), |state, key| {
                reduce(state, key).0
            });
        assert_eq!(texts(&state), [Some("Hello"), Some("Hi")]);
    }

    #[test]
    fn backspace_on_an_empty_last_box_keeps_the_box() {
        let before = stacked(&["Hello", ""], FlexMode::Write);
        assert_eq!(reduce(before.clone(), "\x7f"), (before, None));
    }

    #[test]
    fn starts_with_a_start_justify() {
        assert_eq!(selected_box(&FlexState::default()).justify, Justify::Start);
    }

    #[test]
    fn toggling_the_justify_flips_between_start_and_space_between() {
        assert_eq!(Justify::Start.toggle(), Justify::SpaceBetween);
        assert_eq!(Justify::SpaceBetween.toggle(), Justify::Start);
    }

    #[test]
    fn toggling_a_centred_justify_returns_to_start() {
        assert_eq!(Justify::Center.toggle(), Justify::Start);
    }

    #[test]
    fn the_window_is_a_borderless_top_aligned_column_with_no_text() {
        assert_eq!(
            FlexBox::window(),
            FlexBox {
                text: None,
                border: false,
                direction: Direction::Column,
                justify: Justify::Start,
                filled: false,
                padding: 0,
            }
        );
    }

    #[test]
    fn a_default_box_has_a_border_and_no_text() {
        let default = FlexBox::default();
        assert!(default.border);
        assert_eq!(default.text, None);
    }

    #[test]
    fn s_in_move_mode_spreads_the_texts_and_keeps_the_rest() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "s");
        assert_eq!(selected_box(&state).justify, Justify::SpaceBetween);
        assert_eq!(text_of(&state, &state.selected), Some("Hello"));
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn s_twice_in_move_mode_toggles_the_justify_back_to_start() {
        let (state, _) = reduce(hello_in(FlexMode::Move), "s");
        let (state, effect) = reduce(state, "s");
        assert_eq!(state, hello_in(FlexMode::Move));
        assert_eq!(effect, None);
    }

    #[test]
    fn s_in_write_mode_is_typed_into_the_box_and_leaves_the_justify_alone() {
        let (state, effect) = reduce(hello_in(FlexMode::Write), "s");
        assert_eq!(text_of(&state, &state.selected), Some("Hellos"));
        assert_eq!(selected_box(&state).justify, Justify::Start);
        assert_eq!(effect, None);
    }

    #[test]
    fn s_only_spreads_the_selected_box() {
        let (state, _) = reduce(stacked(&["Hello", "World"], FlexMode::Move), "s");
        assert_eq!(box_at(&state, &[0]).justify, Justify::Start);
        assert_eq!(box_at(&state, &[1]).justify, Justify::SpaceBetween);
    }

    #[test]
    fn starts_with_a_column_direction() {
        assert_eq!(
            selected_box(&FlexState::default()).direction,
            Direction::Column
        );
    }

    #[test]
    fn toggling_the_direction_flips_between_row_and_column() {
        assert_eq!(Direction::Row.toggle(), Direction::Column);
        assert_eq!(Direction::Column.toggle(), Direction::Row);
    }

    #[test]
    fn r_in_move_mode_turns_the_box_into_a_row_and_keeps_the_rest() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "r");
        assert_eq!(selected_box(&state).direction, Direction::Row);
        assert_eq!(text_of(&state, &state.selected), Some("Hello"));
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn r_twice_in_move_mode_toggles_the_direction_back_to_row() {
        let (state, _) = reduce(hello_in(FlexMode::Move), "r");
        let (state, effect) = reduce(state, "r");
        assert_eq!(state, hello_in(FlexMode::Move));
        assert_eq!(effect, None);
    }

    #[test]
    fn r_in_write_mode_is_typed_into_the_box_and_leaves_the_direction_alone() {
        let (state, effect) = reduce(hello_in(FlexMode::Write), "r");
        assert_eq!(text_of(&state, &state.selected), Some("Hellor"));
        assert_eq!(selected_box(&state).direction, Direction::Column);
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(effect, None);
    }

    #[test]
    fn r_only_changes_the_direction_of_the_selected_box() {
        let (state, _) = reduce(stacked(&["Hello", "World"], FlexMode::Move), "r");
        assert_eq!(box_at(&state, &[0]).direction, Direction::Column);
        assert_eq!(box_at(&state, &[1]).direction, Direction::Row);
    }

    #[test]
    fn new_outer_and_inner_boxes_start_as_columns() {
        let state = moved(hello_in(FlexMode::Move), &["a", "A", "A"]);
        assert_eq!(box_at(&state, &[1]).direction, Direction::Column);
        assert_eq!(box_at(&state, &[1, 0]).direction, Direction::Column);
        assert_eq!(box_at(&state, &[1, 1]).direction, Direction::Column);
    }

    #[test]
    fn pressing_capital_a_twice_from_launch_stacks_two_inner_boxes_in_a_column() {
        let state = moved(FlexState::default(), &["A", "A"]);
        assert_eq!(selected_box(&state).direction, Direction::Column);
        assert_eq!(inner_boxes_of(&state, 0), 2);
    }

    fn moved(state: FlexState, keys: &[&str]) -> FlexState {
        keys.iter().fold(state, |state, key| reduce(state, key).0)
    }

    fn three_boxes_in_move() -> FlexState {
        stacked(&["Top", "Middle", "Bottom"], FlexMode::Move)
    }

    fn middle_selected() -> FlexState {
        moved(three_boxes_in_move(), &["k"])
    }

    #[test]
    fn starts_with_the_only_box_selected() {
        assert_eq!(FlexState::default().selected, [0]);
    }

    #[test]
    fn k_in_move_mode_selects_the_box_above() {
        let before = three_boxes_in_move();
        let (state, effect) = reduce(before.clone(), "k");
        assert_eq!(state.selected, [before.selected[0] - 1]);
        assert_eq!(state.boxes, before.boxes);
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn j_in_move_mode_selects_the_box_below() {
        let before = moved(three_boxes_in_move(), &["k", "k"]);
        let (state, effect) = reduce(before.clone(), "j");
        assert_eq!(state.selected, [before.selected[0] + 1]);
        assert_eq!(state.boxes, before.boxes);
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn j_on_the_bottom_box_leaves_the_state_unchanged() {
        let before = three_boxes_in_move();
        assert_eq!(before.selected, [before.outer_boxes().count() - 1]);
        assert_eq!(reduce(before.clone(), "j"), (before, None));
    }

    #[test]
    fn k_on_the_top_box_leaves_the_state_unchanged() {
        let before = moved(three_boxes_in_move(), &["k", "k"]);
        assert_eq!(before.selected, [0]);
        assert_eq!(reduce(before.clone(), "k"), (before, None));
    }

    #[test]
    fn a_selects_the_new_bottom_box() {
        let before = moved(three_boxes_in_move(), &["k", "k"]);
        let (state, _) = reduce(before, "a");
        assert_eq!(state.selected, [state.outer_boxes().count() - 1]);
        assert_eq!(
            texts(&state),
            [Some("Top"), Some("Middle"), Some("Bottom"), None]
        );
    }

    fn subtree(state: &FlexState, outer: usize) -> Vec<(Vec<usize>, &FlexBox)> {
        state
            .boxes
            .walk()
            .filter(|(path, _)| path[0] == outer)
            .collect()
    }

    fn only_the_middle_box_changed(before: &FlexState, after: &FlexState) {
        assert_eq!(after.outer_boxes().count(), before.outer_boxes().count());
        assert_ne!(subtree(after, 1), subtree(before, 1));
        assert_eq!(subtree(after, 0), subtree(before, 0));
        assert_eq!(subtree(after, 2), subtree(before, 2));
    }

    #[test]
    fn f_only_fills_the_selected_box() {
        let before = middle_selected();
        let (state, _) = reduce(before.clone(), "f");
        assert!(box_at(&state, &[1]).filled);
        only_the_middle_box_changed(&before, &state);
    }

    #[test]
    fn s_only_spreads_a_selected_box_above_the_bottom() {
        let before = middle_selected();
        let (state, _) = reduce(before.clone(), "s");
        assert_eq!(box_at(&state, &[1]).justify, Justify::SpaceBetween);
        only_the_middle_box_changed(&before, &state);
    }

    #[test]
    fn r_only_changes_a_selected_box_above_the_bottom() {
        let before = middle_selected();
        let (state, _) = reduce(before.clone(), "r");
        assert_eq!(box_at(&state, &[1]).direction, Direction::Row);
        only_the_middle_box_changed(&before, &state);
    }

    #[test]
    fn o_inserts_a_copy_after_a_selected_box_above_the_bottom() {
        let (state, _) = reduce(middle_selected(), "o");
        assert_eq!(
            texts(&state),
            [Some("Top"), Some("Middle"), Some(""), Some("Bottom")]
        );
        assert_eq!(state.selected, [2]);
    }

    #[test]
    fn typing_after_i_only_goes_into_the_selected_box() {
        let before = middle_selected();
        let state = moved(before.clone(), &["i", "!", "?", "\x7f"]);
        assert_eq!(text_of(&state, &[1]), Some("!"));
        only_the_middle_box_changed(&before, &state);
    }

    #[test]
    fn j_and_k_in_write_mode_are_typed_into_the_selected_text() {
        let before = moved(middle_selected(), &["i"]);
        let state = moved(before.clone(), &["j", "k"]);
        assert_eq!(text_of(&state, &[1]), Some("jk"));
        assert_eq!(state.selected, before.selected);
        assert_eq!(state.mode, FlexMode::Write);
        only_the_middle_box_changed(&before, &state);
    }

    fn inner_boxes_of(state: &FlexState, outer: usize) -> usize {
        state
            .boxes
            .walk()
            .filter(|(path, node)| path.len() == 2 && path[0] == outer && node.border)
            .count()
    }

    #[test]
    fn capital_a_in_move_mode_adds_an_inner_box_keeps_the_selection_and_stays_in_move() {
        let before = hello_in(FlexMode::Move);
        let (state, effect) = reduce(before.clone(), "A");
        assert_eq!(inner_boxes_of(&state, 0), 1);
        assert_eq!(state.selected, before.selected);
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn capital_a_adds_an_inner_box_with_no_children() {
        let state = moved(hello_in(FlexMode::Move), &["A"]);
        let hello_box = box_at(&hello_in(FlexMode::Move), &[0]).clone();
        assert_eq!(
            subtree(&state, 0),
            [(vec![0], &hello_box), (vec![0, 0], &FlexBox::default()),]
        );
    }

    #[test]
    fn a_second_capital_a_adds_a_second_inner_box_to_the_same_box() {
        let state = moved(hello_in(FlexMode::Move), &["A", "A"]);
        assert_eq!(inner_boxes_of(&state, 0), 2);
        assert_eq!(state.outer_boxes().count(), 1);
    }

    #[test]
    fn capital_a_only_adds_the_inner_box_to_the_selected_outer_box() {
        let state = moved(three_boxes_in_move(), &["k", "A"]);
        assert_eq!(inner_boxes_of(&state, 1), 1);
        assert_eq!(inner_boxes_of(&state, 0), 0);
        assert_eq!(inner_boxes_of(&state, 2), 0);
    }

    #[test]
    fn capital_a_in_write_mode_types_an_a_and_adds_no_inner_box() {
        let (state, _) = reduce(hello_in(FlexMode::Write), "A");
        assert_eq!(text_of(&state, &state.selected), Some("HelloA"));
        assert_eq!(inner_boxes_of(&state, 0), 0);
    }

    #[test]
    fn typing_after_capital_a_and_i_goes_into_the_selected_boxes_text() {
        let state = moved(hello_in(FlexMode::Move), &["A", "i", "H", "i"]);
        assert_eq!(text_of(&state, &state.selected), Some("Hi"));
        assert_eq!(state.outer_boxes().count(), 1);
    }

    #[test]
    fn j_and_k_after_capital_a_only_move_between_outer_boxes() {
        let state = moved(three_boxes_in_move(), &["A", "k", "A", "k", "j", "j", "j"]);
        assert_eq!(state.selected, [2]);
        let state = moved(state, &["k", "k", "k"]);
        assert_eq!(state.selected, [0]);
    }

    #[test]
    fn a_after_capital_a_still_adds_an_outer_box_at_the_bottom() {
        let state = moved(hello_in(FlexMode::Move), &["A", "a"]);
        assert_eq!(texts(&state), [Some("Hello"), None]);
        assert_eq!(state.selected, [1]);
        assert_eq!(inner_boxes_of(&state, 1), 0);
    }

    fn text(text: &str) -> Tree<FlexBox> {
        Tree::leaf(FlexBox {
            border: false,
            text: Some(text.to_string()),
            ..FlexBox::default()
        })
    }

    fn hello_box_world() -> FlexState {
        FlexState {
            boxes: new_window(vec![Tree::new(
                FlexBox::default(),
                vec![text("Hello"), new_box(), text("World")],
            )]),
            ..FlexState::default()
        }
    }

    fn selected_node(state: &FlexState) -> &FlexBox {
        state.boxes.value(&state.selected)
    }

    fn row_of(texts: &[&str]) -> FlexState {
        let parent = FlexBox {
            direction: Direction::Row,
            ..FlexBox::default()
        };
        FlexState {
            boxes: new_window(vec![Tree::new(
                parent,
                texts.iter().map(|label| text(label)).collect(),
            )]),
            selected: vec![0, 0],
            ..FlexState::default()
        }
    }

    #[test]
    fn l_on_a_text_leaf_leaves_the_state_unchanged() {
        let before = moved(hello_box_world(), &["\r"]);
        assert_eq!(reduce(before.clone(), "l"), (before, None));
    }

    #[test]
    fn l_on_an_empty_inner_box_leaves_the_state_unchanged() {
        let before = moved(hello_box_world(), &["\r", "j"]);
        assert_eq!(reduce(before.clone(), "l"), (before, None));
    }

    #[test]
    fn l_on_an_inner_box_holding_a_text_selects_that_text() {
        let mut inner = moved(hello_box_world(), &["\r", "j"]);
        inner.boxes.insert(&inner.selected, 0, text("x"));
        let mut leaf = inner.clone();
        leaf.selected = [inner.selected.as_slice(), &[0]].concat();
        leaf.mode = FlexMode::Move;
        let state = moved(leaf, &["\x7f", "\r"]);
        assert_eq!(text_of(&state, &state.selected), Some("x"));
        assert_eq!(state.selected, [inner.selected.as_slice(), &[0]].concat());
    }

    #[test]
    fn h_on_an_outer_box_leaves_the_state_unchanged() {
        let before = hello_box_world();
        assert_eq!(reduce(before.clone(), "h"), (before, None));
    }

    #[test]
    fn h_from_inside_an_inner_box_selects_the_inner_box() {
        let mut inner = moved(hello_box_world(), &["\r", "j"]);
        inner.boxes.insert(&inner.selected, 0, text("x"));
        let mut leaf = inner.clone();
        leaf.selected = [inner.selected.as_slice(), &[0]].concat();
        leaf.mode = FlexMode::Move;
        let state = moved(leaf, &["\x7f"]);
        assert_eq!(state.selected, inner.selected);
    }

    #[test]
    fn j_and_k_move_between_texts_and_boxes_and_stop_at_the_ends() {
        let first = moved(hello_box_world(), &["\r"]);
        let inner = moved(first.clone(), &["j"]);
        assert_eq!(selected_node(&inner), &FlexBox::default());
        let last = moved(inner.clone(), &["j"]);
        assert_eq!(text_of(&last, &last.selected), Some("World"));
        assert_eq!(moved(last.clone(), &["j"]), last);
        assert_eq!(moved(last, &["k"]), inner);
        assert_eq!(moved(inner, &["k"]), first);
        assert_eq!(moved(first.clone(), &["k"]), first);
    }

    #[test]
    fn j_and_k_move_between_siblings_under_a_column_parent() {
        let state = moved(hello_box_world(), &["\r"]);
        assert_eq!(state.selected, [0, 0]);
        let state = moved(state, &["j"]);
        assert_eq!(state.selected, [0, 1]);
        let state = moved(state, &["j"]);
        assert_eq!(state.selected, [0, 2]);
        let state = moved(state, &["k"]);
        assert_eq!(state.selected, [0, 1]);
    }

    #[test]
    fn h_and_l_move_between_siblings_under_a_row_parent() {
        let state = row_of(&["Left", "Middle", "Right"]);
        assert_eq!(state.selected, [0, 0]);
        let state = moved(state, &["l"]);
        assert_eq!(state.selected, [0, 1]);
        let state = moved(state, &["l"]);
        assert_eq!(state.selected, [0, 2]);
        let state = moved(state, &["h"]);
        assert_eq!(state.selected, [0, 1]);
    }

    #[test]
    fn j_and_k_do_nothing_under_a_row_parent() {
        let before = row_of(&["Left", "Middle", "Right"]);
        assert_eq!(reduce(before.clone(), "j"), (before.clone(), None));
        assert_eq!(reduce(before.clone(), "k"), (before, None));
    }

    #[test]
    fn h_and_l_do_nothing_under_a_column_parent() {
        let before = moved(hello_box_world(), &["\r"]);
        assert_eq!(reduce(before.clone(), "h"), (before.clone(), None));
        assert_eq!(reduce(before.clone(), "l"), (before, None));
    }

    #[test]
    fn top_level_boxes_walk_with_j_and_k() {
        let state = three_boxes_in_move();
        assert_eq!(state.selected, [2]);
        let state = moved(state, &["k", "k"]);
        assert_eq!(state.selected, [0]);
        let state = moved(state, &["j"]);
        assert_eq!(state.selected, [1]);
        let state = moved(state, &["j"]);
        assert_eq!(state.selected, [2]);
    }

    #[test]
    fn sibling_moves_stop_at_the_ends() {
        let first = moved(hello_box_world(), &["\r"]);
        assert_eq!(first.selected, [0, 0]);
        assert_eq!(reduce(first.clone(), "k"), (first, None));
        let last = moved(hello_box_world(), &["\r", "j", "j"]);
        assert_eq!(last.selected, [0, 2]);
        assert_eq!(reduce(last.clone(), "j"), (last, None));

        let row_first = row_of(&["Left", "Middle"]);
        assert_eq!(row_first.selected, [0, 0]);
        assert_eq!(reduce(row_first.clone(), "h"), (row_first, None));
        let row_last = moved(row_of(&["Left", "Middle"]), &["l"]);
        assert_eq!(row_last.selected, [0, 1]);
        assert_eq!(reduce(row_last.clone(), "l"), (row_last, None));
    }

    #[test]
    fn moving_the_selection_is_not_undoable() {
        let before = hello_box_world();
        let moved_state = moved(before, &["\r", "j", "j", "k", "l", "h"]);
        let (state, _) = reduce(moved_state.clone(), "u");
        assert_eq!(state, moved_state);
    }

    #[test]
    fn h_and_l_in_write_mode_are_typed_into_the_box() {
        let before = hello_in(FlexMode::Write);
        let state = moved(before.clone(), &["h", "l"]);
        assert_eq!(text_of(&state, &state.selected), Some("Hellohl"));
        assert_eq!(state.selected, before.selected);
        assert_eq!(state.mode, FlexMode::Write);
    }

    #[test]
    fn enter_selects_the_first_child_and_stays_on_a_childless_box() {
        let before = hello_box_world();
        let (child, effect) = reduce(before.clone(), "\r");
        assert_eq!(child.selected, [0, 0]);
        assert_eq!(child.boxes, before.boxes);
        assert_eq!(child.mode, FlexMode::Move);
        assert_eq!(effect, None);
        assert_eq!(reduce(child.clone(), "\r"), (child, None));
    }

    #[test]
    fn backspace_selects_the_parent_and_stays_on_a_top_level_box() {
        let child = moved(hello_box_world(), &["\r"]);
        assert_eq!(child.selected, [0, 0]);
        let (state, effect) = reduce(child, "\x7f");
        assert_eq!(state.selected, [0]);
        assert_eq!(effect, None);
        let top = hello_box_world();
        assert_eq!(reduce(top.clone(), "\x7f"), (top, None));
    }

    #[test]
    fn enter_and_backspace_type_themselves_in_write_mode() {
        let (state, _) = reduce(hello_in(FlexMode::Write), "\r");
        assert_eq!(texts(&state), [Some("Hello"), Some("")]);
        let (state, _) = reduce(hello_in(FlexMode::Write), "\x7f");
        assert_eq!(text_of(&state, &state.selected), Some("Hell"));
    }

    fn world_selected() -> FlexState {
        moved(hello_box_world(), &["\r", "j", "j"])
    }

    #[test]
    fn s_on_a_selected_text_spreads_that_text() {
        let state = moved(world_selected(), &["s"]);
        assert_eq!(box_at(&state, &[0, 2]).justify, Justify::SpaceBetween);
        assert_eq!(box_at(&state, &[0]).justify, Justify::Start);
    }

    #[test]
    fn f_on_a_selected_text_fills_that_text() {
        let state = moved(world_selected(), &["f"]);
        assert!(box_at(&state, &[0, 2]).filled);
        assert!(!box_at(&state, &[0]).filled);
    }

    #[test]
    fn r_on_a_selected_text_toggles_the_direction_of_that_text() {
        let state = moved(world_selected(), &["r"]);
        assert_eq!(box_at(&state, &[0, 2]).direction, Direction::Row);
    }

    #[test]
    fn capital_a_on_a_selected_text_nests_a_box_inside_the_text_and_keeps_the_selection() {
        let before = world_selected();
        let (state, effect) = reduce(before.clone(), "A");
        assert_eq!(state.boxes.children(&before.selected).len(), 1);
        assert_eq!(inner_boxes_of(&state, 0), inner_boxes_of(&before, 0));
        assert_eq!(state.selected, before.selected);
        assert_eq!(effect, None);
    }

    #[test]
    fn o_on_a_selected_text_inserts_a_copy_after_it_and_selects_it() {
        let before = moved(hello_box_world(), &["\r"]);
        let (state, effect) = reduce(before, "o");
        assert_eq!(state.selected, [0, 1]);
        assert_eq!(text_of(&state, &state.selected), Some(""));
        assert!(!selected_box(&state).border);
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(effect, None);
    }

    #[test]
    fn i_and_typing_on_a_selected_text_types_into_that_text() {
        let before = world_selected();
        let state = moved(before.clone(), &["i", "!"]);
        assert_eq!(text_of(&state, &before.selected), Some("!"));
        assert_eq!(state.selected, before.selected);
        assert_eq!(state.mode, FlexMode::Write);
    }

    #[test]
    fn backspace_on_a_selected_text_deletes_from_that_text() {
        let state = moved(world_selected(), &["i", "\x7f"]);
        assert_eq!(text_of(&state, &state.selected), Some(""));
    }

    #[test]
    fn o_on_a_selected_box_selects_the_new_copy() {
        let before = hello_in(FlexMode::Move);
        let (state, _) = reduce(before, "o");
        assert_eq!(state.selected, [1]);
        assert_eq!(text_of(&state, &state.selected), Some(""));
    }

    #[test]
    fn u_after_a_or_capital_a_brings_back_the_state_before_the_key() {
        for key in ["a", "A"] {
            let before = middle_selected();
            assert_eq!(moved(before.clone(), &[key, "u"]), before, "{key}");
        }
    }

    #[test]
    fn u_after_a_toggle_brings_back_the_state_before_the_key() {
        for key in ["s", "r", "f"] {
            let before = world_selected();
            assert_eq!(moved(before.clone(), &[key, "u"]), before, "{key}");
        }
    }

    #[test]
    fn u_goes_back_one_change_at_a_time_to_the_start() {
        let (state, _) = typed(&["a", "f", "u", "u"]);
        assert_eq!(state, FlexState::default());
        assert_eq!(moved(state.clone(), &["u"]), state);
    }

    #[test]
    fn u_skips_selection_moves_and_undoes_the_last_change() {
        let before = world_selected();
        let state = moved(before.clone(), &["f", "j", "k", "l", "u"]);
        assert_eq!(state.selected, before.selected);
        assert!(!box_at(&state, &state.selected).filled);
    }

    #[test]
    fn u_with_nothing_to_undo_leaves_the_state_unchanged() {
        let (state, effects) = typed(&["u"]);
        assert_eq!(state, FlexState::default());
        assert!(effects.is_empty());
    }

    #[test]
    fn u_after_o_typing_and_enter_is_typed_into_the_new_sibling_not_undone() {
        let before = hello_in(FlexMode::Move);
        let state = moved(before, &["o", "H", "e", "l", "l", "o", "\r", "u"]);
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(text_of(&state, &[0]), Some("Hello"));
        assert_eq!(text_of(&state, &[2]), Some("u"));
    }

    #[test]
    fn u_after_editing_and_enter_is_typed_into_the_new_sibling_not_undone() {
        let before = hello_in(FlexMode::Move);
        let state = moved(before, &["i", " ", "W", "o", "r", "l", "d", "\r", "u"]);
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(text_of(&state, &[0]), Some(" World"));
        assert_eq!(text_of(&state, &[1]), Some("u"));
    }

    #[test]
    fn u_after_i_typing_and_enter_on_non_empty_text_is_typed_into_the_new_sibling_not_undone() {
        let state = moved(FlexState::default(), &["i", "H", "i", "\r", "u"]);
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(text_of(&state, &state.selected), Some("u"));
    }

    #[test]
    fn u_in_write_mode_is_typed_and_leaves_the_history_alone() {
        let before = holding("Hello", FlexMode::Write);
        let (state, effect) = reduce(before.clone(), "u");
        assert_eq!(text_of(&state, &[0]), Some("Hellou"));
        assert_eq!(state.history.len(), before.history.len());
        assert_eq!(effect, None);
    }

    #[test]
    fn u_twice_undoes_an_accidental_empty_enter_then_the_sibling_creation_itself() {
        let before = hello_in(FlexMode::Move);

        let after_sibling = moved(before.clone(), &["i", "\r"]);
        assert_eq!(after_sibling.selected, [1]);
        assert_eq!(after_sibling.mode, FlexMode::Write);
        assert_eq!(text_of(&after_sibling, &[1]), Some(""));

        let after_accidental_enter = moved(after_sibling, &["\r"]);
        assert_eq!(after_accidental_enter.mode, FlexMode::Move);

        let once = moved(after_accidental_enter.clone(), &["u"]);
        assert_eq!(text_of(&once, &[1]), Some(""));
        assert_eq!(once.mode, FlexMode::Move);

        let state = moved(once, &["u"]);
        assert_eq!(state.boxes, before.boxes);
        assert_eq!(state.selected, before.selected);
        assert_eq!(state.mode, FlexMode::Move);
    }

    #[test]
    fn u_after_backspace_removes_an_empty_box_brings_it_and_its_selection_back() {
        let before = world_selected();
        let after_o = moved(before, &["o"]);
        let state = moved(after_o.clone(), &["\x7f", "u"]);
        assert_eq!(state.boxes, after_o.boxes);
        assert_eq!(state.selected, after_o.selected);
        assert_eq!(state.mode, FlexMode::Move);
    }

    #[test]
    fn close_bracket_on_a_selected_box_adds_one_unit_of_padding_each_time() {
        let before = hello_in(FlexMode::Move);
        let (once, effect) = reduce(before.clone(), "]");
        assert_eq!(
            selected_box(&once).padding,
            selected_box(&before).padding + 1
        );
        assert_eq!(once.mode, FlexMode::Move);
        assert_eq!(effect, None);
        let twice = moved(once.clone(), &["]"]);
        assert_eq!(
            selected_box(&twice).padding,
            selected_box(&once).padding + 1
        );
    }

    #[test]
    fn close_bracket_in_write_mode_is_typed_into_the_box_and_leaves_the_padding_at_zero() {
        let (state, _) = reduce(hello_in(FlexMode::Write), "]");
        assert_eq!(text_of(&state, &state.selected), Some("Hello]"));
        assert_eq!(selected_box(&state).padding, FlexBox::default().padding);
    }

    #[test]
    fn y_and_p_in_move_mode_leave_the_state_unchanged_and_return_no_effect() {
        for key in ["y", "p"] {
            let before = hello_in(FlexMode::Move);
            let (state, effect) = reduce(before.clone(), key);
            assert_eq!(state, before, "{key}");
            assert_eq!(effect, None, "{key}");
        }
    }

    #[test]
    fn d_cuts_the_selected_box_and_its_nested_contents() {
        let before = moved(hello_box_world(), &["\r", "j", "A"]);
        assert_eq!(before.selected, [0, 1]);
        let removed = before.boxes.value(&before.selected).clone();
        assert_eq!(before.boxes.children(&before.selected).len(), 1);

        let (state, effect) = reduce(before, "d");

        assert_eq!(
            state.boxes,
            new_window(vec![Tree::new(
                FlexBox::default(),
                vec![text("Hello"), text("World")],
            )])
        );
        assert_eq!(state.clipboard, Some(Tree::new(removed, vec![new_box()])));
        assert_eq!(effect, None);
    }

    #[test]
    fn d_selects_the_parent_after_a_nested_cut() {
        let before = moved(hello_box_world(), &["\r", "j"]);
        assert_eq!(before.selected, [0, 1]);

        let (state, _) = reduce(before, "d");

        assert_eq!(state.selected, [0]);
    }

    #[test]
    fn d_on_a_top_level_box_selects_the_canvas() {
        let before = hello_box_world();
        assert_eq!(before.selected, [0]);

        let (state, _) = reduce(before, "d");

        assert_eq!(state.selected, Vec::<usize>::new());
        assert_eq!(
            state.clipboard,
            Some(Tree::new(
                FlexBox::default(),
                vec![text("Hello"), new_box(), text("World")],
            ))
        );
    }

    #[test]
    fn d_on_the_only_box_leaves_an_empty_canvas() {
        let (state, _) = reduce(hello_box_world(), "d");

        assert!(state.boxes.children(&[]).is_empty());
        assert_eq!(state.selected, Vec::<usize>::new());
    }

    #[test]
    fn o_and_capital_o_on_an_empty_canvas_copy_the_window_as_a_top_level_box() {
        let (cut, _) = reduce(hello_box_world(), "d");
        assert_eq!(cut.selected, Vec::<usize>::new());
        assert!(cut.boxes.children(&[]).is_empty());

        let window = copied_box(&FlexBox::window());

        for key in ["o", "O"] {
            let (state, _) = reduce(cut.clone(), key);
            assert_eq!(state.boxes.children(&[]).len(), 1, "{key}");
            assert_eq!(state.boxes.value(&[0]), &window, "{key}");
            assert!(state.boxes.children(&[0]).is_empty(), "{key}");
            assert_eq!(state.selected, [0], "{key}");
            assert_eq!(state.mode, FlexMode::Write, "{key}");
        }
    }

    #[test]
    fn d_in_write_mode_is_typed_and_removes_nothing() {
        let before = hello_in(FlexMode::Write);

        let (state, _) = reduce(before.clone(), "d");

        assert_eq!(text_of(&state, &state.selected), Some("Hellod"));
        assert_eq!(state.boxes.walk().count(), before.boxes.walk().count());
        assert_eq!(state.selected, before.selected);
        assert_eq!(state.clipboard, None);
    }

    #[test]
    fn d_on_the_canvas_changes_nothing() {
        let before = FlexState {
            selected: Vec::new(),
            ..hello_box_world()
        };

        let (state, _) = reduce(before.clone(), "d");

        assert_eq!(state.boxes, before.boxes);
        assert_eq!(state.selected, before.selected);
        assert_eq!(state.clipboard, before.clipboard);
    }

    #[test]
    fn p_in_write_mode_is_typed_into_the_box_and_leaves_the_padding_alone() {
        let (state, _) = reduce(hello_in(FlexMode::Write), "p");
        assert_eq!(text_of(&state, &state.selected), Some("Hellop"));
        assert_eq!(selected_box(&state).padding, FlexBox::default().padding);
    }

    #[test]
    fn u_after_close_bracket_brings_back_the_boxes_before_the_key() {
        let before = hello_in(FlexMode::Move);
        let state = moved(before.clone(), &["]", "u"]);
        assert_eq!(state.boxes, before.boxes);
    }

    #[test]
    fn u_after_d_restores_the_box_and_its_contents_and_selection() {
        let before = moved(hello_box_world(), &["\r", "j", "A"]);
        let boxes = before.boxes.clone();
        let selected = before.selected.clone();

        let state = moved(before, &["d", "u"]);

        assert_eq!(state.boxes, boxes);
        assert_eq!(state.selected, selected);
        assert!(state.clipboard.is_some());
    }
}
