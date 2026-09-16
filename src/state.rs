pub(crate) const PLAIN: i64 = -1;
const PALETTE_SIZE: i64 = 5;
const PAD: &str = " ";

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Node {
    pub(crate) label: String,
    pub(crate) colour: i64,
    pub(crate) fill: i64,
    pub(crate) rounded: bool,
    pub(crate) children: Vec<Node>,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            label: String::new(),
            colour: PLAIN,
            fill: PLAIN,
            rounded: false,
            children: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
enum Mode {
    #[default]
    Command,
    Insert,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct Document {
    pub(crate) boxes: Vec<Node>,
    pub(crate) selected: Vec<usize>,
}

#[derive(Clone)]
pub(crate) struct State {
    pub(crate) doc: Document,
    history: Vec<Document>,
    mode: Mode,
    pub(crate) running: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            doc: Document::default(),
            history: Vec::new(),
            mode: Mode::default(),
            running: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Command {
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
    CycleFill,
    ToggleRounded,
    Quit,
}

impl TryFrom<&str> for Command {
    type Error = ();

    fn try_from(key: &str) -> Result<Self, Self::Error> {
        Ok(match key {
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
            "f" => Command::CycleFill,
            "r" => Command::ToggleRounded,
            "q" => Command::Quit,
            _ => return Err(()),
        })
    }
}

fn is_undoable(command: Command) -> bool {
    matches!(
        command,
        Command::NewBox
            | Command::CycleColour
            | Command::CycleFill
            | Command::ToggleRounded
            | Command::CycleSiblingsColour
            | Command::RenameLabel
    )
}

fn snapshot(mut state: State) -> State {
    state.history.push(state.doc.clone());
    state
}

fn undo(mut state: State) -> State {
    if let Some(previous) = state.history.pop() {
        state.doc = previous;
    }
    state
}

fn next_colour(colour: i64) -> i64 {
    (colour + 2).rem_euclid(PALETTE_SIZE + 1) - 1
}

fn at(boxes: &[Node], path: &[usize]) -> Node {
    let mut node = boxes[path[0]].clone();
    for &index in &path[1..] {
        node = node.children[index].clone();
    }
    node
}

fn rewrite(boxes: &[Node], path: &[usize], f: &dyn Fn(&Node) -> Node) -> Vec<Node> {
    let index = path[0];
    let rest = &path[1..];
    let mut result = boxes.to_vec();
    let node = &boxes[index];
    let new_node = if !rest.is_empty() {
        let mut n = node.clone();
        n.children = rewrite(&node.children, rest, f);
        n
    } else {
        f(node)
    };
    result[index] = new_node;
    result
}

fn colour_row(boxes: &[Node], path: &[usize]) -> Vec<Node> {
    let parent = &path[..path.len() - 1];
    let siblings: Vec<Node> = if !parent.is_empty() {
        at(boxes, parent).children
    } else {
        boxes.to_vec()
    };
    let first_colour = siblings[0].colour;
    let uniform = siblings.iter().all(|b| b.colour == first_colour);
    let new_colour = if uniform { next_colour(first_colour) } else { 0 };
    let mut boxes = boxes.to_vec();
    for i in 0..siblings.len() {
        let mut sibling_path = parent.to_vec();
        sibling_path.push(i);
        boxes = rewrite(&boxes, &sibling_path, &|node: &Node| {
            let mut n = node.clone();
            n.colour = new_colour;
            n
        });
    }
    boxes
}

fn grow(boxes: &[Node], path: &[usize]) -> (Vec<Node>, Vec<usize>) {
    if path.is_empty() {
        let mut boxes = boxes.to_vec();
        let new_index = boxes.len();
        boxes.push(Node { label: PAD.to_string(), ..Default::default() });
        (boxes, vec![new_index])
    } else {
        let new_index = at(boxes, path).children.len();
        let grown = rewrite(boxes, path, &|node: &Node| {
            let mut n = node.clone();
            n.children.push(Node { label: PAD.to_string(), ..Default::default() });
            n
        });
        let mut new_path = path.to_vec();
        new_path.push(new_index);
        (grown, new_path)
    }
}

fn drop_last_chars(s: &str, n: usize) -> String {
    let len = s.chars().count();
    s.chars().take(len.saturating_sub(n)).collect()
}

fn enter_insert(state: &State, base_label: &str) -> State {
    let label = format!("{base_label}{PAD}");
    let boxes = rewrite(&state.doc.boxes, &state.doc.selected, &|node: &Node| {
        let mut n = node.clone();
        n.label = label.clone();
        n
    });
    let mut new_state = state.clone();
    new_state.doc.boxes = boxes;
    new_state.mode = Mode::Insert;
    new_state
}

fn handle_command(state: &State, key: &str) -> State {
    let command = match Command::try_from(key) {
        Ok(command) => command,
        Err(()) => return state.clone(),
    };
    let mut state = state.clone();
    if is_undoable(command) {
        state = snapshot(state);
    }
    match command {
        Command::Undo => undo(state),
        Command::NewBox => {
            let (boxes, selected) = grow(&state.doc.boxes, &state.doc.selected);
            state.doc.boxes = boxes;
            state.mode = Mode::Insert;
            state.doc.selected = selected;
            state
        }
        Command::NewSibling => {
            if state.doc.selected.is_empty() {
                return state;
            }
            let parent = &state.doc.selected[..state.doc.selected.len() - 1];
            let (boxes, selected) = grow(&state.doc.boxes, parent);
            state.doc.boxes = boxes;
            state.mode = Mode::Insert;
            state.doc.selected = selected;
            state
        }
        Command::SelectParent => {
            if state.doc.selected.len() <= 1 {
                return state;
            }
            state.doc.selected = state.doc.selected[..state.doc.selected.len() - 1].to_vec();
            state
        }
        Command::SelectChild => {
            if state.doc.selected.is_empty() {
                return state;
            }
            if at(&state.doc.boxes, &state.doc.selected).children.is_empty() {
                return state;
            }
            let mut selected = state.doc.selected.clone();
            selected.push(0);
            state.doc.selected = selected;
            state
        }
        Command::SelectNext => {
            if state.doc.selected.is_empty() {
                return state;
            }
            let parent = &state.doc.selected[..state.doc.selected.len() - 1];
            let index = *state.doc.selected.last().unwrap();
            let siblings = if !parent.is_empty() {
                at(&state.doc.boxes, parent).children
            } else {
                state.doc.boxes.clone()
            };
            if index + 1 >= siblings.len() {
                return state;
            }
            let mut selected = parent.to_vec();
            selected.push(index + 1);
            state.doc.selected = selected;
            state
        }
        Command::SelectPrevious => {
            if state.doc.selected.is_empty() {
                return state;
            }
            let parent = &state.doc.selected[..state.doc.selected.len() - 1];
            let index = *state.doc.selected.last().unwrap();
            if index == 0 {
                return state;
            }
            let mut selected = parent.to_vec();
            selected.push(index - 1);
            state.doc.selected = selected;
            state
        }
        Command::EditLabel => {
            if state.doc.selected.is_empty() {
                return state;
            }
            let label = at(&state.doc.boxes, &state.doc.selected).label;
            enter_insert(&state, &label)
        }
        Command::RenameLabel => {
            if state.doc.selected.is_empty() {
                return state;
            }
            enter_insert(&state, "")
        }
        Command::CycleColour => {
            if state.doc.selected.is_empty() {
                return state;
            }
            state.doc.boxes = rewrite(&state.doc.boxes, &state.doc.selected, &|node: &Node| {
                let mut n = node.clone();
                n.colour = next_colour(n.colour);
                n
            });
            state
        }
        Command::CycleSiblingsColour => {
            if state.doc.selected.len() <= 1 {
                return state;
            }
            state.doc.boxes = colour_row(&state.doc.boxes, &state.doc.selected);
            state
        }
        Command::CycleFill => {
            if state.doc.selected.is_empty() {
                return state;
            }
            state.doc.boxes = rewrite(&state.doc.boxes, &state.doc.selected, &|node: &Node| {
                let mut n = node.clone();
                n.fill = next_colour(n.fill);
                n
            });
            state
        }
        Command::ToggleRounded => {
            if state.doc.selected.is_empty() {
                return state;
            }
            state.doc.boxes = rewrite(&state.doc.boxes, &state.doc.selected, &|node: &Node| {
                let mut n = node.clone();
                n.rounded = !n.rounded;
                n
            });
            state
        }
        Command::Quit => {
            state.running = false;
            state
        }
    }
}

fn handle_insert(state: &State, key: &str) -> State {
    let label = at(&state.doc.boxes, &state.doc.selected).label;
    let mut state = state.clone();
    if key == "\x1b" {
        let trimmed = drop_last_chars(&label, 1);
        state.doc.boxes = rewrite(&state.doc.boxes, &state.doc.selected, &|node: &Node| {
            let mut n = node.clone();
            n.label = trimmed.clone();
            n
        });
        state.mode = Mode::Command;
        return state;
    }
    if key == "\x7f" {
        let new_label = format!("{}{PAD}", drop_last_chars(&label, 2));
        state.doc.boxes = rewrite(&state.doc.boxes, &state.doc.selected, &|node: &Node| {
            let mut n = node.clone();
            n.label = new_label.clone();
            n
        });
        return state;
    }
    if key >= "\x20" && key <= "\x7e" {
        let new_label = format!("{}{key}{PAD}", drop_last_chars(&label, 1));
        state.doc.boxes = rewrite(&state.doc.boxes, &state.doc.selected, &|node: &Node| {
            let mut n = node.clone();
            n.label = new_label.clone();
            n
        });
        return state;
    }
    state
}

pub(crate) fn handle_key(state: &State, key: &str) -> State {
    if state.mode == Mode::Insert {
        handle_insert(state, key)
    } else {
        handle_command(state, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(label: &str) -> Node {
        Node { label: label.to_string(), ..Default::default() }
    }

    fn node_with_children(label: &str, children: Vec<Node>) -> Node {
        Node { label: label.to_string(), children, ..Default::default() }
    }

    #[test]
    fn boxes_are_equal() {
        assert_eq!(Node::default(), Node::default());
    }

    #[test]
    fn boxes_default_to_the_plain_colour() {
        assert_eq!(Node::default().colour, PLAIN);
    }

    #[test]
    fn boxes_default_to_the_plain_fill() {
        assert_eq!(Node::default().fill, PLAIN);
    }

    #[test]
    fn boxes_default_to_an_empty_label() {
        assert_eq!(Node::default(), node(""));
    }

    #[test]
    fn boxes_with_different_labels_are_not_equal() {
        assert_ne!(node("a"), node("b"));
    }

    #[test]
    fn boxes_default_to_no_children() {
        assert_eq!(Node::default().children, Vec::<Node>::new());
    }

    #[test]
    fn boxes_with_different_children_are_not_equal() {
        assert_ne!(
            node_with_children("", vec![node("a")]),
            node_with_children("", vec![node("b")])
        );
    }

    #[test]
    fn new_box_starts_with_square_corners() {
        assert_eq!(node("a").rounded, false);
    }

    #[test]
    fn at_a_single_index_returns_the_top_level_box() {
        let boxes = vec![node("a"), node("b")];
        assert_eq!(at(&boxes, &[1]), node("b"));
    }

    #[test]
    fn at_a_longer_path_walks_into_children() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        assert_eq!(at(&boxes, &[0, 1]), node("d"));
    }

    #[test]
    fn at_a_deep_path_walks_multiple_levels() {
        let boxes = vec![node_with_children(
            "a",
            vec![node_with_children("b", vec![node("c")])],
        )];
        assert_eq!(at(&boxes, &[0, 0, 0]), node("c"));
    }

    #[test]
    fn rewrite_replaces_the_top_level_box() {
        let boxes = vec![node("a"), node("b")];
        let result = rewrite(&boxes, &[1], &|_| node("z"));
        assert_eq!(result, vec![node("a"), node("z")]);
    }

    #[test]
    fn rewrite_replaces_a_nested_box_and_rebuilds_the_spine() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let result = rewrite(&boxes, &[0, 1], &|_| node("z"));
        assert_eq!(
            result,
            vec![node_with_children("a", vec![node("c"), node("z")])]
        );
    }

    #[test]
    fn rewrite_does_not_mutate_the_given_boxes() {
        let boxes = vec![node("a")];
        rewrite(&boxes, &[0], &|_| node("z"));
        assert_eq!(boxes, vec![node("a")]);
    }

    #[test]
    fn grow_on_the_canvas_appends_a_top_level_box() {
        let (boxes, path) = grow(&[], &[]);
        assert_eq!(boxes, vec![node(PAD)]);
        assert_eq!(path, vec![0]);
    }

    #[test]
    fn grow_on_the_canvas_appends_after_existing_boxes() {
        let (boxes, path) = grow(&[node("a")], &[]);
        assert_eq!(boxes, vec![node("a"), node(PAD)]);
        assert_eq!(path, vec![1]);
    }

    #[test]
    fn grow_on_a_box_appends_a_child() {
        let (boxes, path) = grow(&[node("a")], &[0]);
        assert_eq!(boxes, vec![node_with_children("a", vec![node(PAD)])]);
        assert_eq!(path, vec![0, 0]);
    }

    #[test]
    fn grow_on_a_box_with_a_child_appends_a_second_child() {
        let boxes = vec![node_with_children("a", vec![node("c")])];
        let (boxes, path) = grow(&boxes, &[0]);
        assert_eq!(
            boxes,
            vec![node_with_children("a", vec![node("c"), node(PAD)])]
        );
        assert_eq!(path, vec![0, 1]);
    }

    #[test]
    fn grow_does_not_mutate_the_given_boxes() {
        let boxes = vec![node("a")];
        grow(&boxes, &[]);
        assert_eq!(boxes, vec![node("a")]);
    }

    #[test]
    fn next_colour_cycles_through_the_palette_and_back_to_plain() {
        let mut colour = PLAIN;
        for _ in 0..PALETTE_SIZE {
            colour = next_colour(colour);
        }
        assert_ne!(colour, PLAIN);
        colour = next_colour(colour);
        assert_eq!(colour, PLAIN);
    }

    #[test]
    fn colour_row_advances_uniformly_coloured_siblings() {
        let boxes = vec![node("a"), node("b")];
        let mut a = node("a");
        a.colour = next_colour(PLAIN);
        let mut b = node("b");
        b.colour = next_colour(PLAIN);
        assert_eq!(colour_row(&boxes, &[0]), vec![a, b]);
    }

    #[test]
    fn colour_row_sets_mixed_siblings_to_the_first_palette_colour() {
        let mut a = node("a");
        a.colour = 0;
        let mut b = node("b");
        b.colour = 1;
        let boxes = vec![a, b];
        let mut expected_a = node("a");
        expected_a.colour = 0;
        let mut expected_b = node("b");
        expected_b.colour = 0;
        assert_eq!(colour_row(&boxes, &[0]), vec![expected_a, expected_b]);
    }

    #[test]
    fn colour_row_does_not_mutate_the_given_boxes() {
        let boxes = vec![node("a"), node("b")];
        colour_row(&boxes, &[0]);
        assert_eq!(boxes, vec![node("a"), node("b")]);
    }

    fn new_state(boxes: Vec<Node>, mode: Mode, selected: Vec<usize>) -> State {
        State {
            doc: Document { boxes, selected },
            history: Vec::new(),
            mode,
            running: true,
        }
    }

    #[test]
    fn state_starts_running() {
        let state = new_state(vec![], Mode::Command, vec![]);
        assert!(state.running);
    }

    #[test]
    fn state_starts_in_command_mode() {
        let state = new_state(vec![], Mode::Command, vec![]);
        assert_eq!(state.mode, Mode::Command);
    }

    #[test]
    fn b_on_an_empty_canvas_appends_a_box_enters_insert_and_selects_it() {
        let state = new_state(vec![], Mode::Command, vec![]);
        let result: State = handle_key(&state, "b");
        assert_eq!(result.doc.boxes, vec![node(PAD)]);
        assert_eq!(result.mode, Mode::Insert);
        assert_eq!(result.doc.selected, vec![0]);
        assert!(result.running);
    }

    #[test]
    fn b_on_an_empty_canvas_does_not_mutate_the_given_state() {
        let state = new_state(vec![], Mode::Command, vec![]);
        handle_key(&state, "b");
        assert_eq!(state.doc.boxes, Vec::<Node>::new());
    }

    #[test]
    fn b_on_a_selected_box_appends_and_selects_a_child() {
        let state = new_state(vec![node("a")], Mode::Command, vec![0]);
        let result = handle_key(&state, "b");
        assert_eq!(result.doc.boxes, vec![node_with_children("a", vec![node(PAD)])]);
        assert_eq!(result.doc.selected, vec![0, 0]);
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn a_second_b_on_the_same_parent_places_a_second_child() {
        let state = new_state(vec![node("a")], Mode::Command, vec![0]);
        let state = handle_key(&state, "b");
        let state = handle_key(&state, "\x1b");
        let state = handle_key(&state, "h");
        let result = handle_key(&state, "b");
        assert_eq!(
            result.doc.boxes,
            vec![node_with_children("a", vec![node(""), node(PAD)])]
        );
        assert_eq!(result.doc.selected, vec![0, 1]);
    }

    #[test]
    fn unknown_key_returns_the_state_unchanged() {
        let state = new_state(vec![node("a")], Mode::Command, vec![]);
        let result = handle_key(&state, "x");
        assert_eq!(result.doc.boxes, state.doc.boxes);
        assert_eq!(result.doc.selected, state.doc.selected);
        assert_eq!(result.mode, state.mode);
        assert_eq!(result.running, state.running);
    }

    #[test]
    fn q_stops_the_state_and_preserves_boxes_and_selection() {
        let state = new_state(vec![node("a")], Mode::Command, vec![0]);
        let result = handle_key(&state, "q");
        assert!(!result.running);
        assert_eq!(result.doc.boxes, vec![node("a")]);
        assert_eq!(result.doc.selected, vec![0]);
        assert_eq!(result.mode, Mode::Command);
    }

    #[test]
    fn insert_mode_is_dispatched_separately() {
        let state = new_state(vec![node(PAD)], Mode::Insert, vec![0]);
        let result = handle_key(&state, "q");
        assert!(result.running);
    }

    #[test]
    fn s_on_a_top_level_box_appends_a_sibling() {
        let state = new_state(vec![node("a")], Mode::Command, vec![0]);
        let result = handle_key(&state, "s");
        assert_eq!(result.doc.boxes, vec![node("a"), node(PAD)]);
        assert_eq!(result.doc.selected, vec![1]);
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn s_on_a_child_box_appends_a_sibling_to_the_parents_children() {
        let boxes = vec![node_with_children("a", vec![node("b")])];
        let state = new_state(boxes, Mode::Command, vec![0, 0]);
        let result = handle_key(&state, "s");
        assert_eq!(
            result.doc.boxes,
            vec![node_with_children("a", vec![node("b"), node(PAD)])]
        );
        assert_eq!(result.doc.selected, vec![0, 1]);
    }

    #[test]
    fn s_with_no_selection_returns_the_state_unchanged() {
        let state = new_state(vec![node("a")], Mode::Command, vec![]);
        let result = handle_key(&state, "s");
        assert_eq!(result.doc.boxes, vec![node("a")]);
        assert_eq!(result.doc.selected, Vec::<usize>::new());
    }

    #[test]
    fn h_selects_the_parent_and_is_a_no_op_at_the_top() {
        let boxes = vec![node_with_children("a", vec![node("c")])];
        let state = new_state(boxes, Mode::Command, vec![0, 0]);
        let result = handle_key(&state, "h");
        assert_eq!(result.doc.selected, vec![0]);

        let result = handle_key(&result, "h");
        assert_eq!(result.doc.selected, vec![0]);
    }

    #[test]
    fn h_on_an_empty_canvas_returns_the_state_unchanged() {
        let state = new_state(vec![], Mode::Command, vec![]);
        let result = handle_key(&state, "h");
        assert_eq!(result.doc.selected, Vec::<usize>::new());
    }

    #[test]
    fn h_in_insert_mode_types_the_letter_h() {
        let state = new_state(vec![node(&format!("a{PAD}"))], Mode::Insert, vec![0]);
        let result = handle_key(&state, "h");
        assert_eq!(result.doc.boxes, vec![node(&format!("ah{PAD}"))]);
    }

    #[test]
    fn l_selects_the_first_child_or_keeps_selection_with_none() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let state = new_state(boxes, Mode::Command, vec![0]);
        let result = handle_key(&state, "l");
        assert_eq!(result.doc.selected, vec![0, 0]);

        let state = new_state(vec![node("a")], Mode::Command, vec![0]);
        let result = handle_key(&state, "l");
        assert_eq!(result.doc.selected, vec![0]);
    }

    #[test]
    fn j_and_k_move_between_siblings_with_bounds() {
        let state = new_state(vec![node("a"), node("b")], Mode::Command, vec![0]);
        let result = handle_key(&state, "j");
        assert_eq!(result.doc.selected, vec![1]);

        let state = new_state(vec![node("a"), node("b")], Mode::Command, vec![1]);
        let result = handle_key(&state, "j");
        assert_eq!(result.doc.selected, vec![1]);

        let state = new_state(vec![node("a"), node("b")], Mode::Command, vec![1]);
        let result = handle_key(&state, "k");
        assert_eq!(result.doc.selected, vec![0]);

        let state = new_state(vec![node("a"), node("b")], Mode::Command, vec![0]);
        let result = handle_key(&state, "k");
        assert_eq!(result.doc.selected, vec![0]);
    }

    #[test]
    fn i_enters_insert_and_appends_pad_to_the_selected_boxs_label() {
        let state = new_state(vec![node("a"), node("b")], Mode::Command, vec![1]);
        let result = handle_key(&state, "i");
        assert_eq!(result.mode, Mode::Insert);
        assert_eq!(result.doc.selected, vec![1]);
        assert_eq!(result.doc.boxes, vec![node("a"), node(&format!("b{PAD}"))]);
    }

    #[test]
    fn i_on_an_empty_canvas_returns_the_state_unchanged() {
        let state = new_state(vec![], Mode::Command, vec![]);
        let result = handle_key(&state, "i");
        assert_eq!(result.mode, Mode::Command);
        assert_eq!(result.doc.boxes, Vec::<Node>::new());
    }

    #[test]
    fn capital_i_clears_the_selected_boxs_label() {
        let state = new_state(vec![node("a"), node("b")], Mode::Command, vec![1]);
        let result = handle_key(&state, "I");
        assert_eq!(result.mode, Mode::Insert);
        assert_eq!(result.doc.boxes, vec![node("a"), node(PAD)]);
    }

    #[test]
    fn typing_appends_to_the_selected_box_label() {
        let state = new_state(vec![node(&format!("h{PAD}"))], Mode::Insert, vec![0]);
        let result = handle_key(&state, "i");
        assert_eq!(result.doc.boxes, vec![node(&format!("hi{PAD}"))]);
    }

    #[test]
    fn space_and_tilde_are_printable() {
        let state = new_state(vec![node(&format!("a{PAD}"))], Mode::Insert, vec![0]);
        let result = handle_key(&state, " ");
        assert_eq!(result.doc.boxes, vec![node(&format!("a {PAD}"))]);

        let state = new_state(vec![node(PAD)], Mode::Insert, vec![0]);
        let result = handle_key(&state, "~");
        assert_eq!(result.doc.boxes, vec![node(&format!("~{PAD}"))]);
    }

    #[test]
    fn backspace_drops_the_last_character_and_is_a_no_op_when_empty() {
        let state = new_state(vec![node(&format!("hi{PAD}"))], Mode::Insert, vec![0]);
        let result = handle_key(&state, "\x7f");
        assert_eq!(result.doc.boxes, vec![node(&format!("h{PAD}"))]);

        let state = new_state(vec![node(PAD)], Mode::Insert, vec![0]);
        let result = handle_key(&state, "\x7f");
        assert_eq!(result.doc.boxes, vec![node(PAD)]);
    }

    #[test]
    fn esc_returns_to_command_mode_and_trims_pad() {
        let state = new_state(vec![node(&format!("hi{PAD}"))], Mode::Insert, vec![0]);
        let result = handle_key(&state, "\x1b");
        assert_eq!(result.mode, Mode::Command);
        assert_eq!(result.doc.boxes, vec![node("hi")]);
    }

    #[test]
    fn control_and_non_ascii_characters_return_the_state_unchanged() {
        let state = new_state(vec![node(&format!("hi{PAD}"))], Mode::Insert, vec![0]);
        let result = handle_key(&state, "\x01");
        assert_eq!(result.doc.boxes, vec![node(&format!("hi{PAD}"))]);

        let result = handle_key(&state, "é");
        assert_eq!(result.doc.boxes, vec![node(&format!("hi{PAD}"))]);
    }

    #[test]
    fn cycle_colour_and_fill_advance_independently_and_cycle_back_to_plain() {
        let state = new_state(vec![node("a")], Mode::Command, vec![0]);
        let result = handle_key(&state, "c");
        let mut expected = node("a");
        expected.colour = next_colour(PLAIN);
        assert_eq!(result.doc.boxes, vec![expected]);
        assert_eq!(result.doc.boxes[0].label, "a");

        let state = new_state(vec![node("a")], Mode::Command, vec![0]);
        let result = handle_key(&state, "f");
        let mut expected = node("a");
        expected.fill = next_colour(PLAIN);
        assert_eq!(result.doc.boxes, vec![expected]);

        let mut state_boxed = vec![node("a")];
        let mut colour = PLAIN;
        for _ in 0..=PALETTE_SIZE {
            let s = new_state(state_boxed.clone(), Mode::Command, vec![0]);
            let result = handle_key(&s, "c");
            state_boxed = result.doc.boxes;
            colour = state_boxed[0].colour;
        }
        assert_eq!(colour, PLAIN);
    }

    #[test]
    fn toggle_rounded_flips_and_flips_back() {
        let state = new_state(vec![node("a")], Mode::Command, vec![0]);
        let result = handle_key(&state, "r");
        assert_eq!(result.doc.boxes[0].rounded, true);
        let state = new_state(result.doc.boxes.clone(), Mode::Command, vec![0]);
        let result = handle_key(&state, "r");
        assert_eq!(result.doc.boxes[0].rounded, false);
    }

    #[test]
    fn capital_c_on_a_top_level_box_does_nothing() {
        let state = new_state(vec![node("a"), node("b")], Mode::Command, vec![0]);
        let result = handle_key(&state, "C");
        assert_eq!(result.doc.boxes, state.doc.boxes);
        assert_eq!(result.doc.selected, state.doc.selected);
    }

    #[test]
    fn capital_c_advances_uniformly_coloured_siblings() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let state = new_state(boxes, Mode::Command, vec![0, 1]);
        let result = handle_key(&state, "C");
        let mut c = node("c");
        c.colour = next_colour(PLAIN);
        let mut d = node("d");
        d.colour = next_colour(PLAIN);
        assert_eq!(result.doc.boxes, vec![node_with_children("a", vec![c, d])]);
    }

    #[test]
    fn u_after_b_restores_boxes_and_selected() {
        let before = new_state(vec![], Mode::Command, vec![]);
        let after = handle_key(&before, "b");
        let after_escape = handle_key(&after, "\x1b");
        let undone = handle_key(&after_escape, "u");
        assert_eq!(undone.doc.boxes, before.doc.boxes);
        assert_eq!(undone.doc.selected, before.doc.selected);
        assert_eq!(undone.mode, before.mode);
    }

    #[test]
    fn u_after_c_restores_boxes() {
        let before = new_state(vec![node("a")], Mode::Command, vec![0]);
        let after = handle_key(&before, "c");
        let undone = handle_key(&after, "u");
        assert_eq!(undone.doc.boxes, before.doc.boxes);
    }

    #[test]
    fn u_after_c_with_nothing_selected_is_a_no_op() {
        let state = new_state(vec![node("a")], Mode::Command, vec![]);
        let after = handle_key(&state, "c");
        let undone = handle_key(&after, "u");
        assert_eq!(undone.doc.boxes, state.doc.boxes);
        assert_eq!(undone.doc.selected, state.doc.selected);
    }

    #[test]
    fn u_with_no_previous_action_leaves_state_unchanged() {
        let state = new_state(vec![], Mode::Command, vec![]);
        let result = handle_key(&state, "u");
        assert_eq!(result.doc.boxes, Vec::<Node>::new());
        assert_eq!(result.doc.selected, Vec::<usize>::new());
    }

    #[test]
    fn u_twice_in_a_row_does_not_redo() {
        let state = new_state(vec![node("a")], Mode::Command, vec![0]);
        let after_command = handle_key(&state, "c");
        let after_first_undo = handle_key(&after_command, "u");
        let after_second_undo = handle_key(&after_first_undo, "u");
        assert_eq!(after_second_undo.doc.boxes, after_first_undo.doc.boxes);
        assert_eq!(after_second_undo.doc.selected, after_first_undo.doc.selected);
    }

    #[test]
    fn movement_keys_do_not_clobber_an_existing_undo_snapshot() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let before = new_state(boxes, Mode::Command, vec![0, 0]);
        let after_command = handle_key(&before, "c");
        let navigated = handle_key(&after_command, "j");
        let navigated = handle_key(&navigated, "h");
        let navigated = handle_key(&navigated, "l");
        let navigated = handle_key(&navigated, "k");
        let undone = handle_key(&navigated, "u");
        assert_eq!(undone.doc.boxes, before.doc.boxes);
        assert_eq!(undone.doc.selected, before.doc.selected);
    }

    #[test]
    fn q_does_not_clobber_an_existing_undo_snapshot() {
        let before = new_state(vec![node("a")], Mode::Command, vec![0]);
        let after_command = handle_key(&before, "c");
        let after_quit = handle_key(&after_command, "q");
        let undone = handle_key(&after_quit, "u");
        assert_eq!(undone.doc.boxes, before.doc.boxes);
        assert_eq!(undone.doc.selected, before.doc.selected);
    }

    #[test]
    fn u_after_an_insert_session_undoes_the_b_that_started_it() {
        let before = new_state(vec![], Mode::Command, vec![]);
        let after_b = handle_key(&before, "b");
        let after_typing = handle_key(&after_b, "h");
        let after_typing = handle_key(&after_typing, "i");
        let after_escape = handle_key(&after_typing, "\x1b");
        let undone = handle_key(&after_escape, "u");
        assert_eq!(undone.doc.boxes, before.doc.boxes);
        assert_eq!(undone.doc.selected, before.doc.selected);
    }

    #[test]
    fn repeated_u_walks_back_through_every_undoable_command() {
        let start = new_state(vec![node("a")], Mode::Command, vec![0]);
        let after_colour = handle_key(&start, "c");
        let after_fill = handle_key(&after_colour, "f");
        let after_rounded = handle_key(&after_fill, "r");
        let undone = handle_key(&after_rounded, "u");
        assert_eq!(undone.doc.boxes, after_fill.doc.boxes);
        let undone = handle_key(&undone, "u");
        assert_eq!(undone.doc.boxes, after_colour.doc.boxes);
        let undone = handle_key(&undone, "u");
        assert_eq!(undone.doc.boxes, start.doc.boxes);
        assert_eq!(undone.doc.selected, start.doc.selected);
    }

    #[test]
    fn u_on_an_exhausted_history_leaves_the_state_unchanged() {
        let start = new_state(vec![node("a")], Mode::Command, vec![0]);
        let after_colour = handle_key(&start, "c");
        let after_fill = handle_key(&after_colour, "f");
        let undone = handle_key(&after_fill, "u");
        let undone = handle_key(&undone, "u");
        let exhausted = handle_key(&undone, "u");
        assert_eq!(exhausted.doc.boxes, start.doc.boxes);
        assert_eq!(exhausted.doc.selected, start.doc.selected);
        assert_eq!(exhausted.mode, start.mode);
        assert_eq!(exhausted.running, start.running);
    }

    #[test]
    fn u_after_capital_i_restores_the_boxs_previous_label() {
        let before = new_state(vec![node("a")], Mode::Command, vec![0]);
        let after = handle_key(&before, "I");
        let after_escape = handle_key(&after, "\x1b");
        let undone = handle_key(&after_escape, "u");
        assert_eq!(undone.doc.boxes, before.doc.boxes);
    }
}
