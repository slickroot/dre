use crate::state::action::Action;
use crate::state::{KeyBinding, Mode, State};

pub(crate) fn parse(state: &State, key: &str) -> Option<Action> {
    match &state.mode {
        Mode::Command => command_parse(key),
        Mode::Insert => insert_parse(key),
        Mode::SavePrompt { .. } => save_prompt_parse(key),
    }
}

fn command_parse(key: &str) -> Option<Action> {
    if key.len() == 1 && key.as_bytes()[0].is_ascii_digit() {
        return Some(Action::Digit(key.as_bytes()[0] - b'0'));
    }
    Some(match key {
        "u" => Action::Undo,
        "b" => Action::NewBox,
        "s" => Action::NewSibling,
        "d" => Action::Delete,
        "p" => Action::Paste,
        "h" => Action::SelectParent,
        "l" => Action::SelectChild,
        "j" => Action::SelectNext,
        "k" => Action::SelectPrevious,
        "i" => Action::EditLabel,
        "I" => Action::RenameLabel,
        "c" => Action::CycleColour,
        "C" => Action::CycleSiblingsColour,
        "F" => Action::ToggleSiblingsFill,
        "f" => Action::ToggleFill,
        "r" => Action::ToggleRounded,
        "q" => Action::Quit,
        _ => Action::CancelCount,
    })
}

fn insert_parse(key: &str) -> Option<Action> {
    match key {
        "\x1b" => Some(Action::Commit),
        "\r" => Some(Action::CommitAndAddChild),
        "\x7f" => Some(Action::InsertBackspace),
        _ => match key.chars().next() {
            Some(c) if ('\x20'..='\x7e').contains(&c) => Some(Action::InsertAppend(c)),
            _ => None,
        },
    }
}

fn save_prompt_parse(key: &str) -> Option<Action> {
    match key {
        "\r" => Some(Action::Confirm),
        "\x1b" => Some(Action::Cancel),
        "\x7f" => Some(Action::SavePromptBackspace),
        _ => match key.chars().next() {
            Some(c) if ('\x20'..='\x7e').contains(&c) => Some(Action::SavePromptAppend(c)),
            _ => None,
        },
    }
}

#[allow(dead_code)]
pub(crate) const COMMAND_KEYMAP: &[KeyBinding<Action>] = &[
    KeyBinding {
        keys: &["u"],
        command: Action::Undo,
        description: "Undo the last change",
    },
    KeyBinding {
        keys: &["b"],
        command: Action::NewBox,
        description: "Add a child box",
    },
    KeyBinding {
        keys: &["s"],
        command: Action::NewSibling,
        description: "Add a sibling box",
    },
    KeyBinding {
        keys: &["d"],
        command: Action::Delete,
        description: "Delete the selected box and its descendants",
    },
    KeyBinding {
        keys: &["p"],
        command: Action::Paste,
        description: "Paste the cut box and its descendants as the last child of the selected box",
    },
    KeyBinding {
        keys: &["h"],
        command: Action::SelectParent,
        description: "Select the parent box",
    },
    KeyBinding {
        keys: &["l"],
        command: Action::SelectChild,
        description: "Select the first child box",
    },
    KeyBinding {
        keys: &["j"],
        command: Action::SelectNext,
        description: "Select the next sibling",
    },
    KeyBinding {
        keys: &["k"],
        command: Action::SelectPrevious,
        description: "Select the previous sibling",
    },
    KeyBinding {
        keys: &["i"],
        command: Action::EditLabel,
        description: "Edit the selected box's label",
    },
    KeyBinding {
        keys: &["I"],
        command: Action::RenameLabel,
        description: "Rename the selected box's label",
    },
    KeyBinding {
        keys: &["c"],
        command: Action::CycleColour,
        description: "Cycle the box's colour",
    },
    KeyBinding {
        keys: &["C"],
        command: Action::CycleSiblingsColour,
        description: "Cycle the colour of every sibling",
    },
    KeyBinding {
        keys: &["f"],
        command: Action::ToggleFill,
        description: "Toggle the box's fill",
    },
    KeyBinding {
        keys: &["F"],
        command: Action::ToggleSiblingsFill,
        description: "Toggle the fill of every sibling",
    },
    KeyBinding {
        keys: &["r"],
        command: Action::ToggleRounded,
        description: "Toggle rounded corners",
    },
    KeyBinding {
        keys: &["q"],
        command: Action::Quit,
        description: "Save and quit (or choose where to save)",
    },
];

#[allow(dead_code)]
pub(crate) const INSERT_KEYMAP: &[KeyBinding<Action>] = &[
    KeyBinding {
        keys: &["Enter"],
        command: Action::CommitAndAddChild,
        description: "Finish the box and add a child box",
    },
    KeyBinding {
        keys: &["Esc"],
        command: Action::Commit,
        description: "Switch to command mode",
    },
    KeyBinding {
        keys: &["Backspace"],
        command: Action::InsertBackspace,
        description: "Remove the last character",
    },
];

#[allow(dead_code)]
pub(crate) fn command_keymap_markdown() -> String {
    let mut out = String::from("| Key | Description |\n| --- | --- |\n");
    for binding in COMMAND_KEYMAP {
        let keys: Vec<String> = binding.keys.iter().map(|k| format!("`{k}`")).collect();
        out.push_str(&format!(
            "| {} | {} |\n",
            keys.join(", "),
            binding.description
        ));
    }
    out
}

#[allow(dead_code)]
pub(crate) fn insert_keymap_markdown() -> String {
    let mut out = String::from("| Key | Description |\n| --- | --- |\n");
    for binding in INSERT_KEYMAP {
        let keys: Vec<String> = binding.keys.iter().map(|k| format!("`{k}`")).collect();
        out.push_str(&format!(
            "| {} | {} |\n",
            keys.join(", "),
            binding.description
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::new_state;

    fn key_state(mode: Mode) -> State {
        new_state(vec![], mode, None)
    }

    #[test]
    fn parse_dispatches_on_the_mode() {
        assert_eq!(parse(&key_state(Mode::Command), "b"), Some(Action::NewBox));
        assert_eq!(
            parse(&key_state(Mode::Insert), "b"),
            Some(Action::InsertAppend('b'))
        );
        let prompt = Mode::SavePrompt {
            filename: String::new(),
        };
        assert_eq!(
            parse(&key_state(prompt), "b"),
            Some(Action::SavePromptAppend('b'))
        );
    }

    #[test]
    fn readme_keymap_table_stays_in_sync() {
        let markdown = format!("\n\n{}\n", command_keymap_markdown());
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

    #[test]
    fn parse_maps_known_keys_to_their_commands() {
        assert_eq!(command_parse("u"), Some(Action::Undo));
        assert_eq!(command_parse("b"), Some(Action::NewBox));
        assert_eq!(command_parse("s"), Some(Action::NewSibling));
        assert_eq!(command_parse("d"), Some(Action::Delete));
        assert_eq!(command_parse("h"), Some(Action::SelectParent));
        assert_eq!(command_parse("l"), Some(Action::SelectChild));
        assert_eq!(command_parse("j"), Some(Action::SelectNext));
        assert_eq!(command_parse("k"), Some(Action::SelectPrevious));
        assert_eq!(command_parse("i"), Some(Action::EditLabel));
        assert_eq!(command_parse("I"), Some(Action::RenameLabel));
        assert_eq!(command_parse("c"), Some(Action::CycleColour));
        assert_eq!(command_parse("C"), Some(Action::CycleSiblingsColour));
        assert_eq!(command_parse("F"), Some(Action::ToggleSiblingsFill));
        assert_eq!(command_parse("f"), Some(Action::ToggleFill));
        assert_eq!(command_parse("r"), Some(Action::ToggleRounded));
        assert_eq!(command_parse("q"), Some(Action::Quit));
    }

    #[test]
    fn parse_cancels_the_count_for_an_unknown_key() {
        for key in ["x", "\x1b", "é"] {
            assert_eq!(command_parse(key), Some(Action::CancelCount));
        }
    }

    #[test]
    fn parse_maps_each_digit_key_to_its_digit() {
        for digit in 0..=9u8 {
            assert_eq!(
                command_parse(&digit.to_string()),
                Some(Action::Digit(digit))
            );
        }
    }

    #[test]
    fn command_keymap_agrees_with_command_parse() {
        let bound: Vec<String> = COMMAND_KEYMAP
            .iter()
            .flat_map(|binding| binding.keys)
            .map(|key| key.to_string())
            .collect();

        for binding in COMMAND_KEYMAP {
            for key in binding.keys {
                assert_eq!(command_parse(key), Some(binding.command));
            }
        }

        for ch in (b'a'..=b'z').chain(b'A'..=b'Z') {
            let key = (ch as char).to_string();
            if !bound.contains(&key) {
                assert_eq!(command_parse(&key), Some(Action::CancelCount));
            }
        }

        let mut unique = bound.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), bound.len());
    }

    #[test]
    fn p_parses_to_paste() {
        assert_eq!(command_parse("p"), Some(Action::Paste));
    }

    #[test]
    fn readme_insert_keymap_table_stays_in_sync() {
        let markdown = format!("\n\n{}\n", insert_keymap_markdown());
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = std::path::Path::new(manifest_dir).join("README.md");
        let readme = std::fs::read_to_string(&path).unwrap();
        let start_marker = "<!-- insert-keymap:start -->";
        let end_marker = "<!-- insert-keymap:end -->";
        let start = readme
            .find(start_marker)
            .expect("missing <!-- insert-keymap:start --> in README.md");
        let end = readme
            .find(end_marker)
            .expect("missing <!-- insert-keymap:end --> in README.md");
        let start_after = start + start_marker.len();
        let between = &readme[start_after..end];
        if std::env::var("UPDATE_README").unwrap_or_default() == "1" {
            let new_readme = format!("{}{}{}", &readme[..start_after], markdown, &readme[end..]);
            std::fs::write(&path, new_readme).unwrap();
        } else {
            assert_eq!(
                between, markdown,
                "README.md insert keymap table is out of date. Run UPDATE_README=1 cargo test to regenerate."
            );
        }
    }

    #[test]
    fn parse_maps_escape_to_commit() {
        assert_eq!(insert_parse("\x1b"), Some(Action::Commit));
    }

    #[test]
    fn parse_maps_delete_to_backspace() {
        assert_eq!(insert_parse("\x7f"), Some(Action::InsertBackspace));
    }

    #[test]
    fn parse_maps_the_lower_printable_boundary_to_append() {
        assert_eq!(insert_parse("\x20"), Some(Action::InsertAppend('\x20')));
    }

    #[test]
    fn parse_maps_the_upper_printable_boundary_to_append() {
        assert_eq!(insert_parse("\x7e"), Some(Action::InsertAppend('\x7e')));
    }

    #[test]
    fn parse_returns_nothing_just_outside_the_printable_range() {
        assert_eq!(insert_parse("\x1f"), None);
        assert_eq!(insert_parse("\x01"), None);
        assert_eq!(insert_parse("é"), None);
    }

    #[test]
    fn parse_maps_enter_to_commit_and_add_child() {
        assert_eq!(insert_parse("\r"), Some(Action::CommitAndAddChild));
    }

    #[test]
    fn parse_does_not_map_newline_or_backspace_control_to_commit_and_add_child() {
        assert_eq!(insert_parse("\n"), None);
        assert_eq!(insert_parse("\x08"), None);
    }

    #[test]
    fn insert_keymap_agrees_with_insert_parse() {
        let raw_for = |display: &str| match display {
            "Enter" => "\r",
            "Esc" => "\x1b",
            "Backspace" => "\x7f",
            _ => panic!("unknown display key {display}"),
        };
        let display_keys: Vec<&str> = INSERT_KEYMAP
            .iter()
            .flat_map(|binding| binding.keys)
            .copied()
            .collect();
        assert_eq!(display_keys, vec!["Enter", "Esc", "Backspace"]);
        let mut unique = display_keys.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), display_keys.len());
        for binding in INSERT_KEYMAP {
            for display in binding.keys {
                assert_eq!(insert_parse(raw_for(display)), Some(binding.command));
            }
        }
    }
}
