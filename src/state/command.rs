use crate::diagram::{children, parent_of};
use crate::state::action::Action;
use crate::state::history::undo;
use crate::state::{
    add_child_box, blank_box, colour_row, next_colour, Mode, State, DEFAULT_FILENAME, PAD,
};
use types::Tree;

pub(crate) fn min_depth(command: Action) -> usize {
    match command {
        Action::Undo | Action::NewBox | Action::Paste | Action::Quit => 0,
        Action::SelectParent | Action::CycleSiblingsColour | Action::ToggleSiblingsFill => 2,
        _ => 1,
    }
}

fn hide_idle_cursor(mut state: State) -> State {
    if state.mode == Mode::Command && state.selected.is_some() {
        state.last_selected = state.selected.clone();
        state.selected = None;
    }
    state
}

fn interrupt(mut state: State) -> State {
    state.save_to = None;
    state.running = false;
    state
}

fn enter_insert(mut state: State, path: Vec<usize>, base_label: &str) -> State {
    state.doc.root.value_mut(&path).label = format!("{base_label}{PAD}");
    state.selected = Some(path);
    state.mode = Mode::Insert;
    state
}

fn new_sibling(mut state: State, path: Vec<usize>) -> State {
    let sibling = state
        .doc
        .root
        .push(parent_of(&path), Tree::leaf(blank_box()));
    state.doc.root.value_mut(&sibling).label = PAD.to_string();
    state.selected = Some(sibling);
    state.mode = Mode::Insert;
    state
}

fn select_parent(mut state: State, mut path: Vec<usize>) -> State {
    if path.len() > 1 {
        path.pop();
    }
    state.selected = Some(path);
    state
}

fn select_child(mut state: State, path: Vec<usize>) -> State {
    state.selected = Some(state.doc.tree().child(&path));
    state
}

fn select_next(mut state: State, path: Vec<usize>) -> State {
    state.selected = Some(state.doc.tree().next(&path));
    state
}

fn select_previous(mut state: State, path: Vec<usize>) -> State {
    state.selected = Some(state.doc.tree().previous(&path));
    state
}

fn repeat(
    mut state: State,
    path: Vec<usize>,
    count: usize,
    step: fn(State, Vec<usize>) -> State,
) -> State {
    state.selected = Some(path);
    for _ in 0..count {
        let path = state.selected.clone().unwrap();
        state = step(state, path);
    }
    state
}

fn edit_label(state: State, path: Vec<usize>) -> State {
    let label = state.doc.tree().value(&path).label.clone();
    enter_insert(state, path, &label)
}

fn rename_label(mut state: State, path: Vec<usize>) -> State {
    state.doc.root.value_mut(&path).label = PAD.to_string();
    state.selected = Some(path);
    state.mode = Mode::Insert;
    state
}

fn cycle_colour(mut state: State, path: Vec<usize>) -> State {
    let node = state.doc.root.value_mut(&path);
    node.colour = next_colour(node.colour);
    state.selected = Some(path);
    state
}

fn cycle_siblings_colour(mut state: State, path: Vec<usize>) -> State {
    colour_row(&mut state.doc.root, &path);
    state.selected = Some(path);
    state
}

fn toggle_siblings_fill(mut state: State, path: Vec<usize>) -> State {
    let tree = &mut state.doc.root;
    let siblings: Vec<Vec<usize>> = children(tree, parent_of(&path)).collect();
    if siblings
        .iter()
        .all(|sibling| tree.value(sibling).colour.is_none())
    {
        state.selected = Some(path);
        return state;
    }
    let all_filled = siblings.iter().all(|sibling| tree.value(sibling).filled);
    for sibling in &siblings {
        tree.value_mut(sibling).filled = !all_filled;
    }
    state.selected = Some(path);
    state
}

fn toggle_fill(mut state: State, path: Vec<usize>) -> State {
    let node = state.doc.root.value_mut(&path);
    if node.colour.is_none() {
        state.selected = Some(path);
        return state;
    }
    node.filled = !node.filled;
    state.selected = Some(path);
    state
}

fn delete_box(mut state: State, path: Vec<usize>) -> State {
    state.clipboard = Some(state.doc.root.remove(&path));
    let (&last, parent) = path.split_last().expect("a selection is never the root");
    let remaining = children(state.doc.tree(), parent).count();
    state.selected = if remaining > 0 {
        Some([parent, &[last.min(remaining - 1)]].concat())
    } else {
        (!parent.is_empty()).then(|| parent.to_vec())
    };
    state
}

fn paste_box(mut state: State, selected: Option<Vec<usize>>, count: usize) -> State {
    let Some(branch) = state.clipboard.clone() else {
        state.selected = selected;
        return state;
    };
    let parent = selected.unwrap_or_default();
    let first = children(state.doc.tree(), &parent).count();
    for _ in 0..count {
        state.doc.root.push(&parent, branch.clone());
    }
    state.selected = Some([&parent[..], &[first + count - 1]].concat());
    state
}

fn toggle_rounded(mut state: State, path: Vec<usize>) -> State {
    let node = state.doc.root.value_mut(&path);
    node.rounded = !node.rounded;
    state.selected = Some(path);
    state
}

fn quit(mut state: State) -> State {
    if state.new_file && !state.doc.tree().contains(&[0]) {
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

fn reselect(mut state: State, selected: Option<Vec<usize>>) -> State {
    state.selected = selected;
    state
}

fn accumulate_digit(mut state: State, digit: u8) -> State {
    let count = state
        .pending_count
        .unwrap_or(0)
        .saturating_mul(10)
        .saturating_add(digit as usize);
    state.pending_count = Some(count);
    state
}

pub(crate) fn reduce(mut state: State, command: Action) -> State {
    match command {
        Action::Digit(digit) => return accumulate_digit(state, digit),
        Action::CancelCount => {
            state.pending_count = None;
            return state;
        }
        Action::Idle => return hide_idle_cursor(state),
        Action::Interrupt => return interrupt(state),
        _ => {}
    }
    let count = state.pending_count.take().unwrap_or(1);
    let depth = state.selected.as_ref().map_or(0, Vec::len);
    if depth < min_depth(command) {
        return state;
    }
    match (command, state.selected.take()) {
        (Action::Undo, selected) => undo(reselect(state, selected)),
        (Action::NewBox, selected) => add_child_box(state, selected),
        (Action::Quit, selected) => quit(reselect(state, selected)),
        (Action::NewSibling, Some(path)) => new_sibling(state, path),
        (Action::SelectParent, Some(path)) => repeat(state, path, count, select_parent),
        (Action::SelectChild, Some(path)) => select_child(state, path),
        (Action::SelectNext, Some(path)) => repeat(state, path, count, select_next),
        (Action::SelectPrevious, Some(path)) => repeat(state, path, count, select_previous),
        (Action::EditLabel, Some(path)) => edit_label(state, path),
        (Action::RenameLabel, Some(path)) => rename_label(state, path),
        (Action::CycleColour, Some(path)) => cycle_colour(state, path),
        (Action::CycleSiblingsColour, Some(path)) => cycle_siblings_colour(state, path),
        (Action::ToggleSiblingsFill, Some(path)) => toggle_siblings_fill(state, path),
        (Action::ToggleFill, Some(path)) => toggle_fill(state, path),
        (Action::Delete, Some(path)) => delete_box(state, path),
        (Action::Paste, selected) => paste_box(state, selected, count),
        (Action::ToggleRounded, Some(path)) => toggle_rounded(state, path),
        _ => state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{labelled, node, node_with_children, Node};
    use crate::palette::palette;
    use crate::state::apply as reduce;
    use crate::state::new_state;
    use crate::test_support::handle_key;

    const COMMANDS: [Action; 17] = [
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
        Action::Quit,
    ];

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
                _ => Some(vec![0; depth - 1]),
            };
            let state = new_state(boxes.clone(), Mode::Command, selected);
            let result = reduce(state.clone(), command);
            assert_eq!(result.doc, state.doc);
            assert_eq!(result.selected, state.selected);
        }
    }

    #[test]
    fn b_on_an_empty_canvas_appends_a_box_enters_insert_and_selects_it() {
        let state = new_state(vec![], Mode::Command, None);
        let result: State = handle_key(state, "b");
        assert_eq!(result.doc.root, Tree::root(vec![node(PAD)]));
        assert_eq!(result.mode, Mode::Insert);
        assert_eq!(result.selected, Some(vec![0]));
        assert!(result.running);
    }

    #[test]
    fn b_on_an_empty_canvas_does_not_mutate_the_given_state() {
        let state = new_state(vec![], Mode::Command, None);
        handle_key(state.clone(), "b");
        assert_eq!(state.doc.root, Tree::root(vec![]));
    }

    #[test]
    fn b_on_a_selected_box_appends_and_selects_a_child() {
        let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let result = handle_key(state, "b");
        assert_eq!(
            result.doc.root,
            Tree::root(vec![node_with_children("a", vec![node(PAD)])])
        );
        assert_eq!(result.selected, Some(vec![0, 0]));
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn a_second_b_on_the_same_parent_places_a_second_child() {
        let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let state = handle_key(state, "b");
        let state = handle_key(state, "\x1b");
        let state = handle_key(state, "h");
        let result = handle_key(state, "b");
        assert_eq!(
            result.doc.root,
            Tree::root(vec![node_with_children("a", vec![node(""), node(PAD)])])
        );
        assert_eq!(result.selected, Some(vec![0, 1]));
    }

    #[test]
    fn q_opens_the_save_prompt_and_preserves_boxes_and_selection() {
        let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let result = handle_key(state, "q");
        assert!(result.running);
        assert_eq!(result.doc.root, Tree::root(vec![node("a")]));
        assert_eq!(result.selected, Some(vec![0]));
        assert_eq!(
            result.mode,
            Mode::SavePrompt {
                filename: DEFAULT_FILENAME.to_string()
            }
        );
    }

    #[test]
    fn q_with_a_file_to_save_to_stops_without_prompting() {
        let mut state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        state.save_to = Some("plans.dre".to_string());
        let result = handle_key(state, "q");
        assert!(!result.running);
        assert_eq!(result.save_to, Some("plans.dre".to_string()));
        assert_eq!(result.mode, Mode::Command);
        assert_eq!(result.doc.root, Tree::root(vec![node("a")]));
        assert_eq!(result.selected, Some(vec![0]));
    }

    fn new_file_state(boxes: Vec<Tree<Node>>) -> State {
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
        let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let result = handle_key(state, "s");
        assert_eq!(result.doc.root, Tree::root(vec![node("a"), node(PAD)]));
        assert_eq!(result.selected, Some(vec![1]));
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn s_on_a_child_box_appends_a_sibling_to_the_parents_children() {
        let boxes = vec![node_with_children("a", vec![node("b")])];
        let state = new_state(boxes, Mode::Command, Some(vec![0, 0]));
        let result = handle_key(state, "s");
        assert_eq!(
            result.doc.root,
            Tree::root(vec![node_with_children("a", vec![node("b"), node(PAD)])])
        );
        assert_eq!(result.selected, Some(vec![0, 1]));
    }

    #[test]
    fn s_with_no_selection_returns_the_state_unchanged() {
        let state = new_state(vec![node("a")], Mode::Command, None);
        let result = handle_key(state, "s");
        assert_eq!(result.doc.root, Tree::root(vec![node("a")]));
        assert_eq!(result.selected, None);
    }

    #[test]
    fn h_selects_the_parent_and_is_a_no_op_at_the_top() {
        let boxes = vec![node_with_children("a", vec![node("c")])];
        let state = new_state(boxes, Mode::Command, Some(vec![0, 0]));
        let result = handle_key(state, "h");
        assert_eq!(result.selected, Some(vec![0]));

        let result = handle_key(result, "h");
        assert_eq!(result.selected, Some(vec![0]));
    }

    #[test]
    fn h_on_an_empty_canvas_returns_the_state_unchanged() {
        let state = new_state(vec![], Mode::Command, None);
        let result = handle_key(state, "h");
        assert_eq!(result.selected, None);
    }

    #[test]
    fn l_selects_the_first_child_or_keeps_selection_with_none() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let state = new_state(boxes, Mode::Command, Some(vec![0]));
        let result = handle_key(state, "l");
        assert_eq!(result.selected, Some(vec![0, 0]));

        let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let result = handle_key(state, "l");
        assert_eq!(result.selected, Some(vec![0]));
    }

    #[test]
    fn j_and_k_move_between_siblings_with_bounds() {
        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(vec![0]));
        let result = handle_key(state, "j");
        assert_eq!(result.selected, Some(vec![1]));

        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(vec![1]));
        let result = handle_key(state, "j");
        assert_eq!(result.selected, Some(vec![1]));

        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(vec![1]));
        let result = handle_key(state, "k");
        assert_eq!(result.selected, Some(vec![0]));

        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(vec![0]));
        let result = handle_key(state, "k");
        assert_eq!(result.selected, Some(vec![0]));
    }

    #[test]
    fn j_with_a_count_prefix_moves_multiple_steps_forward() {
        let boxes: Vec<Tree<Node>> = (0..7).map(|i| node(&i.to_string())).collect();
        let state = new_state(boxes, Mode::Command, Some(vec![0]));
        let result = handle_key(handle_key(state, "3"), "j");
        assert_eq!(result.selected, Some(vec![3]));
    }

    #[test]
    fn k_with_a_count_prefix_moves_multiple_steps_back() {
        let boxes: Vec<Tree<Node>> = (0..7).map(|i| node(&i.to_string())).collect();
        let state = new_state(boxes, Mode::Command, Some(vec![6]));
        let result = handle_key(handle_key(state, "3"), "k");
        assert_eq!(result.selected, Some(vec![3]));
    }

    #[test]
    fn j_with_a_count_prefix_clamps_at_the_last_sibling() {
        let boxes: Vec<Tree<Node>> = (0..5).map(|i| node(&i.to_string())).collect();
        let state = new_state(boxes, Mode::Command, Some(vec![3]));
        let result = handle_key(handle_key(state, "3"), "j");
        assert_eq!(result.selected, Some(vec![4]));
    }

    #[test]
    fn k_with_a_count_prefix_clamps_at_the_first_sibling() {
        let boxes: Vec<Tree<Node>> = (0..5).map(|i| node(&i.to_string())).collect();
        let state = new_state(boxes, Mode::Command, Some(vec![1]));
        let result = handle_key(handle_key(state, "3"), "k");
        assert_eq!(result.selected, Some(vec![0]));
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
        let state = new_state(boxes, Mode::Command, Some(vec![0, 0, 0, 0]));
        let result = handle_key(handle_key(state, "3"), "h");
        assert_eq!(result.selected, Some(vec![0]));
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
        let state = new_state(boxes, Mode::Command, Some(vec![0, 0, 0]));
        let state = handle_key(state, "1");
        let state = handle_key(state, "2");
        let result = handle_key(state, "h");
        assert_eq!(result.selected, Some(vec![0]));
    }

    #[test]
    fn one_j_moves_one_step_like_plain_j() {
        let boxes = vec![node("a"), node("b"), node("c")];
        let plain = new_state(boxes.clone(), Mode::Command, Some(vec![0]));
        let prefixed = new_state(boxes, Mode::Command, Some(vec![0]));
        let plain = handle_key(plain, "j");
        let prefixed = handle_key(handle_key(prefixed, "1"), "j");
        assert_eq!(plain.selected, prefixed.selected);
        assert_eq!(prefixed.selected, Some(vec![1]));
    }

    #[test]
    fn twelve_j_moves_twelve_steps() {
        let boxes: Vec<Tree<Node>> = (0..15).map(|i| node(&i.to_string())).collect();
        let state = new_state(boxes, Mode::Command, Some(vec![0]));
        let state = handle_key(state, "1");
        let state = handle_key(state, "2");
        let result = handle_key(state, "j");
        assert_eq!(result.selected, Some(vec![12]));
    }

    #[test]
    fn zero_j_leaves_the_selection_unchanged() {
        let boxes = vec![node("a"), node("b"), node("c")];
        let state = new_state(boxes, Mode::Command, Some(vec![1]));
        let result = handle_key(handle_key(state, "0"), "j");
        assert_eq!(result.selected, Some(vec![1]));
    }

    #[test]
    fn zero_k_leaves_the_selection_unchanged() {
        let boxes = vec![node("a"), node("b"), node("c")];
        let state = new_state(boxes, Mode::Command, Some(vec![1]));
        let result = handle_key(handle_key(state, "0"), "k");
        assert_eq!(result.selected, Some(vec![1]));
    }

    #[test]
    fn zero_h_leaves_the_selection_unchanged() {
        let boxes = vec![node_with_children("root", vec![node("a")])];
        let state = new_state(boxes, Mode::Command, Some(vec![0, 0]));
        let result = handle_key(handle_key(state, "0"), "h");
        assert_eq!(result.selected, Some(vec![0, 0]));
    }

    #[test]
    fn a_count_prefix_on_s_behaves_exactly_as_s_and_is_consumed() {
        let boxes = vec![node("a"), node("b"), node("c"), node("d")];
        let plain = new_state(boxes.clone(), Mode::Command, Some(vec![2]));
        let prefixed = new_state(boxes, Mode::Command, Some(vec![2]));
        let plain = handle_key(plain, "s");
        let prefixed = handle_key(handle_key(prefixed, "2"), "s");
        assert_eq!(plain.doc.root, prefixed.doc.root);
        assert_eq!(plain.selected, prefixed.selected);
        assert_eq!(plain.mode, prefixed.mode);
        assert_eq!(prefixed.selected, Some(vec![4]));

        let committed = handle_key(prefixed, "\x1b");
        let result = handle_key(committed, "k");
        assert_eq!(result.selected, Some(vec![3]));
    }

    #[test]
    fn a_digit_followed_by_an_unknown_key_then_j_moves_one_step() {
        let boxes: Vec<Tree<Node>> = (0..5).map(|i| node(&i.to_string())).collect();
        let state = new_state(boxes, Mode::Command, Some(vec![0]));
        let state = handle_key(state, "3");
        let state = handle_key(state, "x");
        let result = handle_key(state, "j");
        assert_eq!(result.selected, Some(vec![1]));
    }

    #[test]
    fn j_with_a_count_prefix_and_no_selection_leaves_the_document_unchanged() {
        let boxes: Vec<Tree<Node>> = (0..5).map(|i| node(&i.to_string())).collect();
        let state = new_state(boxes.clone(), Mode::Command, None);
        let state = handle_key(state, "3");
        let result = handle_key(state, "j");
        assert_eq!(result.doc.root, Tree::root(boxes));
        assert_eq!(result.selected, None);
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
        assert_eq!(result.selected, Some(vec![2]));
    }

    #[test]
    fn a_count_consumed_by_failed_min_depth_does_not_leak_to_the_next_command() {
        let boxes: Vec<Tree<Node>> = (0..5).map(|i| node(&i.to_string())).collect();
        let state = new_state(boxes, Mode::Command, Some(vec![0]));
        let state = handle_key(state, "3");
        let state = handle_key(state, "h");
        let result = handle_key(state, "j");
        assert_eq!(result.selected, Some(vec![1]));
    }

    #[test]
    fn i_enters_insert_and_appends_pad_to_the_selected_boxs_label() {
        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(vec![1]));
        let result = handle_key(state, "i");
        assert_eq!(result.mode, Mode::Insert);
        assert_eq!(result.selected, Some(vec![1]));
        assert_eq!(
            result.doc.root,
            Tree::root(vec![node("a"), node(&format!("b{PAD}"))])
        );
    }

    #[test]
    fn i_on_an_empty_canvas_returns_the_state_unchanged() {
        let state = new_state(vec![], Mode::Command, None);
        let result = handle_key(state, "i");
        assert_eq!(result.mode, Mode::Command);
        assert_eq!(result.doc.root, Tree::root(vec![]));
    }

    #[test]
    fn capital_i_clears_the_selected_boxs_label() {
        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(vec![1]));
        let result = handle_key(state, "I");
        assert_eq!(result.mode, Mode::Insert);
        assert_eq!(result.doc.root, Tree::root(vec![node("a"), node(PAD)]));
    }

    #[test]
    fn a_digit_after_c_is_a_count_not_a_colour() {
        let boxes: Vec<Tree<Node>> = (0..3).map(|i| node(&i.to_string())).collect();
        let state = new_state(boxes, Mode::Command, Some(vec![0]));
        let cycled = handle_key(state, "c");
        let cycled_colour = cycled.doc.tree().value(&[0]).colour;
        let result = handle_key(handle_key(cycled, "2"), "j");
        assert_eq!(result.doc.tree().value(&[0]).colour, cycled_colour);
        assert_eq!(result.selected, Some(vec![2]));
    }

    #[test]
    fn cycle_colour_and_toggle_fill_advance_independently() {
        let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let result = handle_key(state, "c");
        let mut expected = labelled("a");
        expected.colour = next_colour(None);
        assert_eq!(result.doc.root, Tree::root(vec![Tree::leaf(expected)]));
        assert_eq!(result.doc.tree().value(&[0]).label, "a");

        let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let colourised = handle_key(state, "c");
        let state = State {
            doc: colourised.doc,
            ..new_state(vec![], Mode::Command, Some(vec![0]))
        };
        let result = handle_key(state, "f");
        let mut expected = labelled("a");
        expected.colour = next_colour(None);
        expected.filled = true;
        assert_eq!(result.doc.root, Tree::root(vec![Tree::leaf(expected)]));

        let mut state_boxed = new_state(vec![node("a")], Mode::Command, Some(vec![0])).doc;
        let mut colour: Option<u8> = None;
        let presses_past_the_last_colour = (0..).take_while(|&i| palette(i).is_some()).count() + 1;
        for _ in 0..presses_past_the_last_colour {
            let s = State {
                doc: state_boxed.clone(),
                ..new_state(vec![], Mode::Command, Some(vec![0]))
            };
            let result = handle_key(s, "c");
            state_boxed = result.doc;
            colour = state_boxed.tree().value(&[0]).colour;
        }
        assert_eq!(colour, None);
    }

    #[test]
    fn f_toggles_fill_on_and_off() {
        let colour = next_colour(None);
        let mut colourised = labelled("a");
        colourised.colour = colour;
        let state = new_state(
            vec![Tree::leaf(colourised.clone())],
            Mode::Command,
            Some(vec![0]),
        );
        let result = handle_key(state, "f");
        let mut filled = labelled("a");
        filled.colour = colour;
        filled.filled = true;
        assert_eq!(result.doc.root, Tree::root(vec![Tree::leaf(filled)]));

        let state = State {
            doc: result.doc.clone(),
            ..new_state(vec![], Mode::Command, Some(vec![0]))
        };
        let result = handle_key(state, "f");
        assert_eq!(result.doc.root, Tree::root(vec![Tree::leaf(colourised)]));
    }

    #[test]
    fn f_on_a_colourless_box_does_nothing() {
        let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let result = handle_key(state.clone(), "f");
        assert_eq!(result.doc.root, state.doc.root);
        assert_eq!(result.selected, state.selected);
    }

    #[test]
    fn a_toggled_fill_survives_a_colour_cycle_back_to_plain() {
        let mut state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        for _ in (0..).take_while(|&i| palette(i).is_some()) {
            state = handle_key(state, "c");
        }
        assert_ne!(state.doc.tree().value(&[0]).colour, None);
        state = handle_key(state, "f");
        assert!(state.doc.tree().value(&[0]).filled);
        let plain = handle_key(state, "c");
        assert_eq!(plain.doc.tree().value(&[0]).colour, None);
        assert!(plain.doc.tree().value(&[0]).filled);
        let re_coloured = handle_key(plain, "c");
        assert_ne!(re_coloured.doc.tree().value(&[0]).colour, None);
        assert!(re_coloured.doc.tree().value(&[0]).filled);
    }

    #[test]
    fn toggle_rounded_flips_and_flips_back() {
        let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let result = handle_key(state, "r");
        assert!(result.doc.tree().value(&[0]).rounded);
        let state = State {
            doc: result.doc.clone(),
            ..new_state(vec![], Mode::Command, Some(vec![0]))
        };
        let result = handle_key(state, "r");
        assert!(!result.doc.tree().value(&[0]).rounded);
    }

    #[test]
    fn capital_c_on_a_top_level_box_does_nothing() {
        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(vec![0]));
        let result = handle_key(state.clone(), "C");
        assert_eq!(result.doc.root, state.doc.root);
        assert_eq!(result.selected, state.selected);
    }

    #[test]
    fn capital_f_on_a_top_level_box_does_nothing() {
        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(vec![0]));
        let result = handle_key(state.clone(), "F");
        assert_eq!(result.doc.root, state.doc.root);
        assert_eq!(result.selected, state.selected);
    }

    #[test]
    fn capital_f_with_nothing_selected_does_nothing() {
        let state = new_state(vec![node("a")], Mode::Command, None);
        let result = handle_key(state.clone(), "F");
        assert_eq!(result.doc.root, state.doc.root);
        assert_eq!(result.selected, state.selected);
    }

    #[test]
    fn capital_c_advances_uniformly_coloured_siblings() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let state = new_state(boxes, Mode::Command, Some(vec![0, 1]));
        let result = handle_key(state, "C");
        let mut c = labelled("c");
        c.colour = next_colour(None);
        let mut d = labelled("d");
        d.colour = next_colour(None);
        assert_eq!(
            result.doc.root,
            Tree::root(vec![node_with_children(
                "a",
                vec![Tree::leaf(c), Tree::leaf(d)]
            )])
        );
    }

    #[test]
    fn capital_f_toggles_every_sibling_fill_and_leaves_colour_untouched() {
        let colour_one = next_colour(None);
        let colour_two = next_colour(colour_one);
        let mut c = labelled("c");
        c.colour = colour_one;
        let mut d = labelled("d");
        d.colour = colour_two;
        let boxes = vec![node_with_children("a", vec![Tree::leaf(c), Tree::leaf(d)])];
        let state = new_state(boxes, Mode::Command, Some(vec![0, 1]));
        let result = handle_key(state, "F");
        assert!(result.doc.tree().value(&[0, 0]).filled);
        assert_eq!(result.doc.tree().value(&[0, 0]).colour, colour_one);
        assert!(result.doc.tree().value(&[0, 1]).filled);
        assert_eq!(result.doc.tree().value(&[0, 1]).colour, colour_two);
        assert_eq!(result.selected, Some(vec![0, 1]));

        let state = State {
            doc: result.doc.clone(),
            ..new_state(vec![], Mode::Command, Some(vec![0, 1]))
        };
        let result = handle_key(state, "F");
        assert!(!result.doc.tree().value(&[0, 0]).filled);
        assert_eq!(result.doc.tree().value(&[0, 0]).colour, colour_one);
        assert!(!result.doc.tree().value(&[0, 1]).filled);
        assert_eq!(result.doc.tree().value(&[0, 1]).colour, colour_two);
    }

    #[test]
    fn capital_f_fills_every_sibling_including_colourless_ones() {
        let colour = next_colour(None);
        let mut c = labelled("c");
        c.colour = colour;
        let d = node("d");
        let boxes = vec![node_with_children("a", vec![Tree::leaf(c), d])];
        let state = new_state(boxes, Mode::Command, Some(vec![0, 0]));
        let result = handle_key(state, "F");
        assert!(result.doc.tree().value(&[0, 0]).filled);
        assert_eq!(result.doc.tree().value(&[0, 0]).colour, colour);
        assert!(result.doc.tree().value(&[0, 1]).filled);
        assert_eq!(result.doc.tree().value(&[0, 1]).colour, None);
    }

    #[test]
    fn capital_f_does_nothing_when_every_sibling_is_colourless() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let state = new_state(boxes, Mode::Command, Some(vec![0, 1]));
        let result = handle_key(state.clone(), "F");
        assert_eq!(result.doc.root, state.doc.root);
        assert_eq!(result.selected, state.selected);
    }

    #[test]
    fn u_after_b_restores_boxes_and_command_mode() {
        let before = new_state(vec![], Mode::Command, None);
        let after = handle_key(before.clone(), "b");
        let after_escape = handle_key(after, "\x1b");
        let undone = handle_key(after_escape, "u");
        assert_eq!(undone.doc.root, before.doc.root);
        assert_eq!(undone.mode, before.mode);
    }

    #[test]
    fn u_keeps_the_selection_when_the_selected_box_still_exists() {
        let before = selecting(vec![node("a"), node("b")], &[0]);
        let moved = press(before.clone(), &["c", "j"]);
        assert_ne!(moved.selected, before.selected);
        let undone = handle_key(moved.clone(), "u");
        assert_eq!(undone.doc.root, before.doc.root);
        assert_eq!(undone.selected, moved.selected);
    }

    #[test]
    fn u_selects_the_parent_when_the_undone_edit_created_the_selected_box() {
        let parent = selecting(vec![node("a")], &[0]);
        let undone = press(parent.clone(), &["b", "\x1b", "u"]);
        assert_eq!(undone.doc.root, parent.doc.root);
        assert_eq!(undone.selected, parent.selected);
    }

    #[test]
    fn u_selects_nothing_when_the_undone_edit_created_the_selected_top_level_box() {
        let before = selecting(vec![node("a")], &[0]);
        let undone = press(before.clone(), &["s", "\x1b", "u"]);
        assert_eq!(undone.doc.root, before.doc.root);
        assert_eq!(undone.selected, None);
    }

    #[test]
    fn u_after_c_restores_boxes() {
        let before = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let after = handle_key(before.clone(), "c");
        let undone = handle_key(after, "u");
        assert_eq!(undone.doc.root, before.doc.root);
    }

    fn coloured_box_a_selected() -> State {
        let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        handle_key(state, "c")
    }

    fn press(state: State, keys: &[&str]) -> State {
        keys.iter().fold(state, |state, key| handle_key(state, key))
    }

    fn cache_box_selected() -> State {
        new_state(vec![node("Cache")], Mode::Command, Some(vec![0]))
    }

    #[test]
    fn s_makes_the_sibling_and_its_text_one_undo_step() {
        let state = press(
            cache_box_selected(),
            &["s", "Q", "u", "e", "u", "e", "\x1b"],
        );
        assert_eq!(
            state.doc.root,
            Tree::root(vec![node("Cache"), node("Queue")])
        );
        let state = handle_key(state, "u");
        assert_eq!(state.doc.root, Tree::root(vec![node("Cache")]));
        let state = handle_key(state, "u");
        assert_eq!(state.doc.root, Tree::root(vec![node("Cache")]));
    }

    #[test]
    fn s_then_esc_immediately_leaves_only_the_sibling_as_a_step() {
        let state = press(cache_box_selected(), &["s", "\x1b"]);
        assert_eq!(state.doc.root, Tree::root(vec![node("Cache"), node("")]));
        let state = handle_key(state, "u");
        assert_eq!(state.doc.root, Tree::root(vec![node("Cache")]));
        let state = handle_key(state, "u");
        assert_eq!(state.doc.root, Tree::root(vec![node("Cache")]));
    }

    #[test]
    fn capital_i_makes_the_clearing_and_the_new_text_one_undo_step() {
        let state = press(
            cache_box_selected(),
            &["I", "R", "e", "d", "i", "s", "\x1b"],
        );
        assert_eq!(state.doc.root, Tree::root(vec![node("Redis")]));
        let state = handle_key(state, "u");
        assert_eq!(state.doc.root, Tree::root(vec![node("Cache")]));
        let state = handle_key(state, "u");
        assert_eq!(state.doc.root, Tree::root(vec![node("Cache")]));
    }

    #[test]
    fn capital_i_then_esc_immediately_leaves_only_the_clearing_as_a_step() {
        let state = press(cache_box_selected(), &["I", "\x1b"]);
        assert_eq!(state.doc.root, Tree::root(vec![node("")]));
        let state = handle_key(state, "u");
        assert_eq!(state.doc.root, Tree::root(vec![node("Cache")]));
    }

    #[test]
    fn an_i_edit_session_is_undone_by_one_u() {
        let coloured = coloured_box_a_selected();
        let edited = press(coloured.clone(), &["i", "x", "y", "\x7f", "z", "\x1b"]);
        assert_eq!(edited.doc.tree().value(&[0]).label, "axz");
        let undone = handle_key(edited, "u");
        assert_eq!(undone.doc.root, coloured.doc.root);
    }

    #[test]
    fn i_then_esc_immediately_is_not_an_undo_step() {
        let coloured = coloured_box_a_selected();
        let undone = press(coloured.clone(), &["i", "\x1b", "u"]);
        assert_eq!(undone.doc.tree().value(&[0]).colour, None);
        assert_eq!(
            undone.doc.tree().value(&[0]).label,
            coloured.doc.tree().value(&[0]).label
        );
    }

    #[test]
    fn i_typing_then_backspacing_back_to_the_original_is_not_an_undo_step() {
        let coloured = coloured_box_a_selected();
        let undone = press(
            coloured.clone(),
            &["i", "x", "y", "\x7f", "\x7f", "\x1b", "u"],
        );
        assert_eq!(undone.doc.tree().value(&[0]).colour, None);
        assert_eq!(
            undone.doc.tree().value(&[0]).label,
            coloured.doc.tree().value(&[0]).label
        );
    }

    #[test]
    fn u_after_c_with_nothing_selected_is_a_no_op() {
        let state = new_state(vec![node("a")], Mode::Command, None);
        let after = handle_key(state.clone(), "c");
        let undone = handle_key(after, "u");
        assert_eq!(undone.doc.root, state.doc.root);
        assert_eq!(undone.selected, state.selected);
    }

    #[test]
    fn u_with_no_previous_action_leaves_state_unchanged() {
        let state = new_state(vec![], Mode::Command, None);
        let result = handle_key(state, "u");
        assert_eq!(result.doc.root, Tree::root(vec![]));
        assert_eq!(result.selected, None);
    }

    #[test]
    fn u_twice_in_a_row_does_not_redo() {
        let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let after_command = handle_key(state, "c");
        let after_first_undo = handle_key(after_command, "u");
        let after_second_undo = handle_key(after_first_undo.clone(), "u");
        assert_eq!(after_second_undo.doc.root, after_first_undo.doc.root);
        assert_eq!(after_second_undo.selected, after_first_undo.selected);
    }

    #[test]
    fn movement_keys_do_not_clobber_an_existing_undo_snapshot() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let before = new_state(boxes, Mode::Command, Some(vec![0, 0]));
        let after_command = handle_key(before.clone(), "c");
        let navigated = handle_key(after_command, "j");
        let navigated = handle_key(navigated, "h");
        let navigated = handle_key(navigated, "l");
        let navigated = handle_key(navigated, "k");
        let undone = handle_key(navigated.clone(), "u");
        assert_eq!(undone.doc.root, before.doc.root);
        assert_eq!(undone.selected, navigated.selected);
    }

    #[test]
    fn count_prefixed_movement_does_not_clobber_an_existing_undo_snapshot() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let before = new_state(boxes, Mode::Command, Some(vec![0, 0]));
        let after_command = handle_key(before.clone(), "c");
        let navigated = handle_key(after_command.clone(), "j");
        let navigated = handle_key(handle_key(navigated, "2"), "h");
        let navigated = handle_key(navigated, "k");
        let navigated = handle_key(handle_key(navigated, "2"), "l");
        let undone = handle_key(navigated.clone(), "u");
        assert_eq!(undone.doc.root, before.doc.root);
        assert_eq!(undone.selected, navigated.selected);
    }

    #[test]
    fn q_does_not_clobber_an_existing_undo_snapshot() {
        let before = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let after_command = handle_key(before.clone(), "c");
        let mut after_quit = handle_key(after_command, "q");
        after_quit.mode = Mode::Command;
        let undone = handle_key(after_quit, "u");
        assert_eq!(undone.doc.root, before.doc.root);
        assert_eq!(undone.selected, before.selected);
    }

    #[test]
    fn u_after_an_insert_session_undoes_the_b_that_started_it() {
        let before = new_state(vec![], Mode::Command, None);
        let after_b = handle_key(before.clone(), "b");
        let after_typing = handle_key(after_b, "h");
        let after_typing = handle_key(after_typing, "i");
        let after_escape = handle_key(after_typing, "\x1b");
        let undone = handle_key(after_escape, "u");
        assert_eq!(undone.doc.root, before.doc.root);
        assert_eq!(undone.selected, before.selected);
    }

    #[test]
    fn repeated_u_walks_back_through_every_undoable_command() {
        let start = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let after_colour = handle_key(start.clone(), "c");
        let after_fill = handle_key(after_colour.clone(), "f");
        let after_rounded = handle_key(after_fill.clone(), "r");
        let undone = handle_key(after_rounded, "u");
        assert_eq!(undone.doc.root, after_fill.doc.root);
        let undone = handle_key(undone, "u");
        assert_eq!(undone.doc.root, after_colour.doc.root);
        let undone = handle_key(undone, "u");
        assert_eq!(undone.doc.root, start.doc.root);
        assert_eq!(undone.selected, start.selected);
    }

    #[test]
    fn u_on_an_exhausted_history_leaves_the_state_unchanged() {
        let start = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let after_colour = handle_key(start.clone(), "c");
        let after_fill = handle_key(after_colour, "f");
        let undone = handle_key(after_fill, "u");
        let undone = handle_key(undone, "u");
        let exhausted = handle_key(undone, "u");
        assert_eq!(exhausted.doc.root, start.doc.root);
        assert_eq!(exhausted.selected, start.selected);
        assert_eq!(exhausted.mode, start.mode);
        assert_eq!(exhausted.running, start.running);
    }

    #[test]
    fn u_after_capital_i_restores_the_boxs_previous_label() {
        let before = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let after = handle_key(before.clone(), "I");
        let after_escape = handle_key(after, "\x1b");
        let undone = handle_key(after_escape, "u");
        assert_eq!(undone.doc.root, before.doc.root);
    }

    #[test]
    fn d_deletes_the_selected_box_and_u_brings_it_back_with_its_children() {
        let boxes = vec![node_with_children(
            "Shop",
            vec![
                node_with_children("Payments", vec![node("Card"), node("Invoice")]),
                node("Orders"),
            ],
        )];
        let before = new_state(boxes, Mode::Command, Some(vec![0, 0]));
        let deleted = handle_key(before.clone(), "d");
        assert_eq!(
            deleted.doc.root,
            Tree::root(vec![node_with_children("Shop", vec![node("Orders")])])
        );
        assert_eq!(deleted.selected, Some(vec![0, 0]));
        assert_eq!(deleted.mode, Mode::Command);
        let undone = handle_key(deleted.clone(), "u");
        assert_eq!(undone.doc.root, before.doc.root);
        assert_eq!(undone.selected, deleted.selected);
    }

    #[test]
    fn d_with_nothing_selected_changes_nothing_and_pushes_no_history() {
        let start = new_state(vec![node("a")], Mode::Command, None);
        let mut coloured = start.clone();
        coloured.selected = Some(vec![0]);
        let coloured = handle_key(coloured, "c");
        let mut deselected = coloured.clone();
        deselected.selected = None;
        let after_d = handle_key(deselected.clone(), "d");
        assert_eq!(after_d.doc, deselected.doc);
        assert_eq!(after_d.selected, deselected.selected);
        let undone = handle_key(after_d, "u");
        assert_eq!(undone.doc.root, start.doc.root);
    }

    #[test]
    fn a_count_before_d_still_deletes_only_one_box() {
        let boxes = vec![node("a"), node("b"), node("c"), node("d")];
        let state = new_state(boxes, Mode::Command, Some(vec![1]));
        let result = handle_key(handle_key(state, "3"), "d");
        assert_eq!(
            result.doc.root,
            Tree::root(vec![node("a"), node("c"), node("d")])
        );
        assert_eq!(result.pending_count, None);
    }

    fn selecting(boxes: Vec<Tree<Node>>, path: &[usize]) -> State {
        new_state(boxes, Mode::Command, Some(path.to_vec()))
    }

    #[test]
    fn d_selects_the_next_sibling_at_the_same_path() {
        let result = handle_key(selecting(vec![node("a"), node("b"), node("c")], &[1]), "d");
        assert_eq!(result.doc.root, Tree::root(vec![node("a"), node("c")]));
        assert_eq!(result.selected, Some(vec![1]));
    }

    #[test]
    fn d_on_the_last_sibling_selects_the_previous_sibling() {
        let result = handle_key(selecting(vec![node("a"), node("b"), node("c")], &[2]), "d");
        assert_eq!(result.doc.root, Tree::root(vec![node("a"), node("b")]));
        assert_eq!(result.selected, Some(vec![1]));
    }

    #[test]
    fn d_on_an_only_child_selects_the_parent() {
        let boxes = vec![
            node("a"),
            node_with_children("b", vec![node_with_children("c", vec![node("d")])]),
        ];
        let result = handle_key(selecting(boxes, &[1, 0, 0]), "d");
        assert_eq!(
            result.doc.root,
            Tree::root(vec![node("a"), node_with_children("b", vec![node("c")])])
        );
        assert_eq!(result.selected, Some(vec![1, 0]));
    }

    #[test]
    fn d_on_the_only_top_level_box_selects_nothing() {
        let result = handle_key(selecting(vec![node("a")], &[0]), "d");
        assert_eq!(result.doc.root, Tree::root(vec![]));
        assert_eq!(result.selected, None);
    }

    #[test]
    fn d_puts_the_box_and_its_descendants_on_the_clipboard() {
        let payments = node_with_children("Payments", vec![node("Stripe")]);
        let boxes = vec![node_with_children(
            "API gateway",
            vec![node("Auth"), payments.clone()],
        )];
        let result = handle_key(selecting(boxes, &[0, 1]), "d");
        assert_eq!(result.clipboard, Some(payments));
    }

    #[test]
    fn a_second_d_replaces_the_clipboard() {
        let boxes = vec![node("a"), node("b")];
        let result = handle_key(handle_key(selecting(boxes, &[0]), "d"), "d");
        assert_eq!(result.clipboard, Some(node("b")));
    }

    #[test]
    fn u_after_d_restores_the_box_and_leaves_the_clipboard_filled() {
        let boxes = vec![node_with_children("a", vec![node("b")]), node("c")];
        let before = selecting(boxes, &[0]);
        let undone = handle_key(handle_key(before.clone(), "d"), "u");
        assert_eq!(undone.doc, before.doc);
        assert_eq!(
            undone.clipboard,
            Some(node_with_children("a", vec![node("b")]))
        );
    }

    fn story() -> State {
        let payments = node_with_children("Payments", vec![node("Stripe")]);
        let boxes = vec![node_with_children(
            "Orders",
            vec![payments, node("Refunds")],
        )];
        selecting(boxes, &[0, 0])
    }

    fn orders_children(state: &State) -> Vec<Tree<Node>> {
        let mut orders = state.doc.root.clone();
        children(state.doc.tree(), &[0])
            .map(|_| orders.remove(&[0, 0]))
            .collect()
    }

    #[test]
    fn p_appends_the_cut_branch_as_the_last_child_of_the_selected_box() {
        let payments = node_with_children("Payments", vec![node("Stripe")]);
        let cut = handle_key(story(), "d");
        let selected = handle_key(cut, "h");
        let result = handle_key(selected, "p");
        let children = orders_children(&result);
        assert_eq!(children.last(), Some(&payments));
        assert_eq!(result.selected, Some(vec![0, children.len() - 1]));
    }

    #[test]
    fn pasting_again_gives_a_second_copy() {
        let payments = node_with_children("Payments", vec![node("Stripe")]);
        let cut = handle_key(handle_key(story(), "d"), "h");
        let result = handle_key(handle_key(handle_key(cut, "p"), "h"), "p");
        let children = orders_children(&result);
        assert_eq!(children[children.len() - 2..], [payments.clone(), payments]);
        assert_eq!(result.selected, Some(vec![0, children.len() - 1]));
    }

    #[test]
    fn u_after_p_removes_the_branch_and_selects_the_box_it_was_pasted_into() {
        let cut = handle_key(handle_key(story(), "d"), "h");
        let undone = handle_key(handle_key(cut.clone(), "p"), "u");
        assert_eq!(undone.doc, cut.doc);
        assert_eq!(undone.selected, cut.selected);
    }

    #[test]
    fn p_with_an_empty_clipboard_changes_nothing_and_leaves_history_alone() {
        let before = handle_key(story(), "r");
        let pasted = handle_key(before.clone(), "p");
        assert_eq!(pasted.doc, before.doc);
        let undone = handle_key(pasted, "u");
        assert_eq!(undone.doc, story().doc);
    }

    #[test]
    fn p_after_deleting_the_only_top_level_box_restores_and_selects_it() {
        let only = node_with_children("a", vec![node("b")]);
        let deleted = handle_key(selecting(vec![only.clone()], &[0]), "d");
        let result = handle_key(deleted, "p");
        assert_eq!(result.doc.root, Tree::root(vec![only]));
        assert_eq!(result.selected, Some(vec![0]));
    }

    #[test]
    fn three_p_pastes_three_children_selects_the_last_and_one_u_removes_all() {
        let cut = handle_key(handle_key(story(), "d"), "h");
        let pasted = handle_key(handle_key(cut.clone(), "3"), "p");
        let children = orders_children(&pasted);
        assert_eq!(children.len(), orders_children(&cut).len() + 3);
        assert_eq!(pasted.selected, Some(vec![0, children.len() - 1]));
        assert_eq!(handle_key(pasted, "u").doc, cut.doc);
    }

    #[test]
    fn digits_accumulate_into_the_pending_count() {
        let state = selecting(vec![node("a")], &[0]);
        let state = reduce(state, Action::Digit(1));
        let state = reduce(state, Action::Digit(2));
        assert_eq!(state.pending_count, Some(12));
    }

    #[test]
    fn cancel_count_clears_the_pending_count() {
        let state = reduce(story(), Action::Digit(4));
        assert_eq!(reduce(state, Action::CancelCount).pending_count, None);
    }

    #[test]
    fn interrupt_stops_running_and_drops_the_save_path() {
        let mut state = new_state(vec![node("a")], Mode::Command, None);
        state.save_to = Some("a.dre".to_string());
        let result = reduce(state, Action::Interrupt);
        assert!(!result.running);
        assert_eq!(result.save_to, None);
    }
}
