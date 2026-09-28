use crate::state::text_edit::TextKey;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CommandAction {
    Undo,
    NewBox,
    NewSibling,
    Delete,
    Paste,
    SelectParent,
    SelectChild,
    SelectNext,
    SelectPrevious,
    EditLabel,
    RenameLabel,
    CycleColour,
    CycleSiblingsColour,
    ToggleSiblingsFill,
    ToggleSiblingsRounded,
    ToggleFill,
    ToggleRounded,
    Quit,
    Interrupt,
    OpenNamePrompt,
    Digit(u8),
    CancelCount,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InsertAction {
    Commit,
    CommitAndAddChild,
    InsertKey(TextKey),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum NamePromptAction {
    NameAppend(char),
    NameBackspace,
    NameConfirm,
    NameCancel,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Action {
    Command(CommandAction),
    Insert(InsertAction),
    NamePrompt(NamePromptAction),
}

impl CommandAction {
    pub(crate) fn min_depth(&self) -> usize {
        match self {
            CommandAction::Undo
            | CommandAction::NewBox
            | CommandAction::Paste
            | CommandAction::Quit
            | CommandAction::Interrupt
            | CommandAction::OpenNamePrompt
            | CommandAction::Digit(_)
            | CommandAction::CancelCount => 0,
            CommandAction::NewSibling
            | CommandAction::SelectChild
            | CommandAction::SelectNext
            | CommandAction::SelectPrevious
            | CommandAction::EditLabel
            | CommandAction::RenameLabel
            | CommandAction::CycleColour
            | CommandAction::CycleSiblingsColour
            | CommandAction::ToggleSiblingsFill
            | CommandAction::ToggleFill
            | CommandAction::Delete
            | CommandAction::ToggleRounded
            | CommandAction::ToggleSiblingsRounded => 1,
            CommandAction::SelectParent => 2,
        }
    }

    pub(crate) fn undoable(&self) -> bool {
        match self {
            CommandAction::NewBox
            | CommandAction::Paste
            | CommandAction::NewSibling
            | CommandAction::Delete
            | CommandAction::EditLabel
            | CommandAction::RenameLabel
            | CommandAction::CycleColour
            | CommandAction::CycleSiblingsColour
            | CommandAction::ToggleFill
            | CommandAction::ToggleSiblingsFill
            | CommandAction::ToggleRounded
            | CommandAction::ToggleSiblingsRounded => true,
            CommandAction::Undo
            | CommandAction::Quit
            | CommandAction::SelectParent
            | CommandAction::SelectChild
            | CommandAction::SelectNext
            | CommandAction::SelectPrevious
            | CommandAction::Interrupt
            | CommandAction::OpenNamePrompt
            | CommandAction::Digit(_)
            | CommandAction::CancelCount => false,
        }
    }
}

impl InsertAction {
    pub(crate) fn undoable(&self) -> bool {
        match self {
            InsertAction::CommitAndAddChild => true,
            InsertAction::Commit | InsertAction::InsertKey(_) => false,
        }
    }
}

impl Action {
    pub(crate) fn undoable(&self) -> bool {
        match self {
            Action::Command(a) => a.undoable(),
            Action::Insert(a) => a.undoable(),
            Action::NamePrompt(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_actions_below_selection_depth_two_need_no_selection() {
        assert_eq!(CommandAction::SelectParent.min_depth(), 2);
        assert_eq!(CommandAction::EditLabel.min_depth(), 1);
        assert_eq!(CommandAction::Undo.min_depth(), 0);
    }

    #[test]
    fn only_mutating_command_and_commit_and_add_child_actions_are_undoable() {
        assert!(CommandAction::NewBox.undoable());
        assert!(!CommandAction::SelectChild.undoable());
        assert!(InsertAction::CommitAndAddChild.undoable());
        assert!(!InsertAction::Commit.undoable());
    }

    #[test]
    fn no_name_prompt_action_is_undoable() {
        assert!(!Action::NamePrompt(NamePromptAction::NameConfirm).undoable());
        assert!(!Action::NamePrompt(NamePromptAction::NameAppend('a')).undoable());
    }
}
