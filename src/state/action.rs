use crate::state::text_edit::TextKey;

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
    ToggleSiblingsRounded,
    ToggleFill,
    ToggleRounded,
    Quit,
    Idle,
    Interrupt,
    Digit(u8),
    CancelCount,
    Commit,
    CommitAndAddChild,
    InsertKey(TextKey),
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
    NamePrompt,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ActionSpec {
    pub(crate) mode: ActionMode,
    pub(crate) undoable: bool,
    pub(crate) min_depth: usize,
}

impl Action {
    pub(crate) fn spec(&self) -> ActionSpec {
        match self {
            Action::Undo | Action::Quit => ActionSpec {
                mode: ActionMode::Command,
                undoable: false,
                min_depth: 0,
            },
            Action::NewBox | Action::Paste => ActionSpec {
                mode: ActionMode::Command,
                undoable: true,
                min_depth: 0,
            },
            Action::NewSibling
            | Action::Delete
            | Action::EditLabel
            | Action::RenameLabel
            | Action::CycleColour
            | Action::CycleSiblingsColour
            | Action::ToggleFill
            | Action::ToggleSiblingsFill
            | Action::ToggleRounded
            | Action::ToggleSiblingsRounded => ActionSpec {
                mode: ActionMode::Command,
                undoable: true,
                min_depth: 1,
            },
            Action::SelectChild | Action::SelectNext | Action::SelectPrevious => ActionSpec {
                mode: ActionMode::Command,
                undoable: false,
                min_depth: 1,
            },
            Action::SelectParent => ActionSpec {
                mode: ActionMode::Command,
                undoable: false,
                min_depth: 2,
            },
            Action::Idle
            | Action::Interrupt
            | Action::OpenNamePrompt
            | Action::Digit(_)
            | Action::CancelCount => ActionSpec {
                mode: ActionMode::Command,
                undoable: false,
                min_depth: 0,
            },
            Action::CommitAndAddChild => ActionSpec {
                mode: ActionMode::Insert,
                undoable: true,
                min_depth: 0,
            },
            Action::Commit | Action::InsertKey(_) => ActionSpec {
                mode: ActionMode::Insert,
                undoable: false,
                min_depth: 0,
            },
            Action::NameAppend(_)
            | Action::NameBackspace
            | Action::NameConfirm
            | Action::NameCancel => ActionSpec {
                mode: ActionMode::NamePrompt,
                undoable: false,
                min_depth: 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_map_to_their_mode() {
        assert_eq!(
            Action::InsertKey(TextKey::Char('a')).spec().mode,
            ActionMode::Insert
        );
        assert_eq!(Action::NameConfirm.spec().mode, ActionMode::NamePrompt);
        assert_eq!(Action::OpenNamePrompt.spec().mode, ActionMode::Command);
        assert_eq!(Action::Idle.spec().mode, ActionMode::Command);
        assert_eq!(Action::Interrupt.spec().mode, ActionMode::Command);
    }
}
