#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub(crate) enum Mode {
    #[default]
    Command,
    Insert,
    SavePrompt {
        filename: String,
    },
    NamePrompt {
        name: String,
    },
}
