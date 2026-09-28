use crate::state::action::NamePromptAction;
use crate::state::{Mode, State};

const EXTENSION: &str = ".dre";

pub(crate) fn reduce(mut state: State, command: NamePromptAction) -> State {
    let Mode::NamePrompt { name, quits } = &mut state.mode else {
        return state;
    };
    let quits = *quits;
    match command {
        NamePromptAction::NameConfirm if !name.is_empty() => {
            let path = format!("{name}{EXTENSION}");
            state.set_save_to(Some(path));
            if quits {
                state.running = false;
            } else {
                state.mode = Mode::Command;
            }
        }
        NamePromptAction::NameConfirm => {}
        NamePromptAction::NameCancel => {
            if quits {
                state.running = false;
            } else {
                state.mode = Mode::Command;
            }
        }
        NamePromptAction::NameBackspace => {
            name.pop();
        }
        NamePromptAction::NameAppend(c) => name.push(c),
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::node;
    use crate::state::new_state;
    use crate::test_support::handle_key;
    use types::Tree;

    fn prompt(name: &str) -> Mode {
        Mode::NamePrompt {
            name: name.to_string(),
            quits: false,
        }
    }

    fn quitting_prompt(name: &str) -> Mode {
        Mode::NamePrompt {
            name: name.to_string(),
            quits: true,
        }
    }

    #[test]
    fn n_in_command_mode_opens_an_empty_prompt() {
        let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let result = handle_key(state, "n");
        assert_eq!(result.mode, prompt(""));
        assert!(result.running);
    }

    #[test]
    fn typing_appends_and_backspace_drops_the_last_character() {
        let state = new_state(vec![], prompt("a"), None);
        let result = handle_key(state, "b");
        assert_eq!(result.mode, prompt("ab"));
        let result = handle_key(result, "\x7f");
        assert_eq!(result.mode, prompt("a"));

        let state = new_state(vec![], prompt(""), None);
        assert_eq!(handle_key(state, "\x7f").mode, prompt(""));
    }

    #[test]
    fn enter_saves_to_the_name_with_the_extension_and_returns_to_command() {
        let state = new_state(vec![], prompt("plans"), None);
        let result = handle_key(state, "\r");
        assert_eq!(result.save_to(), Some(format!("plans{EXTENSION}").as_str()));
        assert_eq!(result.mode, Mode::Command);
        assert!(result.running);
    }

    #[test]
    fn enter_confirms_the_name_and_drops_the_hint_from_the_footer() {
        let state = new_state(vec![], prompt("plans"), None);
        let result = handle_key(state, "\r");
        assert_eq!(result.footer().mode, crate::state::FooterMode::Move);
        assert_eq!(result.footer().filename, Some("plans".to_string()));
    }

    #[test]
    fn enter_always_adds_the_extension() {
        let name = format!("plans{EXTENSION}");
        let state = new_state(vec![], prompt(&name), None);
        let result = handle_key(state, "\r");
        assert_eq!(
            result.save_to(),
            Some(format!("{name}{EXTENSION}").as_str())
        );
    }

    #[test]
    fn enter_with_an_empty_name_leaves_the_prompt_open() {
        let mut state = new_state(vec![], prompt(""), None);
        state.set_save_to(Some("old.dre".to_string()));
        let result = handle_key(state, "\r");
        assert_eq!(result.mode, prompt(""));
        assert_eq!(result.save_to(), Some("old.dre"));
    }

    #[test]
    fn escape_closes_the_prompt_leaving_save_to_and_the_document() {
        let mut state = new_state(vec![node("a")], prompt("x"), Some(vec![0]));
        state.set_save_to(Some("old.dre".to_string()));
        let result = handle_key(state, "\x1b");
        assert_eq!(result.mode, Mode::Command);
        assert!(result.running);
        assert_eq!(result.save_to(), Some("old.dre"));
        assert_eq!(*result.doc.tree(), Tree::root(vec![node("a")]));
    }

    #[test]
    fn naming_a_named_diagram_changes_its_save_path() {
        let mut state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        state.set_save_to(Some("old.dre".to_string()));
        let mut result = state;
        for key in ["n", "n", "e", "w", "\r"] {
            result = handle_key(result, key);
        }
        assert_eq!(result.save_to(), Some(format!("new{EXTENSION}").as_str()));
    }

    #[test]
    fn enter_with_a_name_in_a_quitting_prompt_saves_and_stops_running() {
        let state = new_state(vec![], quitting_prompt("plans"), None);
        let result = handle_key(state, "\r");
        assert_eq!(result.save_to(), Some(format!("plans{EXTENSION}").as_str()));
        assert!(!result.running);
    }

    #[test]
    fn escape_in_a_quitting_prompt_stops_running() {
        let state = new_state(vec![node("a")], quitting_prompt("x"), Some(vec![0]));
        let result = handle_key(state, "\x1b");
        assert!(!result.running);
    }

    #[test]
    fn enter_with_an_empty_name_in_a_quitting_prompt_leaves_the_prompt_open() {
        let state = new_state(vec![], quitting_prompt(""), None);
        let result = handle_key(state, "\r");
        assert_eq!(result.mode, quitting_prompt(""));
        assert!(result.running);
    }
}
