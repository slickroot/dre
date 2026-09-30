#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlexMode {
    Write,
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
    match key {
        "\x03" => FlexState {
            running: false,
            ..state
        },
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
    fn keys_that_are_not_a_single_printable_char_leave_the_state_unchanged() {
        let before = FlexState {
            text: "Hi".to_string(),
            ..FlexState::default()
        };
        for key in ["\x1b[A", "\x01", "\x1b", "\t", "\r", "", "ab", tty::RESIZE] {
            assert_eq!(reduce(before.clone(), key), before, "key {key:?}");
        }
    }
}
