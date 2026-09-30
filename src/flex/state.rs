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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FlexBox {
    pub(crate) text: String,
    pub(crate) width: FlexWidth,
    pub(crate) filled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlexState {
    pub(crate) boxes: Vec<FlexBox>,
    pub(crate) mode: FlexMode,
}

impl Default for FlexState {
    fn default() -> Self {
        Self {
            boxes: vec![FlexBox::default()],
            mode: FlexMode::Write,
        }
    }
}

impl FlexState {
    fn newest_box(&mut self) -> &mut FlexBox {
        self.boxes
            .last_mut()
            .expect("a flex state always holds at least one box")
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
            state.newest_box().text.pop();
        }
        _ => {
            if let Some(c) = printable_char(key) {
                state.newest_box().text.push(c);
            }
        }
    }
    (state, None)
}

fn move_key(mut state: FlexState, key: &str) -> (FlexState, Option<FlexEffect>) {
    match key {
        "i" => state.mode = FlexMode::Write,
        "a" => state.boxes.push(FlexBox::default()),
        "w" => {
            let newest = state.newest_box();
            newest.width = newest.width.toggle();
        }
        "f" => {
            let newest = state.newest_box();
            newest.filled = !newest.filled;
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

    fn newest(state: &FlexState) -> &FlexBox {
        state.boxes.last().unwrap()
    }

    fn holding(text: &str, mode: FlexMode) -> FlexState {
        FlexState {
            boxes: vec![FlexBox {
                text: text.to_string(),
                ..FlexBox::default()
            }],
            mode,
        }
    }

    #[test]
    fn starts_in_write_mode_with_exactly_one_empty_box() {
        let state = FlexState::default();
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(state.boxes.len(), 1);
        assert_eq!(newest(&state).text, "");
    }

    #[test]
    fn starts_unfilled() {
        assert!(!newest(&FlexState::default()).filled);
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
        assert_eq!(newest(&state).text, "Hello");
        assert!(effects.is_empty());
    }

    #[test]
    fn typing_a_multibyte_character_appends_it() {
        let (state, effects) = typed(&["c", "a", "f", "é"]);
        assert_eq!(newest(&state).text, "café");
        assert!(effects.is_empty());
    }

    #[test]
    fn backspace_removes_the_last_character() {
        let (state, effects) = typed(&["H", "e", "l", "l", "p", "\x7f"]);
        assert_eq!(newest(&state).text, "Hell");
        assert!(effects.is_empty());
    }

    #[test]
    fn backspace_removes_a_whole_multibyte_character() {
        let (state, effects) = typed(&["c", "a", "f", "é", "\x7f"]);
        assert_eq!(newest(&state).text, "caf");
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
        assert_eq!(newest(&state).text, "Hello");
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
        assert_eq!(newest(&state).text, "Helloq");
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(effect, None);
    }

    #[test]
    fn starts_with_a_fit_width() {
        assert_eq!(newest(&FlexState::default()).width, FlexWidth::Fit);
    }

    #[test]
    fn w_in_move_mode_toggles_the_width_to_full_and_keeps_the_rest() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "w");
        assert_eq!(newest(&state).width, FlexWidth::Full);
        assert_eq!(newest(&state).text, "Hello");
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(effect, None);
    }

    #[test]
    fn f_in_move_mode_fills_the_box_and_keeps_the_text_and_mode() {
        let (state, effect) = reduce(hello_in(FlexMode::Move), "f");
        assert!(newest(&state).filled);
        assert_eq!(newest(&state).text, "Hello");
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
        assert_eq!(newest(&state).text, "Hellow");
        assert_eq!(newest(&state).width, FlexWidth::Fit);
        assert_eq!(effect, None);
    }

    #[test]
    fn f_in_write_mode_is_typed_into_the_box_and_leaves_the_fill_alone() {
        let (state, effect) = reduce(hello_in(FlexMode::Write), "f");
        assert_eq!(newest(&state).text, "Hellof");
        assert!(!newest(&state).filled);
        assert_eq!(effect, None);
    }

    #[test]
    fn the_width_carries_over_into_write_mode() {
        let (state, _) = reduce(hello_in(FlexMode::Move), "w");
        let (state, _) = reduce(state, "i");
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(newest(&state).width, FlexWidth::Full);
        let (state, _) = reduce(state, "x");
        assert_eq!(newest(&state).width, FlexWidth::Full);
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
        assert!(newest(&state).filled);
        assert!(effects.is_empty());
    }

    fn stacked(texts: &[&str], mode: FlexMode) -> FlexState {
        FlexState {
            boxes: texts
                .iter()
                .map(|text| FlexBox {
                    text: text.to_string(),
                    ..FlexBox::default()
                })
                .collect(),
            mode,
        }
    }

    fn texts(state: &FlexState) -> Vec<&str> {
        state.boxes.iter().map(|b| b.text.as_str()).collect()
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
}
