use crate::diagram::Node;
use crate::state::action::Action;
use crate::state::State;

fn is_undoable(state: &State, action: &Action) -> bool {
    match action {
        Action::Digit(_) => state.colour_overlay,
        _ => matches!(
            action,
            Action::NewBox
                | Action::Delete
                | Action::Paste
                | Action::CycleColour
                | Action::ToggleFill
                | Action::ToggleRounded
                | Action::CycleSiblingsColour
                | Action::ToggleSiblingsFill
                | Action::RenameLabel
                | Action::EditLabel
                | Action::NewSibling
                | Action::CommitAndAddChild
        ),
    }
}

fn snapshot(mut state: State) -> State {
    state.history.push(state.doc.clone());
    state.dirty = true;
    state
}

fn drop_snapshot_if_unchanged(mut state: State) -> State {
    if state.history.last() == Some(&state.doc) {
        state.history.pop();
    }
    state
}

pub(super) fn recorded(
    state: State,
    action: &Action,
    reduce: impl FnOnce(State) -> State,
) -> State {
    let undoable = is_undoable(&state, action);
    let state = if undoable { snapshot(state) } else { state };
    let state = reduce(state);
    if undoable || *action == Action::Commit {
        drop_snapshot_if_unchanged(state)
    } else {
        state
    }
}

fn contains(boxes: &[Node], path: &[usize]) -> bool {
    let Some((&last, parent)) = path.split_last() else {
        return false;
    };
    let mut siblings = boxes;
    for &index in parent {
        match siblings.get(index) {
            Some(node) => siblings = &node.children,
            None => return false,
        }
    }
    last < siblings.len()
}

fn nearest_existing(boxes: &[Node], mut path: Vec<usize>) -> Option<Vec<usize>> {
    while !path.is_empty() && !contains(boxes, &path) {
        path.pop();
    }
    (!path.is_empty()).then_some(path)
}

pub(super) fn undo(mut state: State) -> State {
    if let Some(previous) = state.history.pop() {
        state.doc = previous;
        state.selected = state
            .selected
            .take()
            .and_then(|path| nearest_existing(&state.doc.boxes, path));
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{new_state, Mode};

    fn command_state() -> State {
        new_state(vec![], Mode::Command, None)
    }

    #[test]
    fn snapshot_marks_the_state_as_dirty() {
        let state = command_state();
        assert!(!state.dirty);
        assert!(snapshot(state).dirty);
    }

    #[test]
    fn scroll_idle_and_interrupt_are_not_undoable() {
        let state = command_state();
        assert!(!is_undoable(&state, &Action::Idle));
        assert!(!is_undoable(&state, &Action::Interrupt));
    }

    #[test]
    fn a_digit_is_undoable_only_under_the_colour_overlay() {
        let mut state = command_state();
        assert!(!is_undoable(&state, &Action::Digit(3)));
        state.colour_overlay = true;
        assert!(is_undoable(&state, &Action::Digit(3)));
    }

    #[test]
    fn creating_and_editing_actions_are_undoable() {
        let state = command_state();
        for action in [
            Action::NewBox,
            Action::NewSibling,
            Action::Delete,
            Action::Paste,
            Action::EditLabel,
            Action::RenameLabel,
            Action::CommitAndAddChild,
        ] {
            assert!(is_undoable(&state, &action));
        }
    }

    #[test]
    fn save_prompt_actions_are_not_undoable() {
        let state = command_state();
        for action in [
            Action::Confirm,
            Action::Cancel,
            Action::SavePromptBackspace,
            Action::SavePromptAppend('a'),
        ] {
            assert!(!is_undoable(&state, &action));
        }
    }

    #[test]
    fn typing_in_insert_mode_is_not_undoable() {
        let state = command_state();
        assert!(!is_undoable(&state, &Action::InsertAppend('a')));
        assert!(!is_undoable(&state, &Action::InsertBackspace));
        assert!(!is_undoable(&state, &Action::Commit));
    }
}
