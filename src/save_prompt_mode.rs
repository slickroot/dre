use crate::state::{Mode, State};

const EXTENSION: &str = ".dre";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Command {
    Confirm,
    Cancel,
    Backspace,
    Append(char),
}

pub(crate) fn parse(key: &str) -> Option<Command> {
    match key {
        "\r" => Some(Command::Confirm),
        "\x1b" => Some(Command::Cancel),
        "\x7f" => Some(Command::Backspace),
        _ => match key.chars().next() {
            Some(c) if ('\x20'..='\x7e').contains(&c) => Some(Command::Append(c)),
            _ => None,
        },
    }
}

fn with_extension(filename: &str) -> String {
    if filename.ends_with(EXTENSION) {
        filename.to_string()
    } else {
        format!("{filename}{EXTENSION}")
    }
}

pub(crate) fn reduce(mut state: State, command: Command) -> State {
    let Mode::SavePrompt { filename } = &mut state.mode else {
        return state;
    };
    match command {
        Command::Confirm => {
            state.save_to = Some(with_extension(filename));
            state.running = false;
        }
        Command::Cancel => state.running = false,
        Command::Backspace => {
            filename.pop();
        }
        Command::Append(c) => filename.push(c),
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{Node, Path};
    use crate::state::{handle_key, new_state, Mode, DEFAULT_FILENAME};

    fn node(label: &str) -> Node {
        Node {
            label: label.to_string(),
            ..Default::default()
        }
    }

    fn prompt(filename: &str) -> Mode {
        Mode::SavePrompt {
            filename: filename.to_string(),
        }
    }

    #[test]
    fn q_in_command_mode_opens_the_prompt_with_the_default_filename() {
        let state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "q");
        assert_eq!(result.mode, prompt(DEFAULT_FILENAME));
        assert!(result.running);
        assert_eq!(result.save_to, None);
    }

    #[test]
    fn typing_appends_to_the_filename() {
        let state = new_state(vec![], prompt("a"), None);
        let result = handle_key(state, "b");
        assert_eq!(result.mode, prompt("ab"));
    }

    #[test]
    fn backspace_drops_the_last_character_and_is_a_no_op_when_empty() {
        let state = new_state(vec![], prompt("ab"), None);
        let result = handle_key(state, "\x7f");
        assert_eq!(result.mode, prompt("a"));

        let state = new_state(vec![], prompt(""), None);
        let result = handle_key(state, "\x7f");
        assert_eq!(result.mode, prompt(""));
    }

    #[test]
    fn control_and_non_ascii_characters_leave_the_filename_unchanged() {
        let state = new_state(vec![], prompt("a"), None);
        let result = handle_key(state.clone(), "\x01");
        assert_eq!(result.mode, prompt("a"));

        let result = handle_key(state, "é");
        assert_eq!(result.mode, prompt("a"));
    }

    #[test]
    fn enter_saves_to_the_filename_with_the_extension_added_and_stops() {
        let state = new_state(vec![], prompt("notes"), None);
        let result = handle_key(state, "\r");
        assert_eq!(result.save_to, Some(format!("notes{EXTENSION}")));
        assert!(!result.running);
    }

    #[test]
    fn enter_keeps_a_filename_that_already_has_the_extension() {
        let state = new_state(vec![], prompt(DEFAULT_FILENAME), None);
        let result = handle_key(state, "\r");
        assert_eq!(result.save_to, Some(DEFAULT_FILENAME.to_string()));
        assert!(!result.running);
    }

    #[test]
    fn escape_stops_without_saving_and_preserves_boxes() {
        let state = new_state(
            vec![node("a")],
            prompt(DEFAULT_FILENAME),
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "\x1b");
        assert!(!result.running);
        assert_eq!(result.save_to, None);
        assert_eq!(result.doc.boxes, vec![node("a")]);
    }
}
