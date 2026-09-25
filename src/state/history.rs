use crate::diagram::Node;
use crate::state::action::Action;
use crate::state::State;
use types::Tree;

fn is_undoable(action: &Action) -> bool {
    matches!(
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
    )
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
    let undoable = is_undoable(action);
    let state = if undoable { snapshot(state) } else { state };
    let state = reduce(state);
    if undoable || *action == Action::Commit {
        drop_snapshot_if_unchanged(state)
    } else {
        state
    }
}

fn nearest_existing(tree: &Tree<Node>, mut path: Vec<usize>) -> Option<Vec<usize>> {
    while !path.is_empty() && !tree.contains(&path) {
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
            .and_then(|path| nearest_existing(state.doc.tree(), path));
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
        assert!(!is_undoable(&Action::Idle));
        assert!(!is_undoable(&Action::Interrupt));
    }

    #[test]
    fn creating_and_editing_actions_are_undoable() {
        for action in [
            Action::NewBox,
            Action::NewSibling,
            Action::Delete,
            Action::Paste,
            Action::EditLabel,
            Action::RenameLabel,
            Action::CommitAndAddChild,
        ] {
            assert!(is_undoable(&action));
        }
    }

    #[test]
    fn save_prompt_actions_are_not_undoable() {
        for action in [
            Action::Confirm,
            Action::Cancel,
            Action::SavePromptBackspace,
            Action::SavePromptAppend('a'),
        ] {
            assert!(!is_undoable(&action));
        }
    }

    #[test]
    fn name_actions_are_not_undoable() {
        for action in [
            Action::OpenNamePrompt,
            Action::NameAppend('a'),
            Action::NameBackspace,
            Action::NameConfirm,
            Action::NameCancel,
        ] {
            assert!(!is_undoable(&action));
        }
    }

    #[test]
    fn typing_in_insert_mode_is_not_undoable() {
        assert!(!is_undoable(&Action::InsertAppend('a')));
        assert!(!is_undoable(&Action::InsertBackspace));
        assert!(!is_undoable(&Action::Commit));
    }
}
