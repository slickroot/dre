use crate::diagram::Node;
use crate::state::action::Action;
use crate::state::State;
use types::Tree;

fn snapshot(mut state: State) -> State {
    state.history.push(state.doc.clone());
    state
}

pub(super) fn recorded(
    state: State,
    action: &Action,
    reduce: impl FnOnce(State) -> State,
) -> State {
    let state = if action.spec().undoable {
        snapshot(state)
    } else {
        state
    };
    reduce(state)
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
    use crate::state::text_edit::TextKey;

    #[test]
    fn scroll_idle_and_interrupt_are_not_undoable() {
        assert!(!Action::Idle.spec().undoable);
        assert!(!Action::Interrupt.spec().undoable);
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
            assert!(action.spec().undoable);
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
            assert!(!action.spec().undoable);
        }
    }

    #[test]
    fn typing_in_insert_mode_is_not_undoable() {
        assert!(!Action::InsertKey(TextKey::Char('a')).spec().undoable);
        assert!(!Action::InsertKey(TextKey::Backspace).spec().undoable);
        assert!(!Action::Commit.spec().undoable);
    }
}
