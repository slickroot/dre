use crate::state::{at, Mode, State, PAD};

fn drop_last_chars(s: &str, n: usize) -> String {
    let len = s.chars().count();
    s.chars().take(len.saturating_sub(n)).collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Command {
    Commit,
    Backspace,
    Append(char),
}

pub(crate) fn parse(key: &str) -> Option<Command> {
    match key {
        "\x1b" => Some(Command::Commit),
        "\x7f" => Some(Command::Backspace),
        _ => match key.chars().next() {
            Some(c) if ('\x20'..='\x7e').contains(&c) => Some(Command::Append(c)),
            _ => None,
        },
    }
}

pub(crate) fn reduce(mut state: State, command: Command) -> State {
    if state.doc.selected.is_none() {
        return state;
    }
    let path = state.doc.selected.clone().unwrap();
    let node = at(&mut state.doc.boxes, &path);
    let label = node.label.clone();
    match command {
        Command::Commit => {
            node.label = drop_last_chars(&label, 1);
            state.mode = Mode::Command;
        }
        Command::Backspace => node.label = format!("{}{PAD}", drop_last_chars(&label, 2)),
        Command::Append(c) => node.label = format!("{}{c}{PAD}", drop_last_chars(&label, 1)),
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{handle_key, new_state, Node, Path};

    fn node(label: &str) -> Node {
        Node { label: label.to_string(), ..Default::default() }
    }

    #[test]
    fn parse_maps_escape_to_commit() {
        assert_eq!(parse("\x1b"), Some(Command::Commit));
    }

    #[test]
    fn parse_maps_delete_to_backspace() {
        assert_eq!(parse("\x7f"), Some(Command::Backspace));
    }

    #[test]
    fn parse_maps_the_lower_printable_boundary_to_append() {
        assert_eq!(parse("\x20"), Some(Command::Append('\x20')));
    }

    #[test]
    fn parse_maps_the_upper_printable_boundary_to_append() {
        assert_eq!(parse("\x7e"), Some(Command::Append('\x7e')));
    }

    #[test]
    fn parse_returns_nothing_just_outside_the_printable_range() {
        assert_eq!(parse("\x1f"), None);
        assert_eq!(parse("\x01"), None);
        assert_eq!(parse("é"), None);
    }

    #[test]
    fn h_in_insert_mode_types_the_letter_h() {
        let state = new_state(vec![node(&format!("a{PAD}"))], Mode::Insert, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "h");
        assert_eq!(result.doc.boxes, vec![node(&format!("ah{PAD}"))]);
    }

    #[test]
    fn typing_appends_to_the_selected_box_label() {
        let state = new_state(vec![node(&format!("h{PAD}"))], Mode::Insert, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "i");
        assert_eq!(result.doc.boxes, vec![node(&format!("hi{PAD}"))]);
    }

    #[test]
    fn space_and_tilde_are_printable() {
        let state = new_state(vec![node(&format!("a{PAD}"))], Mode::Insert, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, " ");
        assert_eq!(result.doc.boxes, vec![node(&format!("a {PAD}"))]);

        let state = new_state(vec![node(PAD)], Mode::Insert, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "~");
        assert_eq!(result.doc.boxes, vec![node(&format!("~{PAD}"))]);
    }

    #[test]
    fn backspace_drops_the_last_character_and_is_a_no_op_when_empty() {
        let state = new_state(vec![node(&format!("hi{PAD}"))], Mode::Insert, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "\x7f");
        assert_eq!(result.doc.boxes, vec![node(&format!("h{PAD}"))]);

        let state = new_state(vec![node(PAD)], Mode::Insert, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "\x7f");
        assert_eq!(result.doc.boxes, vec![node(PAD)]);
    }

    #[test]
    fn esc_returns_to_command_mode_and_trims_pad() {
        let state = new_state(vec![node(&format!("hi{PAD}"))], Mode::Insert, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "\x1b");
        assert_eq!(result.mode, Mode::Command);
        assert_eq!(result.doc.boxes, vec![node("hi")]);
    }

    #[test]
    fn control_and_non_ascii_characters_return_the_state_unchanged() {
        let state = new_state(vec![node(&format!("hi{PAD}"))], Mode::Insert, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state.clone(), "\x01");
        assert_eq!(result.doc.boxes, vec![node(&format!("hi{PAD}"))]);

        let result = handle_key(state, "é");
        assert_eq!(result.doc.boxes, vec![node(&format!("hi{PAD}"))]);
    }

    #[test]
    fn insert_mode_is_dispatched_separately() {
        let state = new_state(vec![node(PAD)], Mode::Insert, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "q");
        assert!(result.running);
    }
}
