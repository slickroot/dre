#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub(crate) enum Mode {
    #[default]
    Command,
    Insert {
        cursor: usize,
    },
    SavePrompt {
        filename: String,
    },
    NamePrompt {
        name: String,
    },
}
