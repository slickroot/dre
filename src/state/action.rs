#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Action {
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
    ToggleFill,
    ToggleRounded,
    Quit,
    Idle,
    Interrupt,
    Digit(u8),
    CancelCount,
    Commit,
    CommitAndAddChild,
    InsertBackspace,
    InsertAppend(char),
    Confirm,
    Cancel,
    SavePromptBackspace,
    SavePromptAppend(char),
    OpenNamePrompt,
    NameAppend(char),
    NameBackspace,
    NameConfirm,
    NameCancel,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ActionMode {
    Command,
    Insert,
    SavePrompt,
    NamePrompt,
}

impl Action {
    pub(crate) fn mode(&self) -> ActionMode {
        match self {
            Action::Commit
            | Action::CommitAndAddChild
            | Action::InsertBackspace
            | Action::InsertAppend(_) => ActionMode::Insert,
            Action::Confirm
            | Action::Cancel
            | Action::SavePromptBackspace
            | Action::SavePromptAppend(_) => ActionMode::SavePrompt,
            Action::NameAppend(_)
            | Action::NameBackspace
            | Action::NameConfirm
            | Action::NameCancel => ActionMode::NamePrompt,
            Action::Undo
            | Action::NewBox
            | Action::NewSibling
            | Action::Delete
            | Action::Paste
            | Action::SelectParent
            | Action::SelectChild
            | Action::SelectNext
            | Action::SelectPrevious
            | Action::EditLabel
            | Action::RenameLabel
            | Action::CycleColour
            | Action::CycleSiblingsColour
            | Action::ToggleSiblingsFill
            | Action::ToggleFill
            | Action::ToggleRounded
            | Action::Quit
            | Action::Idle
            | Action::Interrupt
            | Action::OpenNamePrompt
            | Action::Digit(_)
            | Action::CancelCount => ActionMode::Command,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_map_to_their_mode() {
        assert_eq!(Action::InsertAppend('a').mode(), ActionMode::Insert);
        assert_eq!(Action::Confirm.mode(), ActionMode::SavePrompt);
        assert_eq!(Action::NameConfirm.mode(), ActionMode::NamePrompt);
        assert_eq!(Action::OpenNamePrompt.mode(), ActionMode::Command);
        assert_eq!(Action::Idle.mode(), ActionMode::Command);
        assert_eq!(Action::Interrupt.mode(), ActionMode::Command);
    }
}
