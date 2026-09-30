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
        _ => state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
