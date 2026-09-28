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
    Idle,
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
    pub(crate) fn undoable(&self) -> bool {
        match self {
            CommandAction::NewBox
            | CommandAction::NewSibling
            | CommandAction::Delete
            | CommandAction::Paste
            | CommandAction::EditLabel
            | CommandAction::RenameLabel
            | CommandAction::CycleColour
            | CommandAction::CycleSiblingsColour
            | CommandAction::ToggleSiblingsFill
            | CommandAction::ToggleSiblingsRounded
            | CommandAction::ToggleFill
            | CommandAction::ToggleRounded => true,
            CommandAction::Undo
            | CommandAction::SelectParent
            | CommandAction::SelectChild
            | CommandAction::SelectNext
            | CommandAction::SelectPrevious
            | CommandAction::Quit
            | CommandAction::Idle
            | CommandAction::Interrupt
            | CommandAction::OpenNamePrompt
            | CommandAction::Digit(_)
            | CommandAction::CancelCount => false,
        }
    }

    pub(crate) fn min_depth(&self) -> usize {
        match self {
            CommandAction::Undo
            | CommandAction::NewBox
            | CommandAction::Paste
            | CommandAction::Quit => 0,
            CommandAction::SelectParent => 2,
            CommandAction::NewSibling
            | CommandAction::SelectChild
            | CommandAction::SelectNext
            | CommandAction::SelectPrevious
            | CommandAction::EditLabel
            | CommandAction::RenameLabel
            | CommandAction::CycleColour
            | CommandAction::CycleSiblingsColour
            | CommandAction::ToggleSiblingsFill
            | CommandAction::ToggleSiblingsRounded
            | CommandAction::ToggleFill
            | CommandAction::ToggleRounded
            | CommandAction::Delete => 1,
            CommandAction::Idle
            | CommandAction::Interrupt
            | CommandAction::OpenNamePrompt
            | CommandAction::Digit(_)
            | CommandAction::CancelCount => 0,
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
