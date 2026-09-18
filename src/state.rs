use crate::command_mode;
use crate::insert_mode;
use crate::save_prompt_mode;

pub(crate) const PALETTE_SIZE: u8 = 5;
pub(crate) const PAD: &str = " ";
pub(crate) const DEFAULT_FILENAME: &str = "diagram.dre";

#[allow(dead_code)]
pub(crate) struct KeyBinding<C> {
    pub(crate) keys: &'static [&'static str],
    pub(crate) command: C,
    pub(crate) description: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Node {
    pub(crate) label: String,
    pub(crate) colour: Option<u8>,
    pub(crate) filled: bool,
    pub(crate) rounded: bool,
    pub(crate) children: Vec<Node>,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            label: String::new(),
            colour: None,
            filled: false,
            rounded: false,
            children: Vec::new(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub(crate) enum Mode {
    #[default]
    Command,
    Insert,
    SavePrompt { filename: String },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Path {
    pub(crate) ancestors: Vec<usize>,
    pub(crate) index: usize,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct Document {
    pub(crate) boxes: Vec<Node>,
    pub(crate) selected: Option<Path>,
}

#[derive(Clone)]
pub(crate) struct State {
    pub(crate) doc: Document,
    history: Vec<Document>,
    pub(crate) mode: Mode,
    pub(crate) running: bool,
    pub(crate) save_to: Option<String>,
    pub(crate) new_file: bool,
    pub(crate) pending_count: Option<usize>,
}

impl Default for State {
    fn default() -> Self {
        State {
            doc: Document::default(),
            history: Vec::new(),
            mode: Mode::default(),
            running: true,
            save_to: None,
            new_file: false,
            pending_count: None,
        }
    }
}

pub(crate) fn snapshot(mut state: State) -> State {
    state.history.push(state.doc.clone());
    state
}

pub(crate) fn undo(mut state: State) -> State {
    if let Some(previous) = state.history.pop() {
        state.doc = previous;
    }
    state
}

pub(crate) fn next_colour(colour: Option<u8>) -> Option<u8> {
    match colour {
        None => Some(0),
        Some(i) if i + 1 < PALETTE_SIZE => Some(i + 1),
        Some(_) => None,
    }
}

pub(crate) fn children_at<'a>(boxes: &'a mut Vec<Node>, ancestors: &[usize]) -> &'a mut Vec<Node> {
    let mut children = boxes;
    for &index in ancestors {
        children = &mut children[index].children;
    }
    children
}

pub(crate) fn at<'a>(boxes: &'a mut Vec<Node>, path: &Path) -> &'a mut Node {
    &mut children_at(boxes, &path.ancestors)[path.index]
}

pub(crate) fn colour_row(boxes: &mut Vec<Node>, path: &Path) {
    let siblings = children_at(boxes, &path.ancestors);
    let first_colour = siblings[0].colour;
    let uniform = siblings.iter().all(|b| b.colour == first_colour);
    let new_colour = if uniform { next_colour(first_colour) } else { Some(0) };
    for sibling in siblings.iter_mut() {
        sibling.colour = new_colour;
    }
}

pub(crate) fn grow(siblings: &mut Vec<Node>) -> usize {
    siblings.push(Node { label: PAD.to_string(), ..Default::default() });
    siblings.len() - 1
}

pub(crate) fn add_child_box(mut state: State, selected: Option<Path>) -> State {
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

pub(crate) fn handle_key(mut state: State, key: &str) -> State {
    match &state.mode {
        Mode::Command => {
            if key.len() == 1 && key.as_bytes()[0].is_ascii_digit() {
                let digit = key.as_bytes()[0] - b'0';
                state.pending_count = Some(
                    state
                        .pending_count
                        .unwrap_or(0)
                        .saturating_mul(10)
                        .saturating_add(digit as usize),
                );
                state
            } else {
                match command_mode::parse(key) {
                    Some(command) => command_mode::reduce(state, command),
                    None => {
                        state.pending_count = None;
                        state
                    }
                }
            }
        }
        Mode::Insert => match insert_mode::parse(key) {
            Some(command) => insert_mode::reduce(state, command),
            None => state,
        },
        Mode::SavePrompt { .. } => match save_prompt_mode::parse(key) {
            Some(command) => save_prompt_mode::reduce(state, command),
            None => state,
        },
    }
}

#[cfg(test)]
pub(crate) fn new_state(boxes: Vec<Node>, mode: Mode, selected: Option<Path>) -> State {
    State { doc: Document { boxes, selected }, history: Vec::new(), mode, running: true, save_to: None, new_file: false, pending_count: None }
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
        assert_eq!(Node::default().colour, None);
    }

    #[test]
    fn boxes_default_to_the_plain_fill() {
        assert_eq!(Node::default().filled, false);
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
    fn children_at_no_ancestors_returns_the_top_level_boxes() {
        let mut boxes = vec![node("a"), node("b")];
        assert_eq!(*children_at(&mut boxes, &[]), vec![node("a"), node("b")]);
    }

    #[test]
    fn children_at_ancestors_returns_the_addressed_nodes_children() {
        let mut boxes = vec![node_with_children(
            "a",
            vec![node_with_children("b", vec![node("c"), node("d")])],
        )];
        assert_eq!(*children_at(&mut boxes, &[0]), vec![node_with_children("b", vec![node("c"), node("d")])]);
        assert_eq!(*children_at(&mut boxes, &[0, 0]), vec![node("c"), node("d")]);
    }

    #[test]
    fn at_a_single_index_returns_the_top_level_box() {
        let mut boxes = vec![node("a"), node("b")];
        assert_eq!(*at(&mut boxes, &Path { ancestors: vec![], index: 1 }), node("b"));
    }

    #[test]
    fn at_a_longer_path_walks_into_children() {
        let mut boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        assert_eq!(*at(&mut boxes, &Path { ancestors: vec![0], index: 1 }), node("d"));
    }

    #[test]
    fn at_a_deep_path_walks_multiple_levels() {
        let mut boxes = vec![node_with_children(
            "a",
            vec![node_with_children("b", vec![node("c")])],
        )];
        assert_eq!(*at(&mut boxes, &Path { ancestors: vec![0, 0], index: 0 }), node("c"));
    }

    #[test]
    fn grow_on_an_empty_list_appends_a_padded_node() {
        let mut boxes = Vec::new();
        let index = grow(&mut boxes);
        assert_eq!(boxes, vec![node(PAD)]);
        assert_eq!(index, 0);
    }

    #[test]
    fn grow_appends_after_existing_nodes() {
        let mut boxes = vec![node("a")];
        let index = grow(&mut boxes);
        assert_eq!(boxes, vec![node("a"), node(PAD)]);
        assert_eq!(index, 1);
    }

    #[test]
    fn grow_on_a_nodes_children_appends_a_child() {
        let mut boxes = vec![node("a")];
        let index = grow(&mut boxes[0].children);
        assert_eq!(boxes, vec![node_with_children("a", vec![node(PAD)])]);
        assert_eq!(index, 0);
    }

    #[test]
    fn grow_on_a_nodes_children_appends_a_second_child() {
        let mut boxes = vec![node_with_children("a", vec![node("c")])];
        let index = grow(&mut boxes[0].children);
        assert_eq!(
            boxes,
            vec![node_with_children("a", vec![node("c"), node(PAD)])]
        );
        assert_eq!(index, 1);
    }

    #[test]
    fn add_child_box_without_a_selection_grows_a_top_level_box_and_enters_insert_mode() {
        let state = new_state(vec![], Mode::Command, None);
        let result = add_child_box(state, None);
        assert_eq!(result.doc.boxes, vec![node(PAD)]);
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 0 }));
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn add_child_box_with_a_selection_grows_a_child_and_descends_the_path() {
        let state = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let result = add_child_box(state, Some(Path { ancestors: vec![], index: 0 }));
        assert_eq!(result.doc.boxes, vec![node_with_children("a", vec![node(PAD)])]);
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![0], index: 0 }));
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn next_colour_cycles_through_the_palette_and_back_to_plain() {
        let mut colour = None;
        for _ in 0..PALETTE_SIZE {
            colour = next_colour(colour);
        }
        assert_ne!(colour, None);
        colour = next_colour(colour);
        assert_eq!(colour, None);
    }

    #[test]
    fn colour_row_advances_uniformly_coloured_siblings() {
        let mut boxes = vec![node("a"), node("b")];
        let mut a = node("a");
        a.colour = next_colour(None);
        let mut b = node("b");
        b.colour = next_colour(None);
        colour_row(&mut boxes, &Path { ancestors: vec![], index: 0 });
        assert_eq!(boxes, vec![a, b]);
    }

    #[test]
    fn colour_row_sets_mixed_siblings_to_the_first_palette_colour() {
        let mut a = node("a");
        a.colour = Some(0);
        let mut b = node("b");
        b.colour = Some(1);
        let mut boxes = vec![a, b];
        let mut expected_a = node("a");
        expected_a.colour = Some(0);
        let mut expected_b = node("b");
        expected_b.colour = Some(0);
        colour_row(&mut boxes, &Path { ancestors: vec![], index: 0 });
        assert_eq!(boxes, vec![expected_a, expected_b]);
    }

    #[test]
    fn state_starts_running() {
        let state = new_state(vec![], Mode::Command, None);
        assert!(state.running);
    }

    #[test]
    fn state_starts_in_command_mode() {
        let state = new_state(vec![], Mode::Command, None);
        assert_eq!(state.mode, Mode::Command);
    }

    #[test]
    fn unknown_key_returns_the_state_unchanged() {
        let state = new_state(vec![node("a")], Mode::Command, None);
        let result = handle_key(state.clone(), "x");
        assert_eq!(result.doc.boxes, state.doc.boxes);
        assert_eq!(result.doc.selected, state.doc.selected);
        assert_eq!(result.mode, state.mode);
        assert_eq!(result.running, state.running);
    }

    #[test]
    fn digits_accumulate_across_keystrokes() {
        let boxes: Vec<Node> = (0..40).map(|i| node(&i.to_string())).collect();
        let state = new_state(boxes, Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let state = handle_key(state, "3");
        let result = handle_key(state, "2");
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 0 }));
        let result = handle_key(result, "j");
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 32 }));
    }

    #[test]
    fn repeated_digits_saturate_without_panic() {
        let boxes = vec![node("a"), node("b")];
        let state = new_state(boxes, Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        let mut state = state;
        for _ in 0..20 {
            state = handle_key(state, "9");
        }
        let state = handle_key(state, "x");
        let result = handle_key(state, "j");
        assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 1 }));
    }
}
