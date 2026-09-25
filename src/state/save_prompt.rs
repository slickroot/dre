use crate::state::action::Action;
use crate::state::{Mode, State};

const EXTENSION: &str = ".dre";

fn with_extension(filename: &str) -> String {
    if filename.ends_with(EXTENSION) {
        filename.to_string()
    } else {
        format!("{filename}{EXTENSION}")
    }
}

pub(crate) fn reduce(mut state: State, command: Action) -> State {
    let Mode::SavePrompt { filename } = &mut state.mode else {
        return state;
    };
    match command {
        Action::Confirm => {
            state.save_to = Some(with_extension(filename));
            state.running = false;
        }
        Action::Cancel => state.running = false,
        Action::SavePromptBackspace => {
            filename.pop();
        }
        Action::SavePromptAppend(c) => filename.push(c),
        _ => {}
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::node;
    use crate::state::{new_state, Mode, DEFAULT_FILENAME};
    use crate::test_support::handle_key;
    use types::Tree;

    fn prompt(filename: &str) -> Mode {
        Mode::SavePrompt {
            filename: filename.to_string(),
        }
    }

    #[test]
    fn q_in_command_mode_opens_the_prompt_with_the_default_filename() {
        let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
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
        let state = new_state(vec![node("a")], prompt(DEFAULT_FILENAME), Some(vec![0]));
        let result = handle_key(state, "\x1b");
        assert!(!result.running);
        assert_eq!(result.save_to, None);
        assert_eq!(*result.doc.tree(), Tree::root(vec![node("a")]));
    }
}
