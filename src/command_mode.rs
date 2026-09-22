use crate::diagram::{append, at, children_at, Path};
use crate::state::{
    add_child_box, blank_box, colour_row, next_colour, snapshot, undo, KeyBinding, Mode, State,
    DEFAULT_FILENAME, PAD,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Command {
    Undo,
    NewBox,
    NewSibling,
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
}

pub(crate) fn parse(key: &str) -> Option<Command> {
    if let Some(delta) = key.strip_prefix("\x1bSCROLL") {
        return delta.parse::<i64>().ok().map(Command::ScrollBy);
    }
    Some(match key {
        "u" => Command::Undo,
        "b" => Command::NewBox,
        "s" => Command::NewSibling,
        "h" => Command::SelectParent,
        "l" => Command::SelectChild,
        "j" => Command::SelectNext,
        "k" => Command::SelectPrevious,
        "i" => Command::EditLabel,
        "I" => Command::RenameLabel,
        "c" => Command::CycleColour,
        "C" => Command::CycleSiblingsColour,
        "F" => Command::ToggleSiblingsFill,
        "f" => Command::ToggleFill,
        "r" => Command::ToggleRounded,
        "q" => Command::Quit,
        _ => return None,
    })
}

#[allow(dead_code)]
pub(crate) const COMMAND_KEYMAP: &[KeyBinding<Command>] = &[
    KeyBinding {
        keys: &["u"],
        command: Command::Undo,
        description: "Undo the last change",
    },
    KeyBinding {
        keys: &["b"],
        command: Command::NewBox,
        description: "Add a child box",
    },
    KeyBinding {
        keys: &["s"],
        command: Command::NewSibling,
        description: "Add a sibling box",
    },
    KeyBinding {
        keys: &["h"],
        command: Command::SelectParent,
        description: "Select the parent box",
    },
    KeyBinding {
        keys: &["l"],
        command: Command::SelectChild,
        description: "Select the first child box",
    },
    KeyBinding {
        keys: &["j"],
        command: Command::SelectNext,
        description: "Select the next sibling",
    },
    KeyBinding {
        keys: &["k"],
        command: Command::SelectPrevious,
        description: "Select the previous sibling",
    },
    KeyBinding {
        keys: &["i"],
        command: Command::EditLabel,
        description: "Edit the selected box's label",
    },
    KeyBinding {
        keys: &["I"],
        command: Command::RenameLabel,
        description: "Rename the selected box's label",
    },
    KeyBinding {
        keys: &["c"],
        command: Command::CycleColour,
        description: "Cycle the box's colour",
    },
    KeyBinding {
        keys: &["C"],
        command: Command::CycleSiblingsColour,
        description: "Cycle the colour of every sibling",
    },
    KeyBinding {
        keys: &["f"],
        command: Command::ToggleFill,
        description: "Toggle the box's fill",
    },
    KeyBinding {
        keys: &["F"],
        command: Command::ToggleSiblingsFill,
        description: "Toggle the fill of every sibling",
    },
    KeyBinding {
        keys: &["r"],
        command: Command::ToggleRounded,
        description: "Toggle rounded corners",
    },
    KeyBinding {
        keys: &["q"],
        command: Command::Quit,
        description: "Save and quit (or choose where to save)",
    },
];

pub(crate) fn is_undoable(command: Command) -> bool {
    matches!(
        command,
        Command::NewBox
            | Command::CycleColour
            | Command::ToggleFill
            | Command::ToggleRounded
            | Command::CycleSiblingsColour
            | Command::ToggleSiblingsFill
            | Command::RenameLabel
    )
}

pub(crate) fn min_depth(command: Command) -> usize {
    match command {
        Command::Undo | Command::NewBox | Command::Quit | Command::ScrollBy(_) => 0,
        Command::SelectParent | Command::CycleSiblingsColour | Command::ToggleSiblingsFill => 2,
        _ => 1,
    }
}

fn enter_insert(mut state: State, path: Path, base_label: &str) -> State {
    at(&mut state.doc.boxes, &path).label = format!("{base_label}{PAD}");
    state.doc.selected = Some(path);
    state.mode = Mode::Insert;
    state
}

fn new_sibling(mut state: State, path: Path) -> State {
    let index = append(
        children_at(&mut state.doc.boxes, &path.ancestors),
        blank_box(),
    );
    state.doc.selected = Some(Path {
        ancestors: path.ancestors,
        index,
    });
    state.mode = Mode::Insert;
    state
}

fn select_parent(mut state: State, mut path: Path) -> State {
    state.doc.selected = Some(match path.ancestors.pop() {
        Some(index) => Path {
            ancestors: path.ancestors,
            index,
        },
        None => path,
    });
    state
}

fn select_child(mut state: State, mut path: Path) -> State {
    if !at(&mut state.doc.boxes, &path).children.is_empty() {
        path.ancestors.push(path.index);
        path.index = 0;
    }
    state.doc.selected = Some(path);
    state
}

fn select_next(mut state: State, mut path: Path) -> State {
    let count = children_at(&mut state.doc.boxes, &path.ancestors).len();
    if path.index + 1 < count {
        path.index += 1;
    }
    state.doc.selected = Some(path);
    state
}

fn select_previous(mut state: State, mut path: Path) -> State {
    path.index = path.index.saturating_sub(1);
    state.doc.selected = Some(path);
    state
}

fn repeat(mut state: State, path: Path, count: usize, step: fn(State, Path) -> State) -> State {
    state.doc.selected = Some(path);
    for _ in 0..count {
        let path = state.doc.selected.clone().unwrap();
        state = step(state, path);
    }
    state
}

fn edit_label(mut state: State, path: Path) -> State {
    let label = at(&mut state.doc.boxes, &path).label.clone();
    enter_insert(state, path, &label)
}

fn rename_label(state: State, path: Path) -> State {
    enter_insert(state, path, "")
}

fn cycle_colour(mut state: State, path: Path) -> State {
    let node = at(&mut state.doc.boxes, &path);
    node.colour = next_colour(node.colour);
    state.doc.selected = Some(path);
    state
}

fn cycle_siblings_colour(mut state: State, path: Path) -> State {
    colour_row(&mut state.doc.boxes, &path);
    state.doc.selected = Some(path);
    state
}

fn toggle_siblings_fill(mut state: State, path: Path) -> State {
    let siblings = children_at(&mut state.doc.boxes, &path.ancestors);
    if siblings.iter().all(|sibling| sibling.colour.is_none()) {
        state.doc.selected = Some(path);
        return state;
    }
    let all_filled = siblings.iter().all(|sibling| sibling.filled);
    for sibling in siblings.iter_mut() {
        sibling.filled = !all_filled;
    }
    state.doc.selected = Some(path);
    state
}

fn toggle_fill(mut state: State, path: Path) -> State {
    let node = at(&mut state.doc.boxes, &path);
    if node.colour.is_none() {
        state.doc.selected = Some(path);
        return state;
    }
    node.filled = !node.filled;
    state.doc.selected = Some(path);
    state
}

fn toggle_rounded(mut state: State, path: Path) -> State {
    let node = at(&mut state.doc.boxes, &path);
    node.rounded = !node.rounded;
    state.doc.selected = Some(path);
    state
}

fn quit(mut state: State) -> State {
    if state.new_file && state.doc.boxes.is_empty() {
        state.save_to = None;
        state.running = false;
    } else if state.save_to.is_some() {
        state.running = false;
    } else {
        state.mode = Mode::SavePrompt {
            filename: DEFAULT_FILENAME.to_string(),
        };
    }
    state
}

fn reselect(mut state: State, selected: Option<Path>) -> State {
    state.doc.selected = selected;
    state
}

pub(crate) fn reduce(mut state: State, command: Command) -> State {
    let count = state.pending_count.take().unwrap_or(1);
    let depth = match &state.doc.selected {
        None => 0,
        Some(path) => 1 + path.ancestors.len(),
    };
    if depth < min_depth(command) {
        return state;
    }
    let mut state = if is_undoable(command) {
        snapshot(state)
    } else {
        state
    };
    match (command, state.doc.selected.take()) {
        (Command::Undo, selected) => undo(reselect(state, selected)),
        (Command::NewBox, selected) => add_child_box(state, selected),
        (Command::Quit, selected) => quit(reselect(state, selected)),
        (Command::ScrollBy(delta), selected) => {
            state.scroll_x += delta;
            reselect(state, selected)
        }
        (Command::NewSibling, Some(path)) => new_sibling(state, path),
        (Command::SelectParent, Some(path)) => repeat(state, path, count, select_parent),
        (Command::SelectChild, Some(path)) => select_child(state, path),
        (Command::SelectNext, Some(path)) => repeat(state, path, count, select_next),
        (Command::SelectPrevious, Some(path)) => repeat(state, path, count, select_previous),
        (Command::EditLabel, Some(path)) => edit_label(state, path),
        (Command::RenameLabel, Some(path)) => rename_label(state, path),
        (Command::CycleColour, Some(path)) => cycle_colour(state, path),
        (Command::CycleSiblingsColour, Some(path)) => cycle_siblings_colour(state, path),
        (Command::ToggleSiblingsFill, Some(path)) => toggle_siblings_fill(state, path),
        (Command::ToggleFill, Some(path)) => toggle_fill(state, path),
        (Command::ToggleRounded, Some(path)) => toggle_rounded(state, path),
        (_, None) => state,
    }
}

#[allow(dead_code)]
pub(crate) fn format_keymap_markdown() -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{node, node_with_children, palette, Node};
    use crate::state::{handle_key, new_state};

    const COMMANDS: [Command; 15] = [
        Command::Undo,
        Command::NewBox,
        Command::NewSibling,
        Command::SelectParent,
        Command::SelectChild,
        Command::SelectNext,
        Command::SelectPrevious,
        Command::EditLabel,
        Command::RenameLabel,
        Command::CycleColour,
        Command::CycleSiblingsColour,
        Command::ToggleSiblingsFill,
        Command::ToggleFill,
        Command::ToggleRounded,
        Command::Quit,
    ];

    #[test]
    fn readme_keymap_table_stays_in_sync() {
        let markdown = format!("\n\n{}\n", format_keymap_markdown());
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
        assert_eq!(parse("u"), Some(Command::Undo));
        assert_eq!(parse("b"), Some(Command::NewBox));
        assert_eq!(parse("s"), Some(Command::NewSibling));
        assert_eq!(parse("h"), Some(Command::SelectParent));
        assert_eq!(parse("l"), Some(Command::SelectChild));
        assert_eq!(parse("j"), Some(Command::SelectNext));
        assert_eq!(parse("k"), Some(Command::SelectPrevious));
        assert_eq!(parse("i"), Some(Command::EditLabel));
        assert_eq!(parse("I"), Some(Command::RenameLabel));
        assert_eq!(parse("c"), Some(Command::CycleColour));
        assert_eq!(parse("C"), Some(Command::CycleSiblingsColour));
        assert_eq!(parse("F"), Some(Command::ToggleSiblingsFill));
        assert_eq!(parse("f"), Some(Command::ToggleFill));
        assert_eq!(parse("r"), Some(Command::ToggleRounded));
        assert_eq!(parse("q"), Some(Command::Quit));
    }

    #[test]
    fn parse_returns_nothing_for_an_unknown_key() {
        assert_eq!(parse("x"), None);
        assert_eq!(parse("\x1b"), None);
        assert_eq!(parse("é"), None);
    }

    #[test]
    fn parse_reads_a_scroll_prefix_into_a_signed_delta() {
        assert_eq!(parse("\x1bSCROLL5"), Some(Command::ScrollBy(5)));
        assert_eq!(parse("\x1bSCROLL-5"), Some(Command::ScrollBy(-5)));
        assert_eq!(parse("\x1bSCROLL0"), Some(Command::ScrollBy(0)));
    }

    #[test]
    fn parse_returns_nothing_for_a_malformed_scroll_prefix() {
        assert_eq!(parse("\x1bSCROLL"), None);
        assert_eq!(parse("\x1bSCROLLx"), None);
    }

    #[test]
    fn parse_returns_nothing_for_each_digit_key() {
        for key in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
            assert_eq!(parse(key), None);
        }
    }

    #[test]
    fn command_keymap_agrees_with_parse() {
        let bound: Vec<String> = COMMAND_KEYMAP
            .iter()
            .flat_map(|binding| binding.keys)
            .map(|key| key.to_string())
            .collect();

        for binding in COMMAND_KEYMAP {
            for key in binding.keys {
                assert_eq!(parse(key), Some(binding.command));
            }
        }

        for ch in (b'a'..=b'z').chain(b'A'..=b'Z').chain(b'0'..=b'9') {
            let key = (ch as char).to_string();
            if !bound.contains(&key) {
                assert_eq!(parse(&key), None);
            }
        }

        let mut unique = bound.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), bound.len());
    }

    #[test]
    fn a_command_below_its_minimum_depth_leaves_the_document_unchanged() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        for command in COMMANDS {
            let depth = min_depth(command);
            if depth == 0 {
                continue;
            }
            let selected = match depth {
                1 => None,
                _ => Some(Path {
                    ancestors: vec![0; depth - 2],
                    index: 0,
                }),
            };
            let state = new_state(boxes.clone(), Mode::Command, selected);
            let result = reduce(state.clone(), command);
            assert_eq!(result.doc, state.doc);
        }
    }

    #[test]
    fn b_on_an_empty_canvas_appends_a_box_enters_insert_and_selects_it() {
        let state = new_state(vec![], Mode::Command, None);
        let result: State = handle_key(state, "b");
        assert_eq!(result.doc.boxes, vec![node(PAD)]);
        assert_eq!(result.mode, Mode::Insert);
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
        assert!(result.running);
    }

    #[test]
    fn b_on_an_empty_canvas_does_not_mutate_the_given_state() {
        let state = new_state(vec![], Mode::Command, None);
        handle_key(state.clone(), "b");
        assert_eq!(state.doc.boxes, Vec::<Node>::new());
    }

    #[test]
    fn b_on_a_selected_box_appends_and_selects_a_child() {
        let state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "b");
        assert_eq!(
            result.doc.boxes,
            vec![node_with_children("a", vec![node(PAD)])]
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
    fn a_second_b_on_the_same_parent_places_a_second_child() {
        let state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let state = handle_key(state, "b");
        let state = handle_key(state, "\x1b");
        let state = handle_key(state, "h");
        let result = handle_key(state, "b");
        assert_eq!(
            result.doc.boxes,
            vec![node_with_children("a", vec![node(""), node(PAD)])]
        );
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![0],
                index: 1
            })
        );
    }

    #[test]
    fn q_opens_the_save_prompt_and_preserves_boxes_and_selection() {
        let state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "q");
        assert!(result.running);
        assert_eq!(result.doc.boxes, vec![node("a")]);
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
        assert_eq!(
            result.mode,
            Mode::SavePrompt {
                filename: DEFAULT_FILENAME.to_string()
            }
        );
    }

    #[test]
    fn q_with_a_file_to_save_to_stops_without_prompting() {
        let mut state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        state.save_to = Some("plans.dre".to_string());
        let result = handle_key(state, "q");
        assert!(!result.running);
        assert_eq!(result.save_to, Some("plans.dre".to_string()));
        assert_eq!(result.mode, Mode::Command);
        assert_eq!(result.doc.boxes, vec![node("a")]);
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
    }

    fn new_file_state(boxes: Vec<Node>) -> State {
        let mut state = new_state(boxes, Mode::Command, None);
        state.save_to = Some("ideas.dre".to_string());
        state.new_file = true;
        state
    }

    #[test]
    fn q_on_a_new_file_with_no_boxes_stops_without_a_file_to_save_to() {
        let result = handle_key(new_file_state(vec![]), "q");
        assert!(!result.running);
        assert_eq!(result.save_to, None);
        assert_eq!(result.mode, Mode::Command);
    }

    #[test]
    fn q_on_a_new_file_after_undoing_every_box_stops_without_a_file_to_save_to() {
        let state = handle_key(new_file_state(vec![]), "b");
        let state = handle_key(state, "\x1b");
        let state = handle_key(state, "u");
        let result = handle_key(state, "q");
        assert!(!result.running);
        assert_eq!(result.save_to, None);
    }

    #[test]
    fn q_on_a_new_file_with_boxes_stops_without_prompting() {
        let result = handle_key(new_file_state(vec![node("a")]), "q");
        assert!(!result.running);
        assert_eq!(result.save_to, Some("ideas.dre".to_string()));
        assert_eq!(result.mode, Mode::Command);
    }

    #[test]
    fn q_on_an_existing_file_with_no_boxes_keeps_its_file_to_save_to() {
        let mut state = new_state(vec![], Mode::Command, None);
        state.save_to = Some("plans.dre".to_string());
        let result = handle_key(state, "q");
        assert!(!result.running);
        assert_eq!(result.save_to, Some("plans.dre".to_string()));
        assert_eq!(result.mode, Mode::Command);
    }

    #[test]
    fn s_on_a_top_level_box_appends_a_sibling() {
        let state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "s");
        assert_eq!(result.doc.boxes, vec![node("a"), node(PAD)]);
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 1
            })
        );
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn s_on_a_child_box_appends_a_sibling_to_the_parents_children() {
        let boxes = vec![node_with_children("a", vec![node("b")])];
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![0],
                index: 0,
            }),
        );
        let result = handle_key(state, "s");
        assert_eq!(
            result.doc.boxes,
            vec![node_with_children("a", vec![node("b"), node(PAD)])]
        );
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![0],
                index: 1
            })
        );
    }

    #[test]
    fn s_with_no_selection_returns_the_state_unchanged() {
        let state = new_state(vec![node("a")], Mode::Command, None);
        let result = handle_key(state, "s");
        assert_eq!(result.doc.boxes, vec![node("a")]);
        assert_eq!(result.doc.selected, None);
    }

    #[test]
    fn h_selects_the_parent_and_is_a_no_op_at_the_top() {
        let boxes = vec![node_with_children("a", vec![node("c")])];
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![0],
                index: 0,
            }),
        );
        let result = handle_key(state, "h");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );

        let result = handle_key(result, "h");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
    }

    #[test]
    fn h_on_an_empty_canvas_returns_the_state_unchanged() {
        let state = new_state(vec![], Mode::Command, None);
        let result = handle_key(state, "h");
        assert_eq!(result.doc.selected, None);
    }

    #[test]
    fn l_selects_the_first_child_or_keeps_selection_with_none() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "l");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![0],
                index: 0
            })
        );

        let state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "l");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
    }

    #[test]
    fn j_and_k_move_between_siblings_with_bounds() {
        let state = new_state(
            vec![node("a"), node("b")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "j");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 1
            })
        );

        let state = new_state(
            vec![node("a"), node("b")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 1,
            }),
        );
        let result = handle_key(state, "j");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 1
            })
        );

        let state = new_state(
            vec![node("a"), node("b")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 1,
            }),
        );
        let result = handle_key(state, "k");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );

        let state = new_state(
            vec![node("a"), node("b")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "k");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
    }

    #[test]
    fn j_with_a_count_prefix_moves_multiple_steps_forward() {
        let boxes: Vec<Node> = (0..7).map(|i| node(&i.to_string())).collect();
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(handle_key(state, "3"), "j");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 3
            })
        );
    }

    #[test]
    fn k_with_a_count_prefix_moves_multiple_steps_back() {
        let boxes: Vec<Node> = (0..7).map(|i| node(&i.to_string())).collect();
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 6,
            }),
        );
        let result = handle_key(handle_key(state, "3"), "k");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 3
            })
        );
    }

    #[test]
    fn j_with_a_count_prefix_clamps_at_the_last_sibling() {
        let boxes: Vec<Node> = (0..5).map(|i| node(&i.to_string())).collect();
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 3,
            }),
        );
        let result = handle_key(handle_key(state, "3"), "j");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 4
            })
        );
    }

    #[test]
    fn k_with_a_count_prefix_clamps_at_the_first_sibling() {
        let boxes: Vec<Node> = (0..5).map(|i| node(&i.to_string())).collect();
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 1,
            }),
        );
        let result = handle_key(handle_key(state, "3"), "k");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
    }

    #[test]
    fn h_with_a_count_prefix_climbs_multiple_levels() {
        let boxes = vec![node_with_children(
            "root",
            vec![node_with_children(
                "a",
                vec![node_with_children("b", vec![node("c")])],
            )],
        )];
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![0, 0, 0],
                index: 0,
            }),
        );
        let result = handle_key(handle_key(state, "3"), "h");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
    }

    #[test]
    fn h_with_a_large_count_stops_at_the_top_level_box() {
        let boxes = vec![node_with_children(
            "root",
            vec![node_with_children(
                "a",
                vec![node_with_children("b", vec![node("c")])],
            )],
        )];
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![0, 0],
                index: 0,
            }),
        );
        let state = handle_key(state, "1");
        let state = handle_key(state, "2");
        let result = handle_key(state, "h");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
    }

    #[test]
    fn one_j_moves_one_step_like_plain_j() {
        let boxes = vec![node("a"), node("b"), node("c")];
        let plain = new_state(
            boxes.clone(),
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let prefixed = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let plain = handle_key(plain, "j");
        let prefixed = handle_key(handle_key(prefixed, "1"), "j");
        assert_eq!(plain.doc.selected, prefixed.doc.selected);
        assert_eq!(
            prefixed.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 1
            })
        );
    }

    #[test]
    fn twelve_j_moves_twelve_steps() {
        let boxes: Vec<Node> = (0..15).map(|i| node(&i.to_string())).collect();
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let state = handle_key(state, "1");
        let state = handle_key(state, "2");
        let result = handle_key(state, "j");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 12
            })
        );
    }

    #[test]
    fn zero_j_leaves_the_selection_unchanged() {
        let boxes = vec![node("a"), node("b"), node("c")];
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 1,
            }),
        );
        let result = handle_key(handle_key(state, "0"), "j");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 1
            })
        );
    }

    #[test]
    fn zero_k_leaves_the_selection_unchanged() {
        let boxes = vec![node("a"), node("b"), node("c")];
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 1,
            }),
        );
        let result = handle_key(handle_key(state, "0"), "k");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 1
            })
        );
    }

    #[test]
    fn zero_h_leaves_the_selection_unchanged() {
        let boxes = vec![node_with_children("root", vec![node("a")])];
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![0],
                index: 0,
            }),
        );
        let result = handle_key(handle_key(state, "0"), "h");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![0],
                index: 0
            })
        );
    }

    #[test]
    fn a_count_prefix_on_s_behaves_exactly_as_s_and_is_consumed() {
        let boxes = vec![node("a"), node("b"), node("c"), node("d")];
        let plain = new_state(
            boxes.clone(),
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 2,
            }),
        );
        let prefixed = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 2,
            }),
        );
        let plain = handle_key(plain, "s");
        let prefixed = handle_key(handle_key(prefixed, "2"), "s");
        assert_eq!(plain.doc.boxes, prefixed.doc.boxes);
        assert_eq!(plain.doc.selected, prefixed.doc.selected);
        assert_eq!(plain.mode, prefixed.mode);
        assert_eq!(
            prefixed.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 4
            })
        );

        let committed = handle_key(prefixed, "\x1b");
        let result = handle_key(committed, "k");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 3
            })
        );
    }

    #[test]
    fn a_digit_followed_by_an_unknown_key_then_j_moves_one_step() {
        let boxes: Vec<Node> = (0..5).map(|i| node(&i.to_string())).collect();
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let state = handle_key(state, "3");
        let state = handle_key(state, "x");
        let result = handle_key(state, "j");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 1
            })
        );
    }

    #[test]
    fn j_with_a_count_prefix_and_no_selection_leaves_the_document_unchanged() {
        let boxes: Vec<Node> = (0..5).map(|i| node(&i.to_string())).collect();
        let state = new_state(boxes.clone(), Mode::Command, None);
        let state = handle_key(state, "3");
        let result = handle_key(state, "j");
        assert_eq!(result.doc.boxes, boxes);
        assert_eq!(result.doc.selected, None);
    }

    #[test]
    fn a_count_consumed_by_a_no_op_selection_does_not_leak_to_the_next_command() {
        let boxes = vec![node("a"), node("b"), node("c")];
        let state = new_state(boxes.clone(), Mode::Command, None);
        let state = handle_key(state, "3");
        let state = handle_key(state, "j");
        let state = handle_key(state, "b");
        let state = handle_key(state, "\x1b");
        let result = handle_key(state, "k");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 2
            })
        );
    }

    #[test]
    fn a_count_consumed_by_failed_min_depth_does_not_leak_to_the_next_command() {
        let boxes: Vec<Node> = (0..5).map(|i| node(&i.to_string())).collect();
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let state = handle_key(state, "3");
        let state = handle_key(state, "h");
        let result = handle_key(state, "j");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 1
            })
        );
    }

    #[test]
    fn i_enters_insert_and_appends_pad_to_the_selected_boxs_label() {
        let state = new_state(
            vec![node("a"), node("b")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 1,
            }),
        );
        let result = handle_key(state, "i");
        assert_eq!(result.mode, Mode::Insert);
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 1
            })
        );
        assert_eq!(result.doc.boxes, vec![node("a"), node(&format!("b{PAD}"))]);
    }

    #[test]
    fn i_on_an_empty_canvas_returns_the_state_unchanged() {
        let state = new_state(vec![], Mode::Command, None);
        let result = handle_key(state, "i");
        assert_eq!(result.mode, Mode::Command);
        assert_eq!(result.doc.boxes, Vec::<Node>::new());
    }

    #[test]
    fn capital_i_clears_the_selected_boxs_label() {
        let state = new_state(
            vec![node("a"), node("b")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 1,
            }),
        );
        let result = handle_key(state, "I");
        assert_eq!(result.mode, Mode::Insert);
        assert_eq!(result.doc.boxes, vec![node("a"), node(PAD)]);
    }

    #[test]
    fn cycle_colour_and_toggle_fill_advance_independently() {
        let state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "c");
        let mut expected = node("a");
        expected.colour = next_colour(None);
        assert_eq!(result.doc.boxes, vec![expected]);
        assert_eq!(result.doc.boxes[0].label, "a");

        let state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let colourised = handle_key(state, "c");
        let state = new_state(
            colourised.doc.boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "f");
        let mut expected = node("a");
        expected.colour = next_colour(None);
        expected.filled = true;
        assert_eq!(result.doc.boxes, vec![expected]);

        let mut state_boxed = vec![node("a")];
        let mut colour: Option<u8> = None;
        let presses_past_the_last_colour = (0..).take_while(|&i| palette(i).is_some()).count() + 1;
        for _ in 0..presses_past_the_last_colour {
            let s = new_state(
                state_boxed.clone(),
                Mode::Command,
                Some(Path {
                    ancestors: vec![],
                    index: 0,
                }),
            );
            let result = handle_key(s, "c");
            state_boxed = result.doc.boxes;
            colour = state_boxed[0].colour;
        }
        assert_eq!(colour, None);
    }

    #[test]
    fn f_toggles_fill_on_and_off() {
        let colour = next_colour(None);
        let mut colourised = node("a");
        colourised.colour = colour;
        let state = new_state(
            vec![colourised.clone()],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "f");
        let mut filled = node("a");
        filled.colour = colour;
        filled.filled = true;
        assert_eq!(result.doc.boxes, vec![filled]);

        let state = new_state(
            result.doc.boxes.clone(),
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "f");
        assert_eq!(result.doc.boxes, vec![colourised]);
    }

    #[test]
    fn f_on_a_colourless_box_does_nothing() {
        let state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state.clone(), "f");
        assert_eq!(result.doc.boxes, state.doc.boxes);
        assert_eq!(result.doc.selected, state.doc.selected);
    }

    #[test]
    fn a_toggled_fill_survives_a_colour_cycle_back_to_plain() {
        let mut state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        for _ in (0..).take_while(|&i| palette(i).is_some()) {
            state = handle_key(state, "c");
        }
        assert_ne!(state.doc.boxes[0].colour, None);
        state = handle_key(state, "f");
        assert_eq!(state.doc.boxes[0].filled, true);
        let plain = handle_key(state, "c");
        assert_eq!(plain.doc.boxes[0].colour, None);
        assert_eq!(plain.doc.boxes[0].filled, true);
        let re_coloured = handle_key(plain, "c");
        assert_ne!(re_coloured.doc.boxes[0].colour, None);
        assert_eq!(re_coloured.doc.boxes[0].filled, true);
    }

    #[test]
    fn toggle_rounded_flips_and_flips_back() {
        let state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "r");
        assert_eq!(result.doc.boxes[0].rounded, true);
        let state = new_state(
            result.doc.boxes.clone(),
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "r");
        assert_eq!(result.doc.boxes[0].rounded, false);
    }

    #[test]
    fn capital_c_on_a_top_level_box_does_nothing() {
        let state = new_state(
            vec![node("a"), node("b")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state.clone(), "C");
        assert_eq!(result.doc.boxes, state.doc.boxes);
        assert_eq!(result.doc.selected, state.doc.selected);
    }

    #[test]
    fn capital_f_on_a_top_level_box_does_nothing() {
        let state = new_state(
            vec![node("a"), node("b")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state.clone(), "F");
        assert_eq!(result.doc.boxes, state.doc.boxes);
        assert_eq!(result.doc.selected, state.doc.selected);
    }

    #[test]
    fn capital_f_with_nothing_selected_does_nothing() {
        let state = new_state(vec![node("a")], Mode::Command, None);
        let result = handle_key(state.clone(), "F");
        assert_eq!(result.doc.boxes, state.doc.boxes);
        assert_eq!(result.doc.selected, state.doc.selected);
    }

    #[test]
    fn capital_c_advances_uniformly_coloured_siblings() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![0],
                index: 1,
            }),
        );
        let result = handle_key(state, "C");
        let mut c = node("c");
        c.colour = next_colour(None);
        let mut d = node("d");
        d.colour = next_colour(None);
        assert_eq!(result.doc.boxes, vec![node_with_children("a", vec![c, d])]);
    }

    #[test]
    fn capital_f_toggles_every_sibling_fill_and_leaves_colour_untouched() {
        let colour_one = next_colour(None);
        let colour_two = next_colour(colour_one);
        let mut c = node("c");
        c.colour = colour_one;
        let mut d = node("d");
        d.colour = colour_two;
        let boxes = vec![node_with_children("a", vec![c, d])];
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![0],
                index: 1,
            }),
        );
        let result = handle_key(state, "F");
        assert_eq!(result.doc.boxes[0].children[0].filled, true);
        assert_eq!(result.doc.boxes[0].children[0].colour, colour_one);
        assert_eq!(result.doc.boxes[0].children[1].filled, true);
        assert_eq!(result.doc.boxes[0].children[1].colour, colour_two);
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![0],
                index: 1
            })
        );

        let state = new_state(
            result.doc.boxes.clone(),
            Mode::Command,
            Some(Path {
                ancestors: vec![0],
                index: 1,
            }),
        );
        let result = handle_key(state, "F");
        assert_eq!(result.doc.boxes[0].children[0].filled, false);
        assert_eq!(result.doc.boxes[0].children[0].colour, colour_one);
        assert_eq!(result.doc.boxes[0].children[1].filled, false);
        assert_eq!(result.doc.boxes[0].children[1].colour, colour_two);
    }

    #[test]
    fn capital_f_fills_every_sibling_including_colourless_ones() {
        let colour = next_colour(None);
        let mut c = node("c");
        c.colour = colour;
        let d = node("d");
        let boxes = vec![node_with_children("a", vec![c, d])];
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![0],
                index: 0,
            }),
        );
        let result = handle_key(state, "F");
        assert_eq!(result.doc.boxes[0].children[0].filled, true);
        assert_eq!(result.doc.boxes[0].children[0].colour, colour);
        assert_eq!(result.doc.boxes[0].children[1].filled, true);
        assert_eq!(result.doc.boxes[0].children[1].colour, None);
    }

    #[test]
    fn capital_f_does_nothing_when_every_sibling_is_colourless() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![0],
                index: 1,
            }),
        );
        let result = handle_key(state.clone(), "F");
        assert_eq!(result.doc.boxes, state.doc.boxes);
        assert_eq!(result.doc.selected, state.doc.selected);
    }

    #[test]
    fn u_after_b_restores_boxes_and_selected() {
        let before = new_state(vec![], Mode::Command, None);
        let after = handle_key(before.clone(), "b");
        let after_escape = handle_key(after, "\x1b");
        let undone = handle_key(after_escape, "u");
        assert_eq!(undone.doc.boxes, before.doc.boxes);
        assert_eq!(undone.doc.selected, before.doc.selected);
        assert_eq!(undone.mode, before.mode);
    }

    #[test]
    fn u_after_c_restores_boxes() {
        let before = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let after = handle_key(before.clone(), "c");
        let undone = handle_key(after, "u");
        assert_eq!(undone.doc.boxes, before.doc.boxes);
    }

    #[test]
    fn u_after_c_with_nothing_selected_is_a_no_op() {
        let state = new_state(vec![node("a")], Mode::Command, None);
        let after = handle_key(state.clone(), "c");
        let undone = handle_key(after, "u");
        assert_eq!(undone.doc.boxes, state.doc.boxes);
        assert_eq!(undone.doc.selected, state.doc.selected);
    }

    #[test]
    fn u_with_no_previous_action_leaves_state_unchanged() {
        let state = new_state(vec![], Mode::Command, None);
        let result = handle_key(state, "u");
        assert_eq!(result.doc.boxes, Vec::<Node>::new());
        assert_eq!(result.doc.selected, None);
    }

    #[test]
    fn u_twice_in_a_row_does_not_redo() {
        let state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let after_command = handle_key(state, "c");
        let after_first_undo = handle_key(after_command, "u");
        let after_second_undo = handle_key(after_first_undo.clone(), "u");
        assert_eq!(after_second_undo.doc.boxes, after_first_undo.doc.boxes);
        assert_eq!(
            after_second_undo.doc.selected,
            after_first_undo.doc.selected
        );
    }

    #[test]
    fn movement_keys_do_not_clobber_an_existing_undo_snapshot() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let before = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![0],
                index: 0,
            }),
        );
        let after_command = handle_key(before.clone(), "c");
        let navigated = handle_key(after_command, "j");
        let navigated = handle_key(navigated, "h");
        let navigated = handle_key(navigated, "l");
        let navigated = handle_key(navigated, "k");
        let undone = handle_key(navigated, "u");
        assert_eq!(undone.doc.boxes, before.doc.boxes);
        assert_eq!(undone.doc.selected, before.doc.selected);
    }

    #[test]
    fn count_prefixed_movement_does_not_clobber_an_existing_undo_snapshot() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let before = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![0],
                index: 0,
            }),
        );
        let after_command = handle_key(before.clone(), "c");
        let navigated = handle_key(handle_key(after_command.clone(), "3"), "j");
        let navigated = handle_key(handle_key(navigated, "2"), "h");
        let navigated = handle_key(navigated, "k");
        let navigated = handle_key(handle_key(navigated, "2"), "l");
        let undone = handle_key(navigated, "u");
        assert_eq!(undone.doc.boxes, before.doc.boxes);
        assert_eq!(undone.doc.selected, before.doc.selected);
    }

    #[test]
    fn q_does_not_clobber_an_existing_undo_snapshot() {
        let before = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let after_command = handle_key(before.clone(), "c");
        let mut after_quit = handle_key(after_command, "q");
        after_quit.mode = Mode::Command;
        let undone = handle_key(after_quit, "u");
        assert_eq!(undone.doc.boxes, before.doc.boxes);
        assert_eq!(undone.doc.selected, before.doc.selected);
    }

    #[test]
    fn u_after_an_insert_session_undoes_the_b_that_started_it() {
        let before = new_state(vec![], Mode::Command, None);
        let after_b = handle_key(before.clone(), "b");
        let after_typing = handle_key(after_b, "h");
        let after_typing = handle_key(after_typing, "i");
        let after_escape = handle_key(after_typing, "\x1b");
        let undone = handle_key(after_escape, "u");
        assert_eq!(undone.doc.boxes, before.doc.boxes);
        assert_eq!(undone.doc.selected, before.doc.selected);
    }

    #[test]
    fn repeated_u_walks_back_through_every_undoable_command() {
        let start = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let after_colour = handle_key(start.clone(), "c");
        let after_fill = handle_key(after_colour.clone(), "f");
        let after_rounded = handle_key(after_fill.clone(), "r");
        let undone = handle_key(after_rounded, "u");
        assert_eq!(undone.doc.boxes, after_fill.doc.boxes);
        let undone = handle_key(undone, "u");
        assert_eq!(undone.doc.boxes, after_colour.doc.boxes);
        let undone = handle_key(undone, "u");
        assert_eq!(undone.doc.boxes, start.doc.boxes);
        assert_eq!(undone.doc.selected, start.doc.selected);
    }

    #[test]
    fn u_on_an_exhausted_history_leaves_the_state_unchanged() {
        let start = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let after_colour = handle_key(start.clone(), "c");
        let after_fill = handle_key(after_colour, "f");
        let undone = handle_key(after_fill, "u");
        let undone = handle_key(undone, "u");
        let exhausted = handle_key(undone, "u");
        assert_eq!(exhausted.doc.boxes, start.doc.boxes);
        assert_eq!(exhausted.doc.selected, start.doc.selected);
        assert_eq!(exhausted.mode, start.mode);
        assert_eq!(exhausted.running, start.running);
    }

    #[test]
    fn scroll_by_command_shifts_scroll_x_by_the_given_delta_and_preserves_selection() {
        let mut state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        state.scroll_x = 3;
        let result = reduce(state, Command::ScrollBy(4));
        assert_eq!(result.scroll_x, 7);
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
    }

    #[test]
    fn scroll_by_command_works_with_nothing_selected() {
        let state = new_state(vec![], Mode::Command, None);
        let result = reduce(state, Command::ScrollBy(-2));
        assert_eq!(result.scroll_x, -2);
        assert_eq!(result.doc.selected, None);
    }

    #[test]
    fn scroll_by_is_not_undoable() {
        assert!(!is_undoable(Command::ScrollBy(5)));
    }

    #[test]
    fn u_after_a_scroll_leaves_scroll_x_untouched() {
        let state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let scrolled = handle_key(state, "\x1bSCROLL9");
        let after_undo = handle_key(scrolled, "u");
        assert_eq!(after_undo.scroll_x, 9);
    }

    #[test]
    fn u_after_capital_i_restores_the_boxs_previous_label() {
        let before = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let after = handle_key(before.clone(), "I");
        let after_escape = handle_key(after, "\x1b");
        let undone = handle_key(after_escape, "u");
        assert_eq!(undone.doc.boxes, before.doc.boxes);
    }
}
