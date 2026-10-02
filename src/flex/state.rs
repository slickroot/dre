use super::history::{self, Snapshot};
use types::Tree;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlexMode {
    Write,
    Move,
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
    #[default]
    Row,
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
            direction: Direction::Column,
            justify: Justify::Center,
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

pub(crate) fn new_text() -> Tree<FlexBox> {
    Tree::leaf(FlexBox {
        border: false,
        text: Some(String::new()),
        ..FlexBox::default()
    })
}

#[derive(Debug, Clone)]
pub(crate) struct FlexState {
    pub(crate) boxes: Tree<FlexBox>,
    pub(crate) selected: Vec<usize>,
    pub(crate) mode: FlexMode,
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
        (_, FlexMode::Write) => write_key(state, key),
        (_, FlexMode::Move) => history::recorded(state, key, |s| move_key(s, key)),
    }
}

fn drop_empty_text(mut state: FlexState) -> FlexState {
    let borderless = !state.selected_mut().border;
    if borderless && state.boxes.children(&state.selected).is_empty() {
        let parent = state.boxes.parent(&state.selected);
        state.boxes.remove(&state.selected);
        state.selected = parent;
    } else {
        state.selected_mut().text = None;
    }
    state
}

fn write_key(mut state: FlexState, key: &str) -> (FlexState, Option<FlexEffect>) {
    match key {
        "\r" => {
            if state.selected_mut().text.as_deref() == Some("") {
                state = drop_empty_text(state);
            }
            state.mode = FlexMode::Move;
        }
        "\x7f" => {
            if let Some(text) = state.selected_mut().text.as_mut() {
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

fn move_key(mut state: FlexState, key: &str) -> (FlexState, Option<FlexEffect>) {
    match key {
        "i" => {
            state.selected_mut().text.get_or_insert_with(String::new);
            state.mode = FlexMode::Write;
        }
        "a" => state.selected = state.boxes.push(&[], new_box()),
        "A" => {
            state.boxes.push(&state.selected, new_box());
        }
        "j" => state.selected = state.boxes.next(&state.selected),
        "k" => state.selected = state.boxes.previous(&state.selected),
        "l" => state.selected = state.boxes.child(&state.selected),
        "h" => state.selected = state.boxes.parent(&state.selected),
        "s" => {
            state.selected = state.boxes.push(&state.selected, new_text());
            state.mode = FlexMode::Write;
        }
        "g" => {
            let selected = state.selected_mut();
            selected.justify = selected.justify.toggle();
        }
        "d" => {
            let selected = state.selected_mut();
            selected.direction = selected.direction.toggle();
        }
        "f" => {
            let selected = state.selected_mut();
            selected.filled = !selected.filled;
        }
        "p" => {
            let selected = state.selected_mut();
            selected.padding = selected.padding.saturating_add(1);
        }
        "u" => state = history::undo(state),
        "q" => return (state, Some(FlexEffect::Quit)),
        _ => {}
    }
    (state, None)
}

fn printable_char(key: &str) -> Option<char> {
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
    fn s_in_move_mode_adds_an_empty_text_leaf_and_switches_to_write() {
        let before = hello_in(FlexMode::Move);
        let (state, effect) = reduce(before.clone(), "s");
        assert_eq!(state.selected, [before.selected.as_slice(), &[0]].concat());
        assert_eq!(text_of(&state, &state.selected), Some(""));
        assert!(!box_at(&state, &state.selected).border);
        assert_eq!(
            text_of(&state, &before.selected),
            text_of(&before, &before.selected)
        );
        assert_eq!(state.outer_boxes().count(), 1);
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(effect, None);
    }

    #[test]
    fn typing_after_s_goes_into_the_new_text() {
        let state = ["s", "W", "o", "r", "l", "d"]
            .iter()
            .fold(hello_in(FlexMode::Move), |state, key| reduce(state, key).0);
        assert_eq!(text_of(&state, &[0, 0]), Some("World"));
        assert_eq!(text_of(&state, &[0]), Some("Hello"));
    }

    #[test]
    fn a_second_s_after_enter_nests_a_text_inside_the_text_leaf() {
        let state = ["s", "W", "o", "r", "l", "d", "\r", "s"]
            .iter()
            .fold(hello_in(FlexMode::Move), |state, key| reduce(state, key).0);
        assert_eq!(state.selected, [0, 0, 0]);
        assert_eq!(text_of(&state, &state.selected), Some(""));
        assert_eq!(state.mode, FlexMode::Write);
    }

    #[test]
    fn s_in_write_mode_is_typed_and_adds_no_text() {
        let (state, effect) = reduce(hello_in(FlexMode::Write), "s");
        assert_eq!(text_of(&state, &state.selected), Some("Hellos"));
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(effect, None);
    }

    #[test]
    fn backspace_on_an_empty_new_text_leaves_the_state_unchanged() {
        let (state, _) = reduce(hello_in(FlexMode::Move), "s");
        assert_eq!(reduce(state.clone(), "\x7f"), (state, None));
    }

    #[test]
    fn backspace_empties_the_new_text_and_then_changes_nothing() {
        let (state, _) = reduce(hello_in(FlexMode::Move), "s");
        let (state, _) = reduce(state, "W");
        let (state, _) = reduce(state, "\x7f");
        assert_eq!(text_of(&state, &state.selected), Some(""));
        assert_eq!(reduce(state.clone(), "\x7f"), (state, None));
    }

    #[test]
    fn s_only_adds_a_text_leaf_to_the_selected_box() {
        let (state, _) = reduce(stacked(&["Hello", "World"], FlexMode::Move), "s");
        assert_eq!(state.boxes.children(&[0]).len(), 0);
        assert_eq!(state.boxes.children(&[1]).len(), 1);
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
        for key in ["\x1b[A", "\x01", "\x1b", "\t", "", "ab", tty::RESIZE] {
            assert_eq!(
                reduce(before.clone(), key),
                (before.clone(), None),
                "key {key:?}"
            );
        }
    }

    #[test]
    fn enter_in_write_mode_switches_to_move_and_keeps_the_text() {
        let (state, effect) = reduce(hello_in(FlexMode::Write), "\r");
        assert_eq!(state, hello_in(FlexMode::Move));
        assert_eq!(effect, None);
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
    fn i_in_move_mode_switches_to_write_and_keeps_the_text() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "i");
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(text_of(&state, &state.selected), Some("Hello"));
        assert_eq!(effect, None);
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
    fn s_then_enter_leaves_the_tree_as_it_was_with_the_parent_selected() {
        let before = FlexState::default();
        let state = moved(before.clone(), &["s", "\r"]);
        assert_eq!(state, before);
    }

    #[test]
    fn s_then_enter_on_an_inner_box_selects_that_box() {
        let before = moved(hello_box_world(), &["l", "j"]);
        let state = moved(before.clone(), &["s", "\r"]);
        assert_eq!(state, before);
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
        let nested = moved(FlexState::default(), &["s", "x", "\r", "A", "A"]);
        let state = moved(nested, &["i", "\x7f", "\r"]);
        assert_eq!(text_of(&state, &state.selected), None);
        assert_eq!(state.boxes.children(&state.selected).len(), 2);
    }

    #[test]
    fn s_adds_a_borderless_leaf_and_selects_it() {
        let before = FlexState::default();
        let state = moved(before.clone(), &["s"]);
        assert_eq!(state.boxes.children(&before.selected).len(), 1);
        assert_eq!(state.boxes.parent(&state.selected), before.selected);
        assert!(!selected_box(&state).border);
        assert!(state.boxes.children(&state.selected).is_empty());
    }

    #[test]
    fn capital_a_on_a_text_leaf_nests_a_bordered_box_inside_the_leaf() {
        let before = moved(FlexState::default(), &["s", "x", "\r"]);
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
    fn the_window_is_a_borderless_centred_column_with_no_text() {
        assert_eq!(
            FlexBox::window(),
            FlexBox {
                text: None,
                border: false,
                direction: Direction::Column,
                justify: Justify::Center,
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
    fn a_new_text_is_a_borderless_leaf_with_empty_text() {
        let text = new_text();
        assert!(!text.value(&[]).border);
        assert_eq!(text.value(&[]).text.as_deref(), Some(""));
        assert_eq!(text.children(&[]).len(), 0);
    }

    #[test]
    fn g_in_move_mode_spreads_the_texts_and_keeps_the_rest() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "g");
        assert_eq!(selected_box(&state).justify, Justify::SpaceBetween);
        assert_eq!(text_of(&state, &state.selected), Some("Hello"));
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn g_twice_in_move_mode_toggles_the_justify_back_to_start() {
        let (state, _) = reduce(hello_in(FlexMode::Move), "g");
        let (state, effect) = reduce(state, "g");
        assert_eq!(state, hello_in(FlexMode::Move));
        assert_eq!(effect, None);
    }

    #[test]
    fn g_in_write_mode_is_typed_into_the_box_and_leaves_the_justify_alone() {
        let (state, effect) = reduce(hello_in(FlexMode::Write), "g");
        assert_eq!(text_of(&state, &state.selected), Some("Hellog"));
        assert_eq!(selected_box(&state).justify, Justify::Start);
        assert_eq!(effect, None);
    }

    #[test]
    fn g_only_spreads_the_selected_box() {
        let (state, _) = reduce(stacked(&["Hello", "World"], FlexMode::Move), "g");
        assert_eq!(box_at(&state, &[0]).justify, Justify::Start);
        assert_eq!(box_at(&state, &[1]).justify, Justify::SpaceBetween);
    }

    #[test]
    fn starts_with_a_row_direction() {
        assert_eq!(
            selected_box(&FlexState::default()).direction,
            Direction::Row
        );
    }

    #[test]
    fn toggling_the_direction_flips_between_row_and_column() {
        assert_eq!(Direction::Row.toggle(), Direction::Column);
        assert_eq!(Direction::Column.toggle(), Direction::Row);
    }

    #[test]
    fn d_in_move_mode_turns_the_box_into_a_column_and_keeps_the_rest() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "d");
        assert_eq!(selected_box(&state).direction, Direction::Column);
        assert_eq!(text_of(&state, &state.selected), Some("Hello"));
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn d_twice_in_move_mode_toggles_the_direction_back_to_row() {
        let (state, _) = reduce(hello_in(FlexMode::Move), "d");
        let (state, effect) = reduce(state, "d");
        assert_eq!(state, hello_in(FlexMode::Move));
        assert_eq!(effect, None);
    }

    #[test]
    fn d_in_write_mode_is_typed_into_the_box_and_leaves_the_direction_alone() {
        let (state, effect) = reduce(hello_in(FlexMode::Write), "d");
        assert_eq!(text_of(&state, &state.selected), Some("Hellod"));
        assert_eq!(selected_box(&state).direction, Direction::Row);
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(effect, None);
    }

    #[test]
    fn d_only_changes_the_direction_of_the_selected_box() {
        let (state, _) = reduce(stacked(&["Hello", "World"], FlexMode::Move), "d");
        assert_eq!(box_at(&state, &[0]).direction, Direction::Row);
        assert_eq!(box_at(&state, &[1]).direction, Direction::Column);
    }

    #[test]
    fn new_outer_and_inner_boxes_start_as_rows() {
        let state = moved(hello_in(FlexMode::Move), &["d", "a", "A", "A"]);
        assert_eq!(box_at(&state, &[1]).direction, Direction::Row);
        assert_eq!(box_at(&state, &[1, 0]).direction, Direction::Row);
        assert_eq!(box_at(&state, &[1, 1]).direction, Direction::Row);
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
    fn g_only_spreads_a_selected_box_above_the_bottom() {
        let before = middle_selected();
        let (state, _) = reduce(before.clone(), "g");
        assert_eq!(box_at(&state, &[1]).justify, Justify::SpaceBetween);
        only_the_middle_box_changed(&before, &state);
    }

    #[test]
    fn d_only_changes_a_selected_box_above_the_bottom() {
        let before = middle_selected();
        let (state, _) = reduce(before.clone(), "d");
        assert_eq!(box_at(&state, &[1]).direction, Direction::Column);
        only_the_middle_box_changed(&before, &state);
    }

    #[test]
    fn s_only_adds_a_text_leaf_to_a_selected_box_above_the_bottom() {
        let before = middle_selected();
        let (state, _) = reduce(before.clone(), "s");
        assert_eq!(state.boxes.children(&[1]).len(), 1);
        only_the_middle_box_changed(&before, &state);
    }

    #[test]
    fn typing_after_i_only_goes_into_the_selected_box() {
        let before = middle_selected();
        let state = moved(before.clone(), &["i", "!", "?", "\x7f"]);
        assert_eq!(text_of(&state, &[1]), Some("Middle!"));
        only_the_middle_box_changed(&before, &state);
    }

    #[test]
    fn j_and_k_in_write_mode_are_typed_into_the_selected_text() {
        let before = moved(middle_selected(), &["i"]);
        let state = moved(before.clone(), &["j", "k"]);
        assert_eq!(text_of(&state, &[1]), Some("Middlejk"));
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
        assert_eq!(text_of(&state, &state.selected), Some("HelloHi"));
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

    #[test]
    fn l_in_move_mode_selects_the_first_child() {
        let before = hello_box_world();
        let (state, effect) = reduce(before.clone(), "l");
        assert_eq!(text_of(&state, &state.selected), Some("Hello"));
        assert_eq!(state.boxes, before.boxes);
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn l_on_a_text_leaf_leaves_the_state_unchanged() {
        let before = moved(hello_box_world(), &["l"]);
        assert_eq!(reduce(before.clone(), "l"), (before, None));
    }

    #[test]
    fn l_on_an_empty_inner_box_leaves_the_state_unchanged() {
        let before = moved(hello_box_world(), &["l", "j"]);
        assert_eq!(reduce(before.clone(), "l"), (before, None));
    }

    #[test]
    fn l_on_an_inner_box_holding_a_text_selects_that_text() {
        let inner = moved(hello_box_world(), &["l", "j"]);
        let state = moved(inner.clone(), &["s", "x", "\r", "h", "l"]);
        assert_eq!(text_of(&state, &state.selected), Some("x"));
        assert_eq!(state.selected, [inner.selected.as_slice(), &[0]].concat());
    }

    #[test]
    fn h_in_move_mode_selects_the_parent() {
        let before = hello_box_world();
        let (state, effect) = reduce(moved(before.clone(), &["l", "j", "j"]), "h");
        assert_eq!(state, before);
        assert_eq!(effect, None);
    }

    #[test]
    fn h_on_an_outer_box_leaves_the_state_unchanged() {
        let before = hello_box_world();
        assert_eq!(reduce(before.clone(), "h"), (before, None));
    }

    #[test]
    fn h_from_inside_an_inner_box_selects_the_inner_box() {
        let inner = moved(hello_box_world(), &["l", "j"]);
        let state = moved(inner.clone(), &["s", "x", "\r", "h"]);
        assert_eq!(state.selected, inner.selected);
    }

    #[test]
    fn j_and_k_move_between_texts_and_boxes_and_stop_at_the_ends() {
        let first = moved(hello_box_world(), &["l"]);
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
    fn h_and_l_in_write_mode_are_typed_into_the_box() {
        let before = hello_in(FlexMode::Write);
        let state = moved(before.clone(), &["h", "l"]);
        assert_eq!(text_of(&state, &state.selected), Some("Hellohl"));
        assert_eq!(state.selected, before.selected);
        assert_eq!(state.mode, FlexMode::Write);
    }

    fn world_selected() -> FlexState {
        moved(hello_box_world(), &["l", "j", "j"])
    }

    #[test]
    fn g_on_a_selected_text_spreads_that_text() {
        let state = moved(world_selected(), &["g"]);
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
    fn d_on_a_selected_text_toggles_the_direction_of_that_text() {
        let state = moved(world_selected(), &["d"]);
        assert_eq!(box_at(&state, &[0, 2]).direction, Direction::Column);
        assert_eq!(box_at(&state, &[0]).direction, Direction::Row);
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
    fn s_on_a_selected_text_nests_an_empty_text_inside_it_and_selects_it() {
        let before = moved(hello_box_world(), &["l"]);
        let (state, effect) = reduce(before.clone(), "s");
        assert_eq!(state.boxes.parent(&state.selected), before.selected);
        assert_eq!(text_of(&state, &state.selected), Some(""));
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(effect, None);
    }

    #[test]
    fn i_and_typing_on_a_selected_text_types_into_that_text() {
        let before = world_selected();
        let state = moved(before.clone(), &["i", "!"]);
        assert_eq!(text_of(&state, &before.selected), Some("World!"));
        assert_eq!(state.selected, before.selected);
        assert_eq!(state.mode, FlexMode::Write);
    }

    #[test]
    fn backspace_on_a_selected_text_deletes_from_that_text() {
        let state = moved(world_selected(), &["i", "\x7f"]);
        assert_eq!(text_of(&state, &state.selected), Some("Worl"));
    }

    #[test]
    fn s_on_a_selected_box_selects_the_new_text() {
        let before = hello_in(FlexMode::Move);
        let (state, _) = reduce(before.clone(), "s");
        assert_eq!(state.boxes.parent(&state.selected), before.selected);
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
        for key in ["g", "d", "f"] {
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
    fn u_after_s_and_typing_removes_the_whole_text_and_restores_the_selection() {
        let before = hello_in(FlexMode::Move);
        let state = moved(before.clone(), &["s", "H", "e", "l", "l", "o", "\r", "u"]);
        assert_eq!(state, before);
    }

    #[test]
    fn u_after_editing_a_text_brings_back_the_text_before_the_edit() {
        let before = hello_in(FlexMode::Move);
        let state = moved(
            before.clone(),
            &["i", " ", "W", "o", "r", "l", "d", "\r", "u"],
        );
        assert_eq!(text_of(&state, &[0]), Some("Hello"));
    }

    #[test]
    fn u_after_i_and_typing_on_an_empty_box_removes_the_text() {
        let state = moved(FlexState::default(), &["i", "H", "i", "\r", "u"]);
        assert_eq!(text_of(&state, &state.selected), None);
    }

    #[test]
    fn u_in_write_mode_is_typed_and_leaves_the_history_alone() {
        let before = moved(hello_in(FlexMode::Move), &["i"]);
        let (state, effect) = reduce(before.clone(), "u");
        assert_eq!(text_of(&state, &[0]), Some("Hellou"));
        assert_eq!(state.history.len(), before.history.len());
        assert_eq!(effect, None);
    }

    #[test]
    fn p_on_a_selected_box_adds_one_unit_of_padding_each_time() {
        let before = hello_in(FlexMode::Move);
        let (once, effect) = reduce(before.clone(), "p");
        assert_eq!(
            selected_box(&once).padding,
            selected_box(&before).padding + 1
        );
        assert_eq!(once.mode, FlexMode::Move);
        assert_eq!(effect, None);
        let twice = moved(once.clone(), &["p"]);
        assert_eq!(
            selected_box(&twice).padding,
            selected_box(&once).padding + 1
        );
    }

    #[test]
    fn p_in_write_mode_is_typed_into_the_box_and_leaves_the_padding_alone() {
        let (state, _) = reduce(hello_in(FlexMode::Write), "p");
        assert_eq!(text_of(&state, &state.selected), Some("Hellop"));
        assert_eq!(selected_box(&state).padding, FlexBox::default().padding);
    }

    #[test]
    fn u_after_p_brings_back_the_boxes_before_the_key() {
        let before = hello_in(FlexMode::Move);
        let state = moved(before.clone(), &["p", "u"]);
        assert_eq!(state.boxes, before.boxes);
    }
}
