#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub(crate) enum Mode {
    #[default]
    Command,
    Insert {
        cursor: usize,
    },
    NamePrompt {
        name: String,
        quits: bool,
    },
}
