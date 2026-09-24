#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Action {
    Command(CommandAction),
    Insert(InsertAction),
    SavePrompt(SavePromptAction),
}

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
    ToggleFill,
    ToggleRounded,
    Quit,
    ScrollBy(i64),
    Digit(u8),
    CancelCount,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InsertAction {
    Commit,
    CommitAndAddChild,
    Backspace,
    Append(char),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SavePromptAction {
    Confirm,
    Cancel,
    Backspace,
    Append(char),
}
