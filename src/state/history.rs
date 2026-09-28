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
    let state = if action.undoable() {
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
    use crate::state::action::{CommandAction, InsertAction, NamePromptAction};
    use crate::state::text_edit::TextKey;

    #[test]
    fn interrupt_is_not_undoable() {
        assert!(!Action::Command(CommandAction::Interrupt).undoable());
    }

    #[test]
    fn creating_and_editing_command_actions_are_undoable() {
        for action in [
            CommandAction::NewBox,
            CommandAction::NewSibling,
            CommandAction::Delete,
            CommandAction::Paste,
            CommandAction::EditLabel,
            CommandAction::RenameLabel,
        ] {
            assert!(Action::Command(action).undoable());
        }
    }

    #[test]
    fn commit_and_add_child_is_undoable() {
        assert!(Action::Insert(InsertAction::CommitAndAddChild).undoable());
    }

    #[test]
    fn name_actions_are_not_undoable() {
        assert!(!Action::Command(CommandAction::OpenNamePrompt).undoable());
        for action in [
            NamePromptAction::NameAppend('a'),
            NamePromptAction::NameBackspace,
            NamePromptAction::NameConfirm,
            NamePromptAction::NameCancel,
        ] {
            assert!(!Action::NamePrompt(action).undoable());
        }
    }

    #[test]
    fn typing_in_insert_mode_is_not_undoable() {
        assert!(!Action::Insert(InsertAction::InsertKey(TextKey::Char('a'))).undoable());
        assert!(!Action::Insert(InsertAction::InsertKey(TextKey::Backspace)).undoable());
        assert!(!Action::Insert(InsertAction::Commit).undoable());
    }
}
