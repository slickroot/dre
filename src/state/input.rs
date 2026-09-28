use crate::state::action::Action;
use crate::state::text_edit::TextKey;
use crate::state::{Mode, State};

pub(crate) const INTERRUPT: &str = "\x03";

#[derive(Clone, Copy, PartialEq, Eq)]
enum KeyMatch {
    Exact(&'static str),
    Digit,
    Printable,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum KeyAction {
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
    ToggleSiblingsRounded,
    Quit,
    OpenNamePrompt,
    Interrupt,
    Digit,
    Commit,
    CommitAndAddChild,
    InsertBackspace,
    InsertLeft,
    InsertRight,
    InsertPrintable,
    NameConfirm,
    NameCancel,
    NameBackspace,
    NamePrintable,
}

impl KeyAction {
    fn action(self, key: &str) -> Action {
        match self {
            Self::Undo => Action::Undo,
            Self::NewBox => Action::NewBox,
            Self::NewSibling => Action::NewSibling,
            Self::Delete => Action::Delete,
            Self::Paste => Action::Paste,
            Self::SelectParent => Action::SelectParent,
            Self::SelectChild => Action::SelectChild,
            Self::SelectNext => Action::SelectNext,
            Self::SelectPrevious => Action::SelectPrevious,
            Self::EditLabel => Action::EditLabel,
            Self::RenameLabel => Action::RenameLabel,
            Self::CycleColour => Action::CycleColour,
            Self::CycleSiblingsColour => Action::CycleSiblingsColour,
            Self::ToggleSiblingsFill => Action::ToggleSiblingsFill,
            Self::ToggleFill => Action::ToggleFill,
            Self::ToggleRounded => Action::ToggleRounded,
            Self::ToggleSiblingsRounded => Action::ToggleSiblingsRounded,
            Self::Quit => Action::Quit,
            Self::OpenNamePrompt => Action::OpenNamePrompt,
            Self::Interrupt => Action::Interrupt,
            Self::Digit => Action::Digit(key.as_bytes()[0] - b'0'),
            Self::Commit => Action::Commit,
            Self::CommitAndAddChild => Action::CommitAndAddChild,
            Self::InsertBackspace => Action::InsertKey(TextKey::Backspace),
            Self::InsertLeft => Action::InsertKey(TextKey::Left),
            Self::InsertRight => Action::InsertKey(TextKey::Right),
            Self::InsertPrintable => Action::InsertKey(TextKey::Char(printable(key).unwrap())),
            Self::NameConfirm => Action::NameConfirm,
            Self::NameCancel => Action::NameCancel,
            Self::NameBackspace => Action::NameBackspace,
            Self::NamePrintable => Action::NameAppend(printable(key).unwrap()),
        }
    }
}

#[allow(dead_code)]
struct KeyBinding {
    mode: Mode,
    matcher: KeyMatch,
    display: &'static str,
    action: KeyAction,
    description: &'static str,
}

const COMMAND: Mode = Mode::Command;

const KEYMAP: &[KeyBinding] = &[
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("u"),
        display: "u",
        action: KeyAction::Undo,
        description: "Undo the last change",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("b"),
        display: "b",
        action: KeyAction::NewBox,
        description: "Add a child box",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("s"),
        display: "s",
        action: KeyAction::NewSibling,
        description: "Add a sibling box",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("d"),
        display: "d",
        action: KeyAction::Delete,
        description: "Delete the selected box and its descendants",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("p"),
        display: "p",
        action: KeyAction::Paste,
        description: "Paste the cut box and its descendants as the last child of the selected box",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("h"),
        display: "h",
        action: KeyAction::SelectParent,
        description: "Select the parent box",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("l"),
        display: "l",
        action: KeyAction::SelectChild,
        description: "Select the first child box",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("j"),
        display: "j",
        action: KeyAction::SelectNext,
        description: "Select the next sibling",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("k"),
        display: "k",
        action: KeyAction::SelectPrevious,
        description: "Select the previous sibling",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("i"),
        display: "i",
        action: KeyAction::EditLabel,
        description: "Edit the selected box's label",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("I"),
        display: "I",
        action: KeyAction::RenameLabel,
        description: "Rename the selected box's label",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("c"),
        display: "c",
        action: KeyAction::CycleColour,
        description: "Cycle the box's colour",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("C"),
        display: "C",
        action: KeyAction::CycleSiblingsColour,
        description: "Cycle the colour of every sibling",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("F"),
        display: "F",
        action: KeyAction::ToggleSiblingsFill,
        description: "Toggle the fill of every sibling",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("f"),
        display: "f",
        action: KeyAction::ToggleFill,
        description: "Toggle the box's fill",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("r"),
        display: "r",
        action: KeyAction::ToggleRounded,
        description: "Toggle rounded corners",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("R"),
        display: "R",
        action: KeyAction::ToggleSiblingsRounded,
        description: "Toggle rounded corners of every sibling",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("q"),
        display: "q",
        action: KeyAction::Quit,
        description: "Save and quit (or choose where to save)",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact("n"),
        display: "n",
        action: KeyAction::OpenNamePrompt,
        description: "Name the diagram",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Digit,
        display: "0–9",
        action: KeyAction::Digit,
        description: "Build a count prefix",
    },
    KeyBinding {
        mode: COMMAND,
        matcher: KeyMatch::Exact(INTERRUPT),
        display: "Ctrl-C",
        action: KeyAction::Interrupt,
        description: "Quit without saving",
    },
    KeyBinding {
        mode: Mode::Insert { cursor: 0 },
        matcher: KeyMatch::Exact("\r"),
        display: "Enter",
        action: KeyAction::CommitAndAddChild,
        description: "Finish the box and add a child box",
    },
    KeyBinding {
        mode: Mode::Insert { cursor: 0 },
        matcher: KeyMatch::Exact("\x1b"),
        display: "Esc",
        action: KeyAction::Commit,
        description: "Switch to command mode",
    },
    KeyBinding {
        mode: Mode::Insert { cursor: 0 },
        matcher: KeyMatch::Exact("\x7f"),
        display: "Backspace",
        action: KeyAction::InsertBackspace,
        description: "Remove the last character",
    },
    KeyBinding {
        mode: Mode::Insert { cursor: 0 },
        matcher: KeyMatch::Exact("\x1b[D"),
        display: "←",
        action: KeyAction::InsertLeft,
        description: "Move the cursor left",
    },
    KeyBinding {
        mode: Mode::Insert { cursor: 0 },
        matcher: KeyMatch::Exact("\x1b[C"),
        display: "→",
        action: KeyAction::InsertRight,
        description: "Move the cursor right",
    },
    KeyBinding {
        mode: Mode::Insert { cursor: 0 },
        matcher: KeyMatch::Printable,
        display: "Printable",
        action: KeyAction::InsertPrintable,
        description: "Append a printable character",
    },
    KeyBinding {
        mode: Mode::NamePrompt {
            name: String::new(),
            quits: false,
        },
        matcher: KeyMatch::Exact("\r"),
        display: "Enter",
        action: KeyAction::NameConfirm,
        description: "Save the name",
    },
    KeyBinding {
        mode: Mode::NamePrompt {
            name: String::new(),
            quits: false,
        },
        matcher: KeyMatch::Exact("\x1b"),
        display: "Esc",
        action: KeyAction::NameCancel,
        description: "Cancel naming",
    },
    KeyBinding {
        mode: Mode::NamePrompt {
            name: String::new(),
            quits: false,
        },
        matcher: KeyMatch::Exact("\x7f"),
        display: "Backspace",
        action: KeyAction::NameBackspace,
        description: "Remove a character from the name",
    },
    KeyBinding {
        mode: Mode::NamePrompt {
            name: String::new(),
            quits: false,
        },
        matcher: KeyMatch::Printable,
        display: "Printable",
        action: KeyAction::NamePrintable,
        description: "Append a character to the name",
    },
];

fn same_mode(binding: &KeyBinding, mode: &Mode) -> bool {
    matches!(
        (&binding.mode, mode),
        (Mode::Command, Mode::Command)
            | (Mode::Insert { .. }, Mode::Insert { .. })
            | (Mode::NamePrompt { .. }, Mode::NamePrompt { .. })
    )
}

fn matches_key(matcher: KeyMatch, key: &str) -> bool {
    match matcher {
        KeyMatch::Exact(expected) => key == expected,
        KeyMatch::Digit => key.len() == 1 && key.as_bytes()[0].is_ascii_digit(),
        KeyMatch::Printable => printable(key).is_some(),
    }
}

pub(crate) fn parse(state: &State, key: &str) -> Option<Action> {
    let exact = KEYMAP.iter().find(|binding| {
        same_mode(binding, &state.mode)
            && matches!(binding.matcher, KeyMatch::Exact(_))
            && matches_key(binding.matcher, key)
    });
    let binding = exact.or_else(|| {
        KEYMAP.iter().find(|binding| {
            same_mode(binding, &state.mode)
                && !matches!(binding.matcher, KeyMatch::Exact(_))
                && matches_key(binding.matcher, key)
        })
    });
    match binding {
        Some(binding) => Some(binding.action.action(key)),
        None if matches!(state.mode, Mode::Command) => Some(Action::CancelCount),
        None => None,
    }
}

fn printable(key: &str) -> Option<char> {
    key.chars().next().filter(|c| ('\x20'..='\x7e').contains(c))
}

#[cfg(test)]
fn keymap_markdown() -> String {
    let mut out = String::from("| Mode | Key | Description |\n| --- | --- | --- |\n");
    for binding in KEYMAP {
        let mode = match &binding.mode {
            Mode::Command => "Command",
            Mode::Insert { .. } => "Insert",
            Mode::NamePrompt { .. } => "Name prompt",
        };
        out.push_str(&format!(
            "| {mode} | `{}` | {} |\n",
            binding.display, binding.description
        ));
    }
    out
}

#[cfg(test)]
fn action_key(action: &Action) -> KeyAction {
    match action {
        Action::Undo => KeyAction::Undo,
        Action::NewBox => KeyAction::NewBox,
        Action::NewSibling => KeyAction::NewSibling,
        Action::Delete => KeyAction::Delete,
        Action::Paste => KeyAction::Paste,
        Action::SelectParent => KeyAction::SelectParent,
        Action::SelectChild => KeyAction::SelectChild,
        Action::SelectNext => KeyAction::SelectNext,
        Action::SelectPrevious => KeyAction::SelectPrevious,
        Action::EditLabel => KeyAction::EditLabel,
        Action::RenameLabel => KeyAction::RenameLabel,
        Action::CycleColour => KeyAction::CycleColour,
        Action::CycleSiblingsColour => KeyAction::CycleSiblingsColour,
        Action::ToggleSiblingsFill => KeyAction::ToggleSiblingsFill,
        Action::ToggleFill => KeyAction::ToggleFill,
        Action::ToggleRounded => KeyAction::ToggleRounded,
        Action::ToggleSiblingsRounded => KeyAction::ToggleSiblingsRounded,
        Action::Quit => KeyAction::Quit,
        Action::Interrupt => KeyAction::Interrupt,
        Action::Digit(_) => KeyAction::Digit,
        Action::CancelCount => panic!("CancelCount is not a binding"),
        Action::Commit => KeyAction::Commit,
        Action::CommitAndAddChild => KeyAction::CommitAndAddChild,
        Action::InsertKey(TextKey::Backspace) => KeyAction::InsertBackspace,
        Action::InsertKey(TextKey::Left) => KeyAction::InsertLeft,
        Action::InsertKey(TextKey::Right) => KeyAction::InsertRight,
        Action::InsertKey(TextKey::Char(_)) => KeyAction::InsertPrintable,
        Action::OpenNamePrompt => KeyAction::OpenNamePrompt,
        Action::NameAppend(_) => KeyAction::NamePrintable,
        Action::NameBackspace => KeyAction::NameBackspace,
        Action::NameConfirm => KeyAction::NameConfirm,
        Action::NameCancel => KeyAction::NameCancel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::new_state;

    fn key_state(mode: Mode) -> State {
        new_state(vec![], mode, None)
    }

    #[test]
    fn table_parses_each_mode_and_pattern() {
        assert_eq!(parse(&key_state(Mode::Command), "b"), Some(Action::NewBox));
        assert_eq!(
            parse(&key_state(Mode::Command), "7"),
            Some(Action::Digit(7))
        );
        assert_eq!(
            parse(&key_state(Mode::Insert { cursor: 0 }), "b"),
            Some(Action::InsertKey(TextKey::Char('b')))
        );
        assert_eq!(
            parse(
                &key_state(Mode::NamePrompt {
                    name: String::new(),
                    quits: false
                }),
                "b"
            ),
            Some(Action::NameAppend('b'))
        );
    }

    #[test]
    fn exact_matches_take_precedence_over_patterns() {
        assert_eq!(
            parse(&key_state(Mode::Command), INTERRUPT),
            Some(Action::Interrupt)
        );
        assert_eq!(
            parse(&key_state(Mode::Insert { cursor: 0 }), "\x1b"),
            Some(Action::Commit)
        );
    }

    #[test]
    fn unknown_command_keys_cancel_counts_and_other_unknown_keys_do_nothing() {
        assert_eq!(
            parse(&key_state(Mode::Command), "x"),
            Some(Action::CancelCount)
        );
        assert_eq!(parse(&key_state(Mode::Insert { cursor: 0 }), "\x01"), None);
        assert_eq!(
            parse(
                &key_state(Mode::NamePrompt {
                    name: String::new(),
                    quits: false
                }),
                "é"
            ),
            None
        );
    }

    #[test]
    fn every_user_action_has_one_binding_and_exact_keys_are_unique_per_mode() {
        for action in [
            Action::Undo,
            Action::NewBox,
            Action::NewSibling,
            Action::Delete,
            Action::Paste,
            Action::SelectParent,
            Action::SelectChild,
            Action::SelectNext,
            Action::SelectPrevious,
            Action::EditLabel,
            Action::RenameLabel,
            Action::CycleColour,
            Action::CycleSiblingsColour,
            Action::ToggleSiblingsFill,
            Action::ToggleFill,
            Action::ToggleRounded,
            Action::ToggleSiblingsRounded,
            Action::Quit,
            Action::Interrupt,
            Action::Digit(1),
            Action::Commit,
            Action::CommitAndAddChild,
            Action::InsertKey(TextKey::Backspace),
            Action::InsertKey(TextKey::Left),
            Action::InsertKey(TextKey::Right),
            Action::InsertKey(TextKey::Char('a')),
            Action::OpenNamePrompt,
            Action::NameAppend('a'),
            Action::NameBackspace,
            Action::NameConfirm,
            Action::NameCancel,
        ] {
            assert_eq!(
                KEYMAP
                    .iter()
                    .filter(|binding| binding.action == action_key(&action))
                    .count(),
                1
            );
        }
        for (index, binding) in KEYMAP.iter().enumerate() {
            for other in &KEYMAP[index + 1..] {
                assert!(
                    !(same_mode(binding, &other.mode)
                        && binding.matcher == other.matcher
                        && matches!(binding.matcher, KeyMatch::Exact(_)))
                );
            }
        }
    }

    #[test]
    fn readme_keymap_table_stays_in_sync() {
        let markdown = format!("\n\n{}\n", keymap_markdown());
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");
        let readme = std::fs::read_to_string(&path).unwrap();
        let start = readme.find("<!-- keymap:start -->").unwrap();
        let end = readme.find("<!-- keymap:end -->").unwrap();
        assert_eq!(
            &readme[start + "<!-- keymap:start -->".len()..end],
            markdown
        );
    }
}
