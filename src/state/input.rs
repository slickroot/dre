use crate::state::action::{
    Action, ImmediateCommand, InsertAction, NamePromptAction, SelectionCommand,
};
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
            Self::Undo => Action::Selection(SelectionCommand::Undo),
            Self::NewBox => Action::Selection(SelectionCommand::NewBox),
            Self::NewSibling => Action::Selection(SelectionCommand::NewSibling),
            Self::Delete => Action::Selection(SelectionCommand::Delete),
            Self::Paste => Action::Selection(SelectionCommand::Paste),
            Self::SelectParent => Action::Selection(SelectionCommand::SelectParent),
            Self::SelectChild => Action::Selection(SelectionCommand::SelectChild),
            Self::SelectNext => Action::Selection(SelectionCommand::SelectNext),
            Self::SelectPrevious => Action::Selection(SelectionCommand::SelectPrevious),
            Self::EditLabel => Action::Selection(SelectionCommand::EditLabel),
            Self::RenameLabel => Action::Selection(SelectionCommand::RenameLabel),
            Self::CycleColour => Action::Selection(SelectionCommand::CycleColour),
            Self::CycleSiblingsColour => Action::Selection(SelectionCommand::CycleSiblingsColour),
            Self::ToggleSiblingsFill => Action::Selection(SelectionCommand::ToggleSiblingsFill),
            Self::ToggleFill => Action::Selection(SelectionCommand::ToggleFill),
            Self::ToggleRounded => Action::Selection(SelectionCommand::ToggleRounded),
            Self::ToggleSiblingsRounded => {
                Action::Selection(SelectionCommand::ToggleSiblingsRounded)
            }
            Self::Quit => Action::Selection(SelectionCommand::Quit),
            Self::OpenNamePrompt => Action::Immediate(ImmediateCommand::OpenNamePrompt),
            Self::Interrupt => Action::Immediate(ImmediateCommand::Interrupt),
            Self::Digit => Action::Immediate(ImmediateCommand::Digit(key.as_bytes()[0] - b'0')),
            Self::Commit => Action::Insert(InsertAction::Commit),
            Self::CommitAndAddChild => Action::Insert(InsertAction::CommitAndAddChild),
            Self::InsertBackspace => Action::Insert(InsertAction::InsertKey(TextKey::Backspace)),
            Self::InsertLeft => Action::Insert(InsertAction::InsertKey(TextKey::Left)),
            Self::InsertRight => Action::Insert(InsertAction::InsertKey(TextKey::Right)),
            Self::InsertPrintable => Action::Insert(InsertAction::InsertKey(TextKey::Char(
                printable(key).unwrap(),
            ))),
            Self::NameConfirm => Action::NamePrompt(NamePromptAction::NameConfirm),
            Self::NameCancel => Action::NamePrompt(NamePromptAction::NameCancel),
            Self::NameBackspace => Action::NamePrompt(NamePromptAction::NameBackspace),
            Self::NamePrintable => {
                Action::NamePrompt(NamePromptAction::NameAppend(printable(key).unwrap()))
            }
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
        None if matches!(state.mode, Mode::Command) => {
            Some(Action::Immediate(ImmediateCommand::CancelCount))
        }
        None => None,
    }
}

fn printable(key: &str) -> Option<char> {
    key.chars().next().filter(|c| ('\x20'..='\x7e').contains(c))
}

#[allow(dead_code)]
pub(crate) fn command_label(state: &State, key: &str) -> Option<(String, &'static str)> {
    if matches!(state.mode, Mode::NamePrompt { .. }) {
        return None;
    }
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
    })?;
    let action = binding.action.action(key);
    if matches!(
        action,
        Action::Immediate(ImmediateCommand::Digit(_))
            | Action::Immediate(ImmediateCommand::CancelCount)
            | Action::Immediate(ImmediateCommand::Interrupt)
    ) {
        return None;
    }
    let key_text = if matches!(binding.matcher, KeyMatch::Printable) {
        key.to_string()
    } else {
        binding.display.to_string()
    };
    Some((key_text, binding.description))
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
        Action::Selection(SelectionCommand::Undo) => KeyAction::Undo,
        Action::Selection(SelectionCommand::NewBox) => KeyAction::NewBox,
        Action::Selection(SelectionCommand::NewSibling) => KeyAction::NewSibling,
        Action::Selection(SelectionCommand::Delete) => KeyAction::Delete,
        Action::Selection(SelectionCommand::Paste) => KeyAction::Paste,
        Action::Selection(SelectionCommand::SelectParent) => KeyAction::SelectParent,
        Action::Selection(SelectionCommand::SelectChild) => KeyAction::SelectChild,
        Action::Selection(SelectionCommand::SelectNext) => KeyAction::SelectNext,
        Action::Selection(SelectionCommand::SelectPrevious) => KeyAction::SelectPrevious,
        Action::Selection(SelectionCommand::EditLabel) => KeyAction::EditLabel,
        Action::Selection(SelectionCommand::RenameLabel) => KeyAction::RenameLabel,
        Action::Selection(SelectionCommand::CycleColour) => KeyAction::CycleColour,
        Action::Selection(SelectionCommand::CycleSiblingsColour) => KeyAction::CycleSiblingsColour,
        Action::Selection(SelectionCommand::ToggleSiblingsFill) => KeyAction::ToggleSiblingsFill,
        Action::Selection(SelectionCommand::ToggleFill) => KeyAction::ToggleFill,
        Action::Selection(SelectionCommand::ToggleRounded) => KeyAction::ToggleRounded,
        Action::Selection(SelectionCommand::ToggleSiblingsRounded) => {
            KeyAction::ToggleSiblingsRounded
        }
        Action::Selection(SelectionCommand::Quit) => KeyAction::Quit,
        Action::Immediate(ImmediateCommand::Interrupt) => KeyAction::Interrupt,
        Action::Immediate(ImmediateCommand::Digit(_)) => KeyAction::Digit,
        Action::Immediate(ImmediateCommand::CancelCount) => panic!("CancelCount is not a binding"),
        Action::Immediate(ImmediateCommand::OpenNamePrompt) => KeyAction::OpenNamePrompt,
        Action::Insert(InsertAction::Commit) => KeyAction::Commit,
        Action::Insert(InsertAction::CommitAndAddChild) => KeyAction::CommitAndAddChild,
        Action::Insert(InsertAction::InsertKey(TextKey::Backspace)) => KeyAction::InsertBackspace,
        Action::Insert(InsertAction::InsertKey(TextKey::Left)) => KeyAction::InsertLeft,
        Action::Insert(InsertAction::InsertKey(TextKey::Right)) => KeyAction::InsertRight,
        Action::Insert(InsertAction::InsertKey(TextKey::Char(_))) => KeyAction::InsertPrintable,
        Action::NamePrompt(NamePromptAction::NameAppend(_)) => KeyAction::NamePrintable,
        Action::NamePrompt(NamePromptAction::NameBackspace) => KeyAction::NameBackspace,
        Action::NamePrompt(NamePromptAction::NameConfirm) => KeyAction::NameConfirm,
        Action::NamePrompt(NamePromptAction::NameCancel) => KeyAction::NameCancel,
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
        assert_eq!(
            parse(&key_state(Mode::Command), "b"),
            Some(Action::Selection(SelectionCommand::NewBox))
        );
        assert_eq!(
            parse(&key_state(Mode::Command), "7"),
            Some(Action::Immediate(ImmediateCommand::Digit(7)))
        );
        assert_eq!(
            parse(&key_state(Mode::Insert { cursor: 0 }), "b"),
            Some(Action::Insert(InsertAction::InsertKey(TextKey::Char('b'))))
        );
        assert_eq!(
            parse(
                &key_state(Mode::NamePrompt {
                    name: String::new(),
                    quits: false
                }),
                "b"
            ),
            Some(Action::NamePrompt(NamePromptAction::NameAppend('b')))
        );
    }

    #[test]
    fn exact_matches_take_precedence_over_patterns() {
        assert_eq!(
            parse(&key_state(Mode::Command), INTERRUPT),
            Some(Action::Immediate(ImmediateCommand::Interrupt))
        );
        assert_eq!(
            parse(&key_state(Mode::Insert { cursor: 0 }), "\x1b"),
            Some(Action::Insert(InsertAction::Commit))
        );
    }

    #[test]
    fn unknown_command_keys_cancel_counts_and_other_unknown_keys_do_nothing() {
        assert_eq!(
            parse(&key_state(Mode::Command), "x"),
            Some(Action::Immediate(ImmediateCommand::CancelCount))
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
            Action::Selection(SelectionCommand::Undo),
            Action::Selection(SelectionCommand::NewBox),
            Action::Selection(SelectionCommand::NewSibling),
            Action::Selection(SelectionCommand::Delete),
            Action::Selection(SelectionCommand::Paste),
            Action::Selection(SelectionCommand::SelectParent),
            Action::Selection(SelectionCommand::SelectChild),
            Action::Selection(SelectionCommand::SelectNext),
            Action::Selection(SelectionCommand::SelectPrevious),
            Action::Selection(SelectionCommand::EditLabel),
            Action::Selection(SelectionCommand::RenameLabel),
            Action::Selection(SelectionCommand::CycleColour),
            Action::Selection(SelectionCommand::CycleSiblingsColour),
            Action::Selection(SelectionCommand::ToggleSiblingsFill),
            Action::Selection(SelectionCommand::ToggleFill),
            Action::Selection(SelectionCommand::ToggleRounded),
            Action::Selection(SelectionCommand::ToggleSiblingsRounded),
            Action::Selection(SelectionCommand::Quit),
            Action::Immediate(ImmediateCommand::Interrupt),
            Action::Immediate(ImmediateCommand::Digit(1)),
            Action::Insert(InsertAction::Commit),
            Action::Insert(InsertAction::CommitAndAddChild),
            Action::Insert(InsertAction::InsertKey(TextKey::Backspace)),
            Action::Insert(InsertAction::InsertKey(TextKey::Left)),
            Action::Insert(InsertAction::InsertKey(TextKey::Right)),
            Action::Insert(InsertAction::InsertKey(TextKey::Char('a'))),
            Action::Immediate(ImmediateCommand::OpenNamePrompt),
            Action::NamePrompt(NamePromptAction::NameAppend('a')),
            Action::NamePrompt(NamePromptAction::NameBackspace),
            Action::NamePrompt(NamePromptAction::NameConfirm),
            Action::NamePrompt(NamePromptAction::NameCancel),
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
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = std::path::Path::new(manifest_dir).join("README.md");
        let readme = std::fs::read_to_string(&path).unwrap();
        let start_marker = "<!-- keymap:start -->";
        let end_marker = "<!-- keymap:end -->";
        let start = readme
            .find(start_marker)
            .expect("missing <!-- keymap:start --> in README.md");
        let end = readme
            .find(end_marker)
            .expect("missing <!-- keymap:end --> in README.md");
        let start_after = start + start_marker.len();
        let between = &readme[start_after..end];
        if std::env::var("UPDATE_README").unwrap_or_default() == "1" {
            let new_readme = format!("{}{}{}", &readme[..start_after], markdown, &readme[end..]);
            std::fs::write(&path, new_readme).unwrap();
        } else {
            assert_eq!(
                between, markdown,
                "README.md keymap table is out of date. Run UPDATE_README=1 cargo test to regenerate."
            );
        }
    }

    fn binding_description(mode: &Mode, display: &str) -> &'static str {
        KEYMAP
            .iter()
            .find(|binding| same_mode(binding, mode) && binding.display == display)
            .expect("binding must exist")
            .description
    }

    #[test]
    fn command_label_returns_key_and_description_for_a_real_command() {
        let description = binding_description(&Mode::Command, "b");
        assert_eq!(
            command_label(&key_state(Mode::Command), "b"),
            Some(("b".to_string(), description))
        );
    }

    #[test]
    fn command_label_returns_none_for_an_unrecognized_command_key() {
        assert_eq!(command_label(&key_state(Mode::Command), "x"), None);
    }

    #[test]
    fn command_label_returns_none_for_a_digit_key() {
        assert_eq!(command_label(&key_state(Mode::Command), "7"), None);
    }

    #[test]
    fn command_label_returns_none_for_interrupt() {
        assert_eq!(command_label(&key_state(Mode::Command), INTERRUPT), None);
    }

    #[test]
    fn command_label_returns_the_typed_character_for_insert_printable() {
        let description = binding_description(&Mode::Insert { cursor: 0 }, "Printable");
        assert_eq!(
            command_label(&key_state(Mode::Insert { cursor: 0 }), "b"),
            Some(("b".to_string(), description))
        );
    }

    #[test]
    fn command_label_returns_the_binding_display_for_insert_non_printable() {
        let description = binding_description(&Mode::Insert { cursor: 0 }, "Esc");
        assert_eq!(
            command_label(&key_state(Mode::Insert { cursor: 0 }), "\x1b"),
            Some(("Esc".to_string(), description))
        );
    }

    #[test]
    fn command_label_returns_none_in_name_prompt_mode() {
        let state = key_state(Mode::NamePrompt {
            name: String::new(),
            quits: false,
        });
        assert_eq!(command_label(&state, "b"), None);
    }

    #[test]
    fn command_label_counts_non_undoable_selection_moves() {
        let description = binding_description(&Mode::Command, "j");
        assert_eq!(
            command_label(&key_state(Mode::Command), "j"),
            Some(("j".to_string(), description))
        );
    }

    #[test]
    fn command_label_counts_open_name_prompt() {
        let description = binding_description(&Mode::Command, "n");
        assert_eq!(
            command_label(&key_state(Mode::Command), "n"),
            Some(("n".to_string(), description))
        );
    }
}
