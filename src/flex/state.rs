#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlexMode {
    Write,
    Move,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlexState {
    pub(crate) text: String,
    pub(crate) mode: FlexMode,
    pub(crate) running: bool,
}

impl Default for FlexState {
    fn default() -> Self {
        Self {
            text: String::new(),
            mode: FlexMode::Write,
            running: true,
        }
    }
}

pub(crate) fn reduce(state: FlexState, key: &str) -> FlexState {
    match (key, state.mode) {
        ("\x03", _) => FlexState {
            running: false,
            ..state
        },
        (_, FlexMode::Write) => write_key(state, key),
        (_, FlexMode::Move) => move_key(state, key),
    }
}

fn write_key(state: FlexState, key: &str) -> FlexState {
    match key {
        "\r" => FlexState {
            mode: FlexMode::Move,
            ..state
        },
        "\x7f" => {
            let mut text = state.text;
            text.pop();
            FlexState { text, ..state }
        }
        _ => match printable_char(key) {
            Some(c) => {
                let mut text = state.text;
                text.push(c);
                FlexState { text, ..state }
            }
            None => state,
        },
    }
}

fn move_key(state: FlexState, key: &str) -> FlexState {
    match key {
        "i" => FlexState {
            mode: FlexMode::Write,
            ..state
        },
        _ => state,
    }
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

    #[test]
    fn starts_in_write_mode_with_an_empty_box_and_running() {
        let state = FlexState::default();
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(state.text, "");
        assert!(state.running);
    }

    #[test]
    fn ctrl_c_stops_running() {
        let state = reduce(FlexState::default(), "\x03");
        assert!(!state.running);
    }

    #[test]
    fn ctrl_c_keeps_the_text() {
        let before = FlexState {
            text: "Hello".to_string(),
            ..FlexState::default()
        };
        let after = reduce(before.clone(), "\x03");
        assert_eq!(after.text, before.text);
    }

    fn typed(keys: &[&str]) -> FlexState {
        keys.iter()
            .fold(FlexState::default(), |state, key| reduce(state, key))
    }

    #[test]
    fn typing_appends_each_character() {
        assert_eq!(typed(&["H", "e", "l", "l", "o"]).text, "Hello");
    }

    #[test]
    fn typing_a_multibyte_character_appends_it() {
        assert_eq!(typed(&["c", "a", "f", "é"]).text, "café");
    }

    #[test]
    fn backspace_removes_the_last_character() {
        assert_eq!(typed(&["H", "e", "l", "l", "p", "\x7f"]).text, "Hell");
    }

    #[test]
    fn backspace_removes_a_whole_multibyte_character() {
        assert_eq!(typed(&["c", "a", "f", "é", "\x7f"]).text, "caf");
    }

    #[test]
    fn backspace_on_an_empty_box_leaves_the_state_unchanged() {
        let before = FlexState::default();
        assert_eq!(reduce(before.clone(), "\x7f"), before);
    }

    #[test]
    fn keys_that_are_not_a_single_printable_char_leave_the_state_unchanged() {
        let before = FlexState {
            text: "Hi".to_string(),
            ..FlexState::default()
        };
        for key in ["\x1b[A", "\x01", "\x1b", "\t", "", "ab", tty::RESIZE] {
            assert_eq!(reduce(before.clone(), key), before, "key {key:?}");
        }
    }

    fn hello_in(mode: FlexMode) -> FlexState {
        FlexState {
            text: "Hello".to_string(),
            mode,
            ..FlexState::default()
        }
    }

    #[test]
    fn enter_in_write_mode_switches_to_move_and_keeps_the_text() {
        let state = reduce(hello_in(FlexMode::Write), "\r");
        assert_eq!(state.mode, FlexMode::Move);
        assert_eq!(state.text, "Hello");
    }

    #[test]
    fn line_feed_in_write_mode_leaves_the_state_unchanged() {
        let before = hello_in(FlexMode::Write);
        assert_eq!(reduce(before.clone(), "\n"), before);
    }

    #[test]
    fn typing_in_move_mode_leaves_the_text_and_mode_unchanged() {
        let state = reduce(hello_in(FlexMode::Move), "x");
        assert_eq!(state.text, "Hello");
        assert_eq!(state.mode, FlexMode::Move);
    }

    #[test]
    fn backspace_in_move_mode_leaves_the_text_unchanged() {
        let state = reduce(hello_in(FlexMode::Move), "\x7f");
        assert_eq!(state.text, "Hello");
    }

    #[test]
    fn i_in_move_mode_switches_to_write_and_keeps_the_text() {
        let state = reduce(hello_in(FlexMode::Move), "i");
        assert_eq!(state.mode, FlexMode::Write);
        assert_eq!(state.text, "Hello");
    }

    #[test]
    fn ctrl_c_in_move_mode_stops_running() {
        let state = reduce(hello_in(FlexMode::Move), "\x03");
        assert!(!state.running);
    }
}
