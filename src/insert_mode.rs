use crate::diagram::at;
use crate::state::{
    add_child_box, drop_snapshot_if_unchanged, snapshot, KeyBinding, Mode, State, PAD,
};

fn drop_last_chars(s: &str, n: usize) -> String {
    let len = s.chars().count();
    s.chars().take(len.saturating_sub(n)).collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Command {
    Commit,
    CommitAndAddChild,
    Backspace,
    Append(char),
}

pub(crate) fn parse(key: &str) -> Option<Command> {
    match key {
        "\x1b" => Some(Command::Commit),
        "\r" => Some(Command::CommitAndAddChild),
        "\x7f" => Some(Command::Backspace),
        _ => match key.chars().next() {
            Some(c) if ('\x20'..='\x7e').contains(&c) => Some(Command::Append(c)),
            _ => None,
        },
    }
}

#[allow(dead_code)]
pub(crate) const INSERT_KEYMAP: &[KeyBinding<Command>] = &[
    KeyBinding {
        keys: &["Enter"],
        command: Command::CommitAndAddChild,
        description: "Finish the box and add a child box",
    },
    KeyBinding {
        keys: &["Esc"],
        command: Command::Commit,
        description: "Switch to command mode",
    },
    KeyBinding {
        keys: &["Backspace"],
        command: Command::Backspace,
        description: "Remove the last character",
    },
];

#[allow(dead_code)]
pub(crate) fn format_keymap_markdown() -> String {
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

pub(crate) fn reduce(mut state: State, command: Command) -> State {
    let Some(path) = &state.doc.selected else {
        return state;
    };
    let node = at(&mut state.doc.boxes, path);
    let label = node.label.clone();
    match command {
        Command::Commit => {
            node.label = drop_last_chars(&label, 1);
            state.mode = Mode::Command;
            drop_snapshot_if_unchanged(state)
        }
        Command::CommitAndAddChild => {
            node.label = drop_last_chars(&label, 1);
            let selected = state.doc.selected.clone();
            add_child_box(snapshot(state), selected)
        }
        Command::Backspace => {
            node.label = format!("{}{PAD}", drop_last_chars(&label, 2));
            state
        }
        Command::Append(c) => {
            node.label = format!("{}{c}{PAD}", drop_last_chars(&label, 1));
            state
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{node, node_with_children, Path};
    use crate::state::{handle_key, new_state};

    #[test]
    fn parse_maps_escape_to_commit() {
        assert_eq!(parse("\x1b"), Some(Command::Commit));
    }

    #[test]
    fn parse_maps_delete_to_backspace() {
        assert_eq!(parse("\x7f"), Some(Command::Backspace));
    }

    #[test]
    fn parse_maps_the_lower_printable_boundary_to_append() {
        assert_eq!(parse("\x20"), Some(Command::Append('\x20')));
    }

    #[test]
    fn parse_maps_the_upper_printable_boundary_to_append() {
        assert_eq!(parse("\x7e"), Some(Command::Append('\x7e')));
    }

    #[test]
    fn parse_returns_nothing_just_outside_the_printable_range() {
        assert_eq!(parse("\x1f"), None);
        assert_eq!(parse("\x01"), None);
        assert_eq!(parse("é"), None);
    }

    #[test]
    fn parse_maps_enter_to_commit_and_add_child() {
        assert_eq!(parse("\r"), Some(Command::CommitAndAddChild));
    }

    #[test]
    fn parse_does_not_map_newline_or_backspace_control_to_commit_and_add_child() {
        assert_eq!(parse("\n"), None);
        assert_eq!(parse("\x08"), None);
    }

    #[test]
    fn enter_finishes_the_box_and_adds_an_empty_child_ready_for_typing() {
        let state = new_state(
            vec![node(&format!("hi{PAD}"))],
            Mode::Insert,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "\r");
        assert_eq!(
            result.doc.boxes,
            vec![node_with_children("hi", vec![node(PAD)])]
        );
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![0],
                index: 0
            })
        );
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn enter_on_an_empty_label_still_creates_an_empty_child() {
        let state = new_state(
            vec![node(PAD)],
            Mode::Insert,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "\r");
        assert_eq!(
            result.doc.boxes,
            vec![node_with_children("", vec![node(PAD)])]
        );
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![0],
                index: 0
            })
        );
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn u_after_enter_and_esc_reverts_the_child_and_keeps_the_label() {
        let state = new_state(
            vec![node(&format!("hi{PAD}"))],
            Mode::Insert,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let state = handle_key(state, "\r");
        let state = handle_key(state, "\x1b");
        let result = handle_key(state, "u");
        assert_eq!(result.doc.boxes, vec![node("hi")]);
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
        assert_eq!(result.mode, Mode::Command);
    }

    #[test]
    fn repeated_enter_drills_deeper_creating_a_child_then_a_grandchild() {
        let state = new_state(
            vec![node(&format!("a{PAD}"))],
            Mode::Insert,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let state = handle_key(state, "\r");
        let result = handle_key(state, "\r");
        assert_eq!(
            result.doc.boxes,
            vec![node_with_children(
                "a",
                vec![node_with_children("", vec![node(PAD)])]
            )]
        );
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![0, 0],
                index: 0
            })
        );
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn h_in_insert_mode_types_the_letter_h() {
        let state = new_state(
            vec![node(&format!("a{PAD}"))],
            Mode::Insert,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "h");
        assert_eq!(result.doc.boxes, vec![node(&format!("ah{PAD}"))]);
    }

    #[test]
    fn typing_appends_to_the_selected_box_label() {
        let state = new_state(
            vec![node(&format!("h{PAD}"))],
            Mode::Insert,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "i");
        assert_eq!(result.doc.boxes, vec![node(&format!("hi{PAD}"))]);
    }

    #[test]
    fn space_and_tilde_are_printable() {
        let state = new_state(
            vec![node(&format!("a{PAD}"))],
            Mode::Insert,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, " ");
        assert_eq!(result.doc.boxes, vec![node(&format!("a {PAD}"))]);

        let state = new_state(
            vec![node(PAD)],
            Mode::Insert,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "~");
        assert_eq!(result.doc.boxes, vec![node(&format!("~{PAD}"))]);
    }

    #[test]
    fn backspace_drops_the_last_character_and_is_a_no_op_when_empty() {
        let state = new_state(
            vec![node(&format!("hi{PAD}"))],
            Mode::Insert,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "\x7f");
        assert_eq!(result.doc.boxes, vec![node(&format!("h{PAD}"))]);

        let state = new_state(
            vec![node(PAD)],
            Mode::Insert,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "\x7f");
        assert_eq!(result.doc.boxes, vec![node(PAD)]);
    }

    #[test]
    fn esc_returns_to_command_mode_and_trims_pad() {
        let state = new_state(
            vec![node(&format!("hi{PAD}"))],
            Mode::Insert,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "\x1b");
        assert_eq!(result.mode, Mode::Command);
        assert_eq!(result.doc.boxes, vec![node("hi")]);
    }

    #[test]
    fn control_and_non_ascii_characters_return_the_state_unchanged() {
        let state = new_state(
            vec![node(&format!("hi{PAD}"))],
            Mode::Insert,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state.clone(), "\x01");
        assert_eq!(result.doc.boxes, vec![node(&format!("hi{PAD}"))]);

        let result = handle_key(state, "é");
        assert_eq!(result.doc.boxes, vec![node(&format!("hi{PAD}"))]);
    }

    #[test]
    fn insert_mode_is_dispatched_separately() {
        let state = new_state(
            vec![node(PAD)],
            Mode::Insert,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "q");
        assert!(result.running);
    }

    #[test]
    fn readme_insert_keymap_table_stays_in_sync() {
        let markdown = format!("\n\n{}\n", format_keymap_markdown());
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
    fn digit_in_insert_mode_types_the_digit_as_label_text() {
        for key in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
            let state = new_state(
                vec![node(&format!("a{PAD}"))],
                Mode::Insert,
                Some(Path {
                    ancestors: vec![],
                    index: 0,
                }),
            );
            let result = handle_key(state, key);
            assert_eq!(result.doc.boxes, vec![node(&format!("a{key}{PAD}"))]);
            assert_eq!(result.mode, Mode::Insert);
            assert_eq!(
                result.doc.selected,
                Some(Path {
                    ancestors: vec![],
                    index: 0
                })
            );
        }
    }

    #[test]
    fn insert_keymap_agrees_with_parse() {
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
                assert_eq!(parse(raw_for(display)), Some(binding.command));
            }
        }
    }
}
