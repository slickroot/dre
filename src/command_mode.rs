use crate::state::{
    at, children_at, colour_row, fill_row, grow, next_colour, snapshot, undo, Mode, Path, State, DEFAULT_FILENAME,
    PAD,
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
    CycleSiblingsFill,
    CycleFill,
    ToggleRounded,
    Quit,
}

pub(crate) fn parse(key: &str) -> Option<Command> {
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
        "F" => Command::CycleSiblingsFill,
        "f" => Command::CycleFill,
        "r" => Command::ToggleRounded,
        "q" => Command::Quit,
        _ => return None,
    })
}

pub(crate) fn is_undoable(command: Command) -> bool {
    matches!(
        command,
        Command::NewBox
            | Command::CycleColour
            | Command::CycleFill
            | Command::ToggleRounded
            | Command::CycleSiblingsColour
            | Command::CycleSiblingsFill
            | Command::RenameLabel
    )
}

pub(crate) fn min_depth(command: Command) -> usize {
    match command {
        Command::Undo | Command::NewBox | Command::Quit => 0,
        Command::SelectParent | Command::CycleSiblingsColour | Command::CycleSiblingsFill => 2,
        _ => 1,
    }
}

fn enter_insert(mut state: State, path: Path, base_label: &str) -> State {
    at(&mut state.doc.boxes, &path).label = format!("{base_label}{PAD}");
    state.doc.selected = Some(path);
    state.mode = Mode::Insert;
    state
}

fn new_box(mut state: State, selected: Option<Path>) -> State {
    state.doc.selected = match selected {
        Some(mut path) => {
            let index = grow(&mut at(&mut state.doc.boxes, &path).children);
            path.ancestors.push(path.index);
            Some(Path { ancestors: path.ancestors, index })
        }
        None => {
            let index = grow(&mut state.doc.boxes);
            Some(Path { ancestors: Vec::new(), index })
        }
    };
    state.mode = Mode::Insert;
    state
}

fn new_sibling(mut state: State, path: Path) -> State {
    let index = grow(children_at(&mut state.doc.boxes, &path.ancestors));
    state.doc.selected = Some(Path { ancestors: path.ancestors, index });
    state.mode = Mode::Insert;
    state
}

fn select_parent(mut state: State, mut path: Path) -> State {
    state.doc.selected = path.ancestors.pop().map(|index| Path { ancestors: path.ancestors, index });
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

fn cycle_siblings_fill(mut state: State, path: Path) -> State {
    fill_row(&mut state.doc.boxes, &path);
    state.doc.selected = Some(path);
    state
}

fn cycle_fill(mut state: State, path: Path) -> State {
    let node = at(&mut state.doc.boxes, &path);
    node.fill = next_colour(node.fill);
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
        state.mode = Mode::SavePrompt { filename: DEFAULT_FILENAME.to_string() };
    }
    state
}

fn reselect(mut state: State, selected: Option<Path>) -> State {
    state.doc.selected = selected;
    state
}

pub(crate) fn reduce(state: State, command: Command) -> State {
    let depth = match &state.doc.selected {
        None => 0,
        Some(path) => 1 + path.ancestors.len(),
    };
    if depth < min_depth(command) {
        return state;
    }
    let mut state = if is_undoable(command) { snapshot(state) } else { state };
    match (command, state.doc.selected.take()) {
        (Command::Undo, selected) => undo(reselect(state, selected)),
        (Command::NewBox, selected) => new_box(state, selected),
        (Command::Quit, selected) => quit(reselect(state, selected)),
        (Command::NewSibling, Some(path)) => new_sibling(state, path),
        (Command::SelectParent, Some(path)) => select_parent(state, path),
        (Command::SelectChild, Some(path)) => select_child(state, path),
        (Command::SelectNext, Some(path)) => select_next(state, path),
        (Command::SelectPrevious, Some(path)) => select_previous(state, path),
        (Command::EditLabel, Some(path)) => edit_label(state, path),
        (Command::RenameLabel, Some(path)) => rename_label(state, path),
        (Command::CycleColour, Some(path)) => cycle_colour(state, path),
        (Command::CycleSiblingsColour, Some(path)) => cycle_siblings_colour(state, path),
        (Command::CycleSiblingsFill, Some(path)) => cycle_siblings_fill(state, path),
        (Command::CycleFill, Some(path)) => cycle_fill(state, path),
        (Command::ToggleRounded, Some(path)) => toggle_rounded(state, path),
        (_, None) => state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{handle_key, new_state, Node, PALETTE_SIZE};

    fn node(label: &str) -> Node {
        Node { label: label.to_string(), ..Default::default() }
    }

    fn node_with_children(label: &str, children: Vec<Node>) -> Node {
        Node { label: label.to_string(), children, ..Default::default() }
    }

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
        Command::CycleSiblingsFill,
        Command::CycleFill,
        Command::ToggleRounded,
        Command::Quit,
    ];

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
        assert_eq!(parse("F"), Some(Command::CycleSiblingsFill));
        assert_eq!(parse("f"), Some(Command::CycleFill));
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
    fn a_command_below_its_minimum_depth_leaves_the_document_unchanged() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        for command in COMMANDS {
            let depth = min_depth(command);
            if depth == 0 {
                continue;
            }
            let selected = match depth {
                1 => None,
                _ => Some(Path { ancestors: vec![0; depth - 2], index: 0 }),
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
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 0 }));
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
        let state = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "b");
        assert_eq!(result.doc.boxes, vec![node_with_children("a", vec![node(PAD)])]);
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![0], index: 0 }));
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn a_second_b_on_the_same_parent_places_a_second_child() {
        let state = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let state = handle_key(state, "b");
        let state = handle_key(state, "\x1b");
        let state = handle_key(state, "h");
        let result = handle_key(state, "b");
        assert_eq!(
            result.doc.boxes,
            vec![node_with_children("a", vec![node(""), node(PAD)])]
        );
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![0], index: 1 }));
    }

    #[test]
    fn q_opens_the_save_prompt_and_preserves_boxes_and_selection() {
        let state = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "q");
        assert!(result.running);
        assert_eq!(result.doc.boxes, vec![node("a")]);
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 0 }));
        assert_eq!(result.mode, Mode::SavePrompt { filename: DEFAULT_FILENAME.to_string() });
    }

    #[test]
    fn q_with_a_file_to_save_to_stops_without_prompting() {
        let mut state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path { ancestors: vec![], index: 0 }),
        );
        state.save_to = Some("plans.dre".to_string());
        let result = handle_key(state, "q");
        assert!(!result.running);
        assert_eq!(result.save_to, Some("plans.dre".to_string()));
        assert_eq!(result.mode, Mode::Command);
        assert_eq!(result.doc.boxes, vec![node("a")]);
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 0 }));
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
        let state = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "s");
        assert_eq!(result.doc.boxes, vec![node("a"), node(PAD)]);
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 1 }));
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn s_on_a_child_box_appends_a_sibling_to_the_parents_children() {
        let boxes = vec![node_with_children("a", vec![node("b")])];
        let state = new_state(boxes, Mode::Command, Some(Path { ancestors: vec![0], index: 0 }));
        let result = handle_key(state, "s");
        assert_eq!(
            result.doc.boxes,
            vec![node_with_children("a", vec![node("b"), node(PAD)])]
        );
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![0], index: 1 }));
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
        let state = new_state(boxes, Mode::Command, Some(Path { ancestors: vec![0], index: 0 }));
        let result = handle_key(state, "h");
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 0 }));

        let result = handle_key(result, "h");
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 0 }));
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
        let state = new_state(boxes, Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "l");
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![0], index: 0 }));

        let state = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "l");
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 0 }));
    }

    #[test]
    fn j_and_k_move_between_siblings_with_bounds() {
        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "j");
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 1 }));

        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(Path { ancestors: vec![], index: 1 }));
        let result = handle_key(state, "j");
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 1 }));

        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(Path { ancestors: vec![], index: 1 }));
        let result = handle_key(state, "k");
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 0 }));

        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "k");
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 0 }));
    }

    #[test]
    fn i_enters_insert_and_appends_pad_to_the_selected_boxs_label() {
        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(Path { ancestors: vec![], index: 1 }));
        let result = handle_key(state, "i");
        assert_eq!(result.mode, Mode::Insert);
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 1 }));
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
        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(Path { ancestors: vec![], index: 1 }));
        let result = handle_key(state, "I");
        assert_eq!(result.mode, Mode::Insert);
        assert_eq!(result.doc.boxes, vec![node("a"), node(PAD)]);
    }

    #[test]
    fn cycle_colour_and_fill_advance_independently_and_cycle_back_to_plain() {
        let state = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "c");
        let mut expected = node("a");
        expected.colour = next_colour(None);
        assert_eq!(result.doc.boxes, vec![expected]);
        assert_eq!(result.doc.boxes[0].label, "a");

        let state = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "f");
        let mut expected = node("a");
        expected.fill = next_colour(None);
        assert_eq!(result.doc.boxes, vec![expected]);

        let mut state_boxed = vec![node("a")];
        let mut colour: Option<u8> = None;
        for _ in 0..=PALETTE_SIZE {
            let s = new_state(state_boxed.clone(), Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
            let result = handle_key(s, "c");
            state_boxed = result.doc.boxes;
            colour = state_boxed[0].colour;
        }
        assert_eq!(colour, None);
    }

    #[test]
    fn toggle_rounded_flips_and_flips_back() {
        let state = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "r");
        assert_eq!(result.doc.boxes[0].rounded, true);
        let state = new_state(result.doc.boxes.clone(), Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state, "r");
        assert_eq!(result.doc.boxes[0].rounded, false);
    }

    #[test]
    fn capital_c_on_a_top_level_box_does_nothing() {
        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(state.clone(), "C");
        assert_eq!(result.doc.boxes, state.doc.boxes);
        assert_eq!(result.doc.selected, state.doc.selected);
    }

    #[test]
    fn capital_f_on_a_top_level_box_does_nothing() {
        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
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
        let state = new_state(boxes, Mode::Command, Some(Path { ancestors: vec![0], index: 1 }));
        let result = handle_key(state, "C");
        let mut c = node("c");
        c.colour = next_colour(None);
        let mut d = node("d");
        d.colour = next_colour(None);
        assert_eq!(result.doc.boxes, vec![node_with_children("a", vec![c, d])]);
    }

    #[test]
    fn capital_f_advances_uniformly_filled_siblings_and_leaves_colour_untouched() {
        let mut c = node("c");
        c.colour = Some(1);
        let mut d = node("d");
        d.colour = Some(2);
        let boxes = vec![node_with_children("a", vec![c, d])];
        let state = new_state(boxes, Mode::Command, Some(Path { ancestors: vec![0], index: 1 }));
        let result = handle_key(state, "F");
        let mut expected_c = node("c");
        expected_c.colour = Some(1);
        expected_c.fill = next_colour(None);
        let mut expected_d = node("d");
        expected_d.colour = Some(2);
        expected_d.fill = next_colour(None);
        assert_eq!(result.doc.boxes, vec![node_with_children("a", vec![expected_c, expected_d])]);
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
        let before = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
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
        let state = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let after_command = handle_key(state, "c");
        let after_first_undo = handle_key(after_command, "u");
        let after_second_undo = handle_key(after_first_undo.clone(), "u");
        assert_eq!(after_second_undo.doc.boxes, after_first_undo.doc.boxes);
        assert_eq!(after_second_undo.doc.selected, after_first_undo.doc.selected);
    }

    #[test]
    fn movement_keys_do_not_clobber_an_existing_undo_snapshot() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let before = new_state(boxes, Mode::Command, Some(Path { ancestors: vec![0], index: 0 }));
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
    fn q_does_not_clobber_an_existing_undo_snapshot() {
        let before = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
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
        let start = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
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
        let start = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
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
    fn u_after_capital_i_restores_the_boxs_previous_label() {
        let before = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let after = handle_key(before.clone(), "I");
        let after_escape = handle_key(after, "\x1b");
        let undone = handle_key(after_escape, "u");
        assert_eq!(undone.doc.boxes, before.doc.boxes);
    }
}
