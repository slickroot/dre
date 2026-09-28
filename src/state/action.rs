use crate::state::text_edit::TextKey;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SelectionCommand {
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
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ImmediateCommand {
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
    Selection(SelectionCommand),
    Immediate(ImmediateCommand),
    Insert(InsertAction),
    NamePrompt(NamePromptAction),
}

impl SelectionCommand {
    pub(crate) fn min_depth(&self) -> usize {
        match self {
            SelectionCommand::Undo
            | SelectionCommand::NewBox
            | SelectionCommand::Paste
            | SelectionCommand::Quit => 0,
            SelectionCommand::NewSibling
            | SelectionCommand::SelectChild
            | SelectionCommand::SelectNext
            | SelectionCommand::SelectPrevious
            | SelectionCommand::EditLabel
            | SelectionCommand::RenameLabel
            | SelectionCommand::CycleColour
            | SelectionCommand::CycleSiblingsColour
            | SelectionCommand::ToggleSiblingsFill
            | SelectionCommand::ToggleFill
            | SelectionCommand::Delete
            | SelectionCommand::ToggleRounded
            | SelectionCommand::ToggleSiblingsRounded => 1,
            SelectionCommand::SelectParent => 2,
        }
    }

    pub(crate) fn undoable(&self) -> bool {
        match self {
            SelectionCommand::NewBox
            | SelectionCommand::Paste
            | SelectionCommand::NewSibling
            | SelectionCommand::Delete
            | SelectionCommand::EditLabel
            | SelectionCommand::RenameLabel
            | SelectionCommand::CycleColour
            | SelectionCommand::CycleSiblingsColour
            | SelectionCommand::ToggleFill
            | SelectionCommand::ToggleSiblingsFill
            | SelectionCommand::ToggleRounded
            | SelectionCommand::ToggleSiblingsRounded => true,
            SelectionCommand::Undo
            | SelectionCommand::Quit
            | SelectionCommand::SelectParent
            | SelectionCommand::SelectChild
            | SelectionCommand::SelectNext
            | SelectionCommand::SelectPrevious => false,
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
            Action::Selection(a) => a.undoable(),
            Action::Immediate(_) => false,
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
        assert_eq!(SelectionCommand::SelectParent.min_depth(), 2);
        assert_eq!(SelectionCommand::EditLabel.min_depth(), 1);
        assert_eq!(SelectionCommand::Undo.min_depth(), 0);
    }

    #[test]
    fn only_mutating_command_and_commit_and_add_child_actions_are_undoable() {
        assert!(SelectionCommand::NewBox.undoable());
        assert!(!SelectionCommand::SelectChild.undoable());
        assert!(InsertAction::CommitAndAddChild.undoable());
        assert!(!InsertAction::Commit.undoable());
    }

    #[test]
    fn no_name_prompt_action_is_undoable() {
        assert!(!Action::NamePrompt(NamePromptAction::NameConfirm).undoable());
        assert!(!Action::NamePrompt(NamePromptAction::NameAppend('a')).undoable());
    }
}
