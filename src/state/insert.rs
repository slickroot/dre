use crate::diagram::at;
use crate::state::action::Action;
use crate::state::{add_child_box, Mode, State, PAD};

fn drop_last_chars(s: &str, n: usize) -> String {
    let len = s.chars().count();
    s.chars().take(len.saturating_sub(n)).collect()
}

pub(crate) fn reduce(mut state: State, command: Action) -> State {
    let Some(path) = &state.selected else {
        return state;
    };
    let node = at(&mut state.doc.boxes, path);
    let label = node.label.clone();
    match command {
        Action::Commit => {
            node.label = drop_last_chars(&label, 1);
            state.mode = Mode::Command;
            state
        }
        Action::CommitAndAddChild => {
            node.label = drop_last_chars(&label, 1);
            let selected = state.selected.clone();
            add_child_box(state, selected)
        }
        Action::InsertBackspace => {
            node.label = format!("{}{PAD}", drop_last_chars(&label, 2));
            state
        }
        Action::InsertAppend(c) => {
            node.label = format!("{}{c}{PAD}", drop_last_chars(&label, 1));
            state
        }
        _ => state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{node, node_with_children};
    use crate::state::new_state;
    use crate::test_support::handle_key;

    #[test]
    fn enter_finishes_the_box_and_adds_an_empty_child_ready_for_typing() {
        let state = new_state(vec![node(&format!("hi{PAD}"))], Mode::Insert, Some(vec![0]));
        let result = handle_key(state, "\r");
        assert_eq!(
            result.doc.boxes,
            vec![node_with_children("hi", vec![node(PAD)])]
        );
        assert_eq!(result.selected, Some(vec![0, 0]));
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn enter_on_an_empty_label_still_creates_an_empty_child() {
        let state = new_state(vec![node(PAD)], Mode::Insert, Some(vec![0]));
        let result = handle_key(state, "\r");
        assert_eq!(
            result.doc.boxes,
            vec![node_with_children("", vec![node(PAD)])]
        );
        assert_eq!(result.selected, Some(vec![0, 0]));
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn u_after_enter_and_esc_reverts_the_child_and_keeps_the_label() {
        let state = new_state(vec![node(&format!("hi{PAD}"))], Mode::Insert, Some(vec![0]));
        let state = handle_key(state, "\r");
        let state = handle_key(state, "\x1b");
        let result = handle_key(state, "u");
        assert_eq!(result.doc.boxes, vec![node(&format!("hi{PAD}"))]);
        assert_eq!(result.selected, Some(vec![0]));
        assert_eq!(result.mode, Mode::Command);
    }

    fn press(state: State, keys: &[&str]) -> State {
        keys.iter().fold(state, |state, key| handle_key(state, key))
    }

    fn command_mode_cache_box() -> State {
        new_state(vec![node("Cache")], Mode::Command, Some(vec![0]))
    }

    #[test]
    fn enter_in_an_edit_session_makes_the_child_and_its_text_one_undo_step() {
        let state = press(
            command_mode_cache_box(),
            &["i", "!", "\r", "R", "e", "d", "i", "s", "\x1b"],
        );
        assert_eq!(
            state.doc.boxes,
            vec![node_with_children("Cache!", vec![node("Redis")])]
        );
        let state = handle_key(state, "u");
        assert_eq!(state.doc.boxes, vec![node(&format!("Cache!{PAD}"))]);
        let state = handle_key(state, "u");
        assert_eq!(state.doc.boxes, vec![node("Cache")]);
        let state = handle_key(state, "u");
        assert_eq!(state.doc.boxes, vec![node("Cache")]);
    }

    #[test]
    fn enter_without_changing_the_text_still_leaves_the_edit_entry_as_a_step() {
        let state = press(command_mode_cache_box(), &["i", "\r", "R", "\x1b"]);
        let state = handle_key(state, "u");
        assert_eq!(state.doc.boxes, vec![node(&format!("Cache{PAD}"))]);
        let state = handle_key(state, "u");
        assert_eq!(state.doc.boxes, vec![node("Cache")]);
        let state = handle_key(state, "u");
        assert_eq!(state.doc.boxes, vec![node("Cache")]);
    }

    #[test]
    fn enter_then_esc_in_the_empty_child_leaves_the_child_as_a_step_after_the_edit_entry() {
        let state = press(command_mode_cache_box(), &["i", "x", "\r", "\x1b"]);
        let state = handle_key(state, "u");
        assert_eq!(state.doc.boxes, vec![node(&format!("Cachex{PAD}"))]);
        let state = handle_key(state, "u");
        assert_eq!(state.doc.boxes, vec![node("Cache")]);
    }

    #[test]
    fn repeated_enter_drills_deeper_creating_a_child_then_a_grandchild() {
        let state = new_state(vec![node(&format!("a{PAD}"))], Mode::Insert, Some(vec![0]));
        let state = handle_key(state, "\r");
        let result = handle_key(state, "\r");
        assert_eq!(
            result.doc.boxes,
            vec![node_with_children(
                "a",
                vec![node_with_children("", vec![node(PAD)])]
            )]
        );
        assert_eq!(result.selected, Some(vec![0, 0, 0]));
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn h_in_insert_mode_types_the_letter_h() {
        let state = new_state(vec![node(&format!("a{PAD}"))], Mode::Insert, Some(vec![0]));
        let result = handle_key(state, "h");
        assert_eq!(result.doc.boxes, vec![node(&format!("ah{PAD}"))]);
    }

    #[test]
    fn typing_appends_to_the_selected_box_label() {
        let state = new_state(vec![node(&format!("h{PAD}"))], Mode::Insert, Some(vec![0]));
        let result = handle_key(state, "i");
        assert_eq!(result.doc.boxes, vec![node(&format!("hi{PAD}"))]);
    }

    #[test]
    fn space_and_tilde_are_printable() {
        let state = new_state(vec![node(&format!("a{PAD}"))], Mode::Insert, Some(vec![0]));
        let result = handle_key(state, " ");
        assert_eq!(result.doc.boxes, vec![node(&format!("a {PAD}"))]);

        let state = new_state(vec![node(PAD)], Mode::Insert, Some(vec![0]));
        let result = handle_key(state, "~");
        assert_eq!(result.doc.boxes, vec![node(&format!("~{PAD}"))]);
    }

    #[test]
    fn backspace_drops_the_last_character_and_is_a_no_op_when_empty() {
        let state = new_state(vec![node(&format!("hi{PAD}"))], Mode::Insert, Some(vec![0]));
        let result = handle_key(state, "\x7f");
        assert_eq!(result.doc.boxes, vec![node(&format!("h{PAD}"))]);

        let state = new_state(vec![node(PAD)], Mode::Insert, Some(vec![0]));
        let result = handle_key(state, "\x7f");
        assert_eq!(result.doc.boxes, vec![node(PAD)]);
    }

    #[test]
    fn esc_returns_to_command_mode_and_trims_pad() {
        let state = new_state(vec![node(&format!("hi{PAD}"))], Mode::Insert, Some(vec![0]));
        let result = handle_key(state, "\x1b");
        assert_eq!(result.mode, Mode::Command);
        assert_eq!(result.doc.boxes, vec![node("hi")]);
    }

    #[test]
    fn control_and_non_ascii_characters_return_the_state_unchanged() {
        let state = new_state(vec![node(&format!("hi{PAD}"))], Mode::Insert, Some(vec![0]));
        let result = handle_key(state.clone(), "\x01");
        assert_eq!(result.doc.boxes, vec![node(&format!("hi{PAD}"))]);

        let result = handle_key(state, "é");
        assert_eq!(result.doc.boxes, vec![node(&format!("hi{PAD}"))]);
    }

    #[test]
    fn insert_mode_is_dispatched_separately() {
        let state = new_state(vec![node(PAD)], Mode::Insert, Some(vec![0]));
        let result = handle_key(state, "q");
        assert!(result.running);
    }

    #[test]
    fn digit_in_insert_mode_types_the_digit_as_label_text() {
        for key in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
            let state = new_state(vec![node(&format!("a{PAD}"))], Mode::Insert, Some(vec![0]));
            let result = handle_key(state, key);
            assert_eq!(result.doc.boxes, vec![node(&format!("a{key}{PAD}"))]);
            assert_eq!(result.mode, Mode::Insert);
            assert_eq!(result.selected, Some(vec![0]));
        }
    }
}
