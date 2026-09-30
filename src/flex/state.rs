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
pub(crate) enum FlexWidth {
    #[default]
    Fit,
    Full,
}

impl FlexWidth {
    pub(crate) fn toggle(self) -> Self {
        match self {
            FlexWidth::Fit => FlexWidth::Full,
            FlexWidth::Full => FlexWidth::Fit,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Justify {
    #[default]
    Start,
    SpaceBetween,
}

impl Justify {
    pub(crate) fn toggle(self) -> Self {
        match self {
            Justify::Start => Justify::SpaceBetween,
            Justify::SpaceBetween => Justify::Start,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FlexBox {
    pub(crate) width: FlexWidth,
    pub(crate) justify: Justify,
    pub(crate) direction: Direction,
    pub(crate) filled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FlexNode {
    Text(String),
    Box(FlexBox),
}

impl Default for FlexNode {
    fn default() -> Self {
        FlexNode::Box(FlexBox::default())
    }
}

pub(crate) fn new_box() -> Tree<FlexNode> {
    Tree::new(
        FlexNode::Box(FlexBox::default()),
        vec![Tree::leaf(FlexNode::Text(String::new()))],
    )
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FlexState {
    pub(crate) boxes: Tree<FlexNode>,
    pub(crate) selected: Vec<usize>,
    pub(crate) mode: FlexMode,
}

impl Default for FlexState {
    fn default() -> Self {
        Self {
            boxes: Tree::root(vec![new_box()]),
            selected: vec![0],
            mode: FlexMode::Write,
        }
    }
}

impl FlexState {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn outer_boxes(&self) -> impl Iterator<Item = &FlexBox> {
        self.boxes
            .walk()
            .filter(|(path, _)| path.len() == 1)
            .filter_map(|(_, node)| match node {
                FlexNode::Box(flex_box) => Some(flex_box),
                FlexNode::Text(_) => None,
            })
    }

    pub(crate) fn children_of(
        &self,
        path: &[usize],
    ) -> impl Iterator<Item = (Vec<usize>, &FlexNode)> {
        let path = path.to_vec();
        (0..)
            .map(move |index| [path.as_slice(), &[index]].concat())
            .take_while(|child| self.boxes.contains(child))
            .map(|child| {
                let node = self.boxes.value(&child);
                (child, node)
            })
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn texts_of(&self, path: &[usize]) -> Vec<&str> {
        self.children_of(path)
            .filter_map(|(_, node)| match node {
                FlexNode::Text(text) => Some(text.as_str()),
                FlexNode::Box(_) => None,
            })
            .collect()
    }

    fn selected_box(&mut self) -> &mut FlexBox {
        let FlexNode::Box(flex_box) = self.boxes.value_mut(&self.selected) else {
            unreachable!("the selection is always a box")
        };
        flex_box
    }

    fn selected_text(&mut self) -> &mut String {
        let last_text = self
            .children_of(&self.selected)
            .filter(|(_, node)| matches!(node, FlexNode::Text(_)))
            .map(|(path, _)| path)
            .last()
            .expect("a flex box always holds at least one text");
        let FlexNode::Text(text) = self.boxes.value_mut(&last_text) else {
            unreachable!("the path was found as a text")
        };
        text
    }
}

pub(crate) fn reduce(state: FlexState, key: &str) -> (FlexState, Option<FlexEffect>) {
    match (key, state.mode) {
        ("\x03", _) => (state, Some(FlexEffect::Quit)),
        (_, FlexMode::Write) => write_key(state, key),
        (_, FlexMode::Move) => move_key(state, key),
    }
}

fn write_key(mut state: FlexState, key: &str) -> (FlexState, Option<FlexEffect>) {
    match key {
        "\r" => state.mode = FlexMode::Move,
        "\x7f" => {
            state.selected_text().pop();
        }
        _ => {
            if let Some(c) = printable_char(key) {
                state.selected_text().push(c);
            }
        }
    }
    (state, None)
}

fn move_key(mut state: FlexState, key: &str) -> (FlexState, Option<FlexEffect>) {
    match key {
        "i" => state.mode = FlexMode::Write,
        "a" => state.selected = state.boxes.push(&[], new_box()),
        "A" => {
            state.boxes.push(&state.selected, new_box());
        }
        "j" => state.selected = state.boxes.next(&state.selected),
        "k" => state.selected = state.boxes.previous(&state.selected),
        "s" => {
            state
                .boxes
                .push(&state.selected, Tree::leaf(FlexNode::Text(String::new())));
            state.mode = FlexMode::Write;
        }
        "w" => {
            let selected = state.selected_box();
            selected.width = selected.width.toggle();
        }
        "g" => {
            let selected = state.selected_box();
            selected.justify = selected.justify.toggle();
        }
        "d" => {
            let selected = state.selected_box();
            selected.direction = selected.direction.toggle();
        }
        "f" => {
            let selected = state.selected_box();
            selected.filled = !selected.filled;
        }
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
        let FlexNode::Box(flex_box) = state.boxes.value(path) else {
            panic!("no box at {path:?}")
        };
        flex_box
    }

    fn selected_box(state: &FlexState) -> &FlexBox {
        box_at(state, &state.selected)
    }

    fn box_with(text: &str) -> Tree<FlexNode> {
        Tree::new(
            FlexNode::Box(FlexBox::default()),
            vec![Tree::leaf(FlexNode::Text(text.to_string()))],
        )
    }

    fn holding(text: &str, mode: FlexMode) -> FlexState {
        FlexState {
            boxes: Tree::root(vec![box_with(text)]),
            mode,
            ..FlexState::default()
        }
    }

    #[test]
    fn starts_in_write_mode_with_exactly_one_empty_box() {
        let state = FlexState::default();
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(state.outer_boxes().count(), 1);
        assert_eq!(state.boxes, Tree::root(vec![new_box()]));
        assert_eq!(state.texts_of(&state.selected), [""]);
    }

    #[test]
    fn texts_of_returns_the_texts_of_the_box_at_the_path_in_order() {
        let state = moved(stacked(&["Hello", "World"], FlexMode::Move), &["s", "!"]);
        assert_eq!(state.texts_of(&[0]), ["Hello"]);
        assert_eq!(state.texts_of(&[1]), ["World", "!"]);
    }

    #[test]
    fn a_default_box_has_exactly_one_empty_text() {
        let boxes = Tree::root(vec![new_box()]);
        assert_eq!(
            boxes.walk().collect::<Vec<_>>(),
            [
                (vec![0], &FlexNode::Box(FlexBox::default())),
                (vec![0, 0], &FlexNode::Text(String::new())),
            ]
        );
    }

    #[test]
    fn s_in_move_mode_adds_an_empty_text_and_switches_to_write() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "s");
        assert_eq!(state.texts_of(&state.selected), ["Hello", ""]);
        assert_eq!(state.outer_boxes().count(), 1);
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(effect, None);
    }

    #[test]
    fn typing_after_s_goes_into_the_new_text() {
        let state = ["s", "W", "o", "r", "l", "d"]
            .iter()
            .fold(hello_in(FlexMode::Move), |state, key| reduce(state, key).0);
        assert_eq!(state.texts_of(&state.selected), ["Hello", "World"]);
    }

    #[test]
    fn a_second_s_after_enter_adds_a_third_text() {
        let state = ["s", "W", "o", "r", "l", "d", "\r", "s"]
            .iter()
            .fold(hello_in(FlexMode::Move), |state, key| reduce(state, key).0);
        assert_eq!(state.texts_of(&state.selected), ["Hello", "World", ""]);
        assert_eq!(state.mode, FlexMode::Write);
    }

    #[test]
    fn s_in_write_mode_is_typed_and_adds_no_text() {
        let (state, effect) = reduce(hello_in(FlexMode::Write), "s");
        assert_eq!(state.texts_of(&state.selected), ["Hellos"]);
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
        assert_eq!(state.texts_of(&state.selected), ["Hello", ""]);
        assert_eq!(reduce(state.clone(), "\x7f"), (state, None));
    }

    #[test]
    fn s_only_adds_a_text_to_the_selected_box() {
        let (state, _) = reduce(stacked(&["Hello", "World"], FlexMode::Move), "s");
        assert_eq!(state.texts_of(&[0]), ["Hello"]);
        assert_eq!(state.texts_of(&[1]), ["World", ""]);
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
        let (state, effects) = typed(&["H", "e", "l", "l", "o"]);
        assert_eq!(state.texts_of(&state.selected), ["Hello"]);
        assert!(effects.is_empty());
    }

    #[test]
    fn typing_a_multibyte_character_appends_it() {
        let (state, effects) = typed(&["c", "a", "f", "é"]);
        assert_eq!(state.texts_of(&state.selected), ["café"]);
        assert!(effects.is_empty());
    }

    #[test]
    fn backspace_removes_the_last_character() {
        let (state, effects) = typed(&["H", "e", "l", "l", "p", "\x7f"]);
        assert_eq!(state.texts_of(&state.selected), ["Hell"]);
        assert!(effects.is_empty());
    }

    #[test]
    fn backspace_removes_a_whole_multibyte_character() {
        let (state, effects) = typed(&["c", "a", "f", "é", "\x7f"]);
        assert_eq!(state.texts_of(&state.selected), ["caf"]);
        assert!(effects.is_empty());
    }

    #[test]
    fn backspace_on_an_empty_box_leaves_the_state_unchanged() {
        let before = FlexState::default();
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
        assert_eq!(state.texts_of(&state.selected), ["Hello"]);
        assert_eq!(effect, None);
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
        assert_eq!(state.texts_of(&state.selected), ["Helloq"]);
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(effect, None);
    }

    #[test]
    fn starts_with_a_fit_width() {
        assert_eq!(selected_box(&FlexState::default()).width, FlexWidth::Fit);
    }

    #[test]
    fn w_in_move_mode_toggles_the_width_to_full_and_keeps_the_rest() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "w");
        assert_eq!(selected_box(&state).width, FlexWidth::Full);
        assert_eq!(state.texts_of(&state.selected), ["Hello"]);
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn f_in_move_mode_fills_the_box_and_keeps_the_text_and_mode() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "f");
        assert!(selected_box(&state).filled);
        assert_eq!(state.texts_of(&state.selected), ["Hello"]);
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn w_twice_in_move_mode_toggles_the_width_back_to_fit() {
        let (state, _) = reduce(hello_in(FlexMode::Move), "w");
        let (state, effect) = reduce(state, "w");
        assert_eq!(state, hello_in(FlexMode::Move));
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
    fn w_in_write_mode_is_typed_into_the_box() {
        let (state, effect) = reduce(hello_in(FlexMode::Write), "w");
        assert_eq!(state.texts_of(&state.selected), ["Hellow"]);
        assert_eq!(selected_box(&state).width, FlexWidth::Fit);
        assert_eq!(effect, None);
    }

    #[test]
    fn f_in_write_mode_is_typed_into_the_box_and_leaves_the_fill_alone() {
        let (state, effect) = reduce(hello_in(FlexMode::Write), "f");
        assert_eq!(state.texts_of(&state.selected), ["Hellof"]);
        assert!(!selected_box(&state).filled);
        assert_eq!(effect, None);
    }

    #[test]
    fn the_width_carries_over_into_write_mode() {
        let (state, _) = reduce(hello_in(FlexMode::Move), "w");
        let (state, _) = reduce(state, "i");
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(selected_box(&state).width, FlexWidth::Full);
        let (state, _) = reduce(state, "x");
        assert_eq!(selected_box(&state).width, FlexWidth::Full);
    }

    #[test]
    fn toggling_the_width_flips_between_fit_and_full() {
        assert_eq!(FlexWidth::Fit.toggle(), FlexWidth::Full);
        assert_eq!(FlexWidth::Full.toggle(), FlexWidth::Fit);
    }

    #[test]
    fn the_fill_survives_switching_back_to_write_mode() {
        let (state, effects) = typed(&["\r", "f", "i"]);
        assert_eq!(state.mode, FlexMode::Write);
        assert!(selected_box(&state).filled);
        assert!(effects.is_empty());
    }

    fn stacked(texts: &[&str], mode: FlexMode) -> FlexState {
        FlexState {
            boxes: Tree::root(texts.iter().map(|text| box_with(text)).collect()),
            selected: vec![texts.len() - 1],
            mode,
        }
    }

    fn texts(state: &FlexState) -> Vec<&str> {
        (0..state.outer_boxes().count())
            .map(|index| state.texts_of(&[index])[0])
            .collect()
    }

    #[test]
    fn a_in_move_mode_adds_an_empty_box_below_and_stays_in_move() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "a");
        assert_eq!(texts(&state), ["Hello", ""]);
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn a_second_a_in_move_mode_adds_a_third_box() {
        let (state, _) = reduce(hello_in(FlexMode::Move), "a");
        let (state, effect) = reduce(state, "a");
        assert_eq!(texts(&state), ["Hello", "", ""]);
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn a_in_write_mode_is_typed_into_the_box_and_adds_no_box() {
        let (state, effect) = reduce(hello_in(FlexMode::Write), "a");
        assert_eq!(texts(&state), ["Helloa"]);
        assert_eq!(effect, None);
    }

    #[test]
    fn typing_goes_into_the_last_box() {
        let state = ["i", "H", "i"]
            .iter()
            .fold(stacked(&["Hello", ""], FlexMode::Move), |state, key| {
                reduce(state, key).0
            });
        assert_eq!(texts(&state), ["Hello", "Hi"]);
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
    fn g_in_move_mode_spreads_the_texts_and_keeps_the_rest() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "g");
        assert_eq!(selected_box(&state).justify, Justify::SpaceBetween);
        assert_eq!(state.texts_of(&state.selected), ["Hello"]);
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
        assert_eq!(state.texts_of(&state.selected), ["Hellog"]);
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
        assert_eq!(state.texts_of(&state.selected), ["Hello"]);
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
        assert_eq!(state.texts_of(&state.selected), ["Hellod"]);
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
        assert_eq!(box_at(&state, &[1, 1]).direction, Direction::Row);
        assert_eq!(box_at(&state, &[1, 2]).direction, Direction::Row);
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
        assert_eq!(texts(&state), ["Top", "Middle", "Bottom", ""]);
    }

    fn subtree(state: &FlexState, outer: usize) -> Vec<(Vec<usize>, &FlexNode)> {
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
    fn w_only_toggles_the_width_of_the_selected_box() {
        let before = middle_selected();
        let (state, _) = reduce(before.clone(), "w");
        assert_eq!(box_at(&state, &[1]).width, FlexWidth::Full);
        only_the_middle_box_changed(&before, &state);
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
    fn s_only_adds_a_text_to_a_selected_box_above_the_bottom() {
        let before = middle_selected();
        let (state, _) = reduce(before.clone(), "s");
        assert_eq!(state.texts_of(&[1]), ["Middle", ""]);
        only_the_middle_box_changed(&before, &state);
    }

    #[test]
    fn typing_after_i_only_goes_into_the_selected_box() {
        let before = middle_selected();
        let state = moved(before.clone(), &["i", "!", "?", "\x7f"]);
        assert_eq!(state.texts_of(&[1]), ["Middle!"]);
        only_the_middle_box_changed(&before, &state);
    }

    #[test]
    fn j_and_k_in_write_mode_are_typed_into_the_selected_text() {
        let before = moved(middle_selected(), &["i"]);
        let state = moved(before.clone(), &["j", "k"]);
        assert_eq!(state.texts_of(&[1]), ["Middlejk"]);
        assert_eq!(state.selected, before.selected);
        assert_eq!(state.mode, FlexMode::Write);
        only_the_middle_box_changed(&before, &state);
    }

    fn inner_boxes_of(state: &FlexState, outer: usize) -> usize {
        state
            .boxes
            .walk()
            .filter(|(path, node)| {
                path.len() == 2 && path[0] == outer && matches!(node, FlexNode::Box(_))
            })
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
        assert_eq!(state.texts_of(&state.selected), ["HelloA"]);
        assert_eq!(inner_boxes_of(&state, 0), 0);
    }

    #[test]
    fn typing_after_capital_a_and_i_goes_into_the_outer_boxes_last_text() {
        let state = moved(hello_in(FlexMode::Move), &["A", "i", "H", "i"]);
        assert_eq!(state.texts_of(&state.selected), ["HelloHi"]);
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
        assert_eq!(texts(&state), ["Hello", ""]);
        assert_eq!(state.selected, [1]);
        assert_eq!(inner_boxes_of(&state, 1), 0);
    }
}
