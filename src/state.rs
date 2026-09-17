use crate::command_mode;
use crate::insert_mode;
use crate::save_prompt_mode;

pub(crate) const PLAIN: i64 = -1;
pub(crate) const PALETTE_SIZE: i64 = 5;
pub(crate) const PAD: &str = " ";
pub(crate) const DEFAULT_FILENAME: &str = "diagram.dre";

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

#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub(crate) enum Mode {
    #[default]
    Command,
    Insert,
    SavePrompt { filename: String },
}

#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct Document {
    pub(crate) boxes: Vec<Node>,
    pub(crate) selected: Vec<usize>,
}

#[derive(Clone)]
pub(crate) struct State {
    pub(crate) doc: Document,
    pub(crate) history: Vec<Document>,
    pub(crate) mode: Mode,
    pub(crate) running: bool,
    pub(crate) save_to: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        State {
            doc: Document::default(),
            history: Vec::new(),
            mode: Mode::default(),
            running: true,
            save_to: None,
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

pub(crate) fn next_colour(colour: i64) -> i64 {
    (colour + 2).rem_euclid(PALETTE_SIZE + 1) - 1
}

pub(crate) fn at<'a>(boxes: &'a mut [Node], path: &[usize]) -> &'a mut Node {
    let mut node = &mut boxes[path[0]];
    for &index in &path[1..] {
        node = &mut node.children[index];
    }
    node
}

pub(crate) fn colour_row(boxes: &mut Vec<Node>, path: &[usize]) {
    let parent = &path[..path.len() - 1];
    let siblings = if parent.is_empty() {
        boxes
    } else {
        &mut at(boxes, parent).children
    };
    let first_colour = siblings[0].colour;
    let uniform = siblings.iter().all(|b| b.colour == first_colour);
    let new_colour = if uniform { next_colour(first_colour) } else { 0 };
    for sibling in siblings.iter_mut() {
        sibling.colour = new_colour;
    }
}

pub(crate) fn grow(boxes: &mut Vec<Node>, path: &[usize]) -> Vec<usize> {
    if path.is_empty() {
        boxes.push(Node { label: PAD.to_string(), ..Default::default() });
        vec![boxes.len() - 1]
    } else {
        let children = &mut at(boxes, path).children;
        children.push(Node { label: PAD.to_string(), ..Default::default() });
        let new_index = children.len() - 1;
        let mut new_path = path.to_vec();
        new_path.push(new_index);
        new_path
    }
}

pub(crate) fn handle_key(state: State, key: &str) -> State {
    match &state.mode {
        Mode::Command => match command_mode::parse(key) {
            Some(command) => command_mode::reduce(state, command),
            None => state,
        },
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
pub(crate) fn new_state(boxes: Vec<Node>, mode: Mode, selected: Vec<usize>) -> State {
    State { doc: Document { boxes, selected }, history: Vec::new(), mode, running: true, save_to: None }
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
        let mut boxes = vec![node("a"), node("b")];
        assert_eq!(*at(&mut boxes, &[1]), node("b"));
    }

    #[test]
    fn at_a_longer_path_walks_into_children() {
        let mut boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        assert_eq!(*at(&mut boxes, &[0, 1]), node("d"));
    }

    #[test]
    fn at_a_deep_path_walks_multiple_levels() {
        let mut boxes = vec![node_with_children(
            "a",
            vec![node_with_children("b", vec![node("c")])],
        )];
        assert_eq!(*at(&mut boxes, &[0, 0, 0]), node("c"));
    }

    #[test]
    fn grow_on_the_canvas_appends_a_top_level_box() {
        let mut boxes = Vec::new();
        let path = grow(&mut boxes, &[]);
        assert_eq!(boxes, vec![node(PAD)]);
        assert_eq!(path, vec![0]);
    }

    #[test]
    fn grow_on_the_canvas_appends_after_existing_boxes() {
        let mut boxes = vec![node("a")];
        let path = grow(&mut boxes, &[]);
        assert_eq!(boxes, vec![node("a"), node(PAD)]);
        assert_eq!(path, vec![1]);
    }

    #[test]
    fn grow_on_a_box_appends_a_child() {
        let mut boxes = vec![node("a")];
        let path = grow(&mut boxes, &[0]);
        assert_eq!(boxes, vec![node_with_children("a", vec![node(PAD)])]);
        assert_eq!(path, vec![0, 0]);
    }

    #[test]
    fn grow_on_a_box_with_a_child_appends_a_second_child() {
        let mut boxes = vec![node_with_children("a", vec![node("c")])];
        let path = grow(&mut boxes, &[0]);
        assert_eq!(
            boxes,
            vec![node_with_children("a", vec![node("c"), node(PAD)])]
        );
        assert_eq!(path, vec![0, 1]);
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
        let mut boxes = vec![node("a"), node("b")];
        let mut a = node("a");
        a.colour = next_colour(PLAIN);
        let mut b = node("b");
        b.colour = next_colour(PLAIN);
        colour_row(&mut boxes, &[0]);
        assert_eq!(boxes, vec![a, b]);
    }

    #[test]
    fn colour_row_sets_mixed_siblings_to_the_first_palette_colour() {
        let mut a = node("a");
        a.colour = 0;
        let mut b = node("b");
        b.colour = 1;
        let mut boxes = vec![a, b];
        let mut expected_a = node("a");
        expected_a.colour = 0;
        let mut expected_b = node("b");
        expected_b.colour = 0;
        colour_row(&mut boxes, &[0]);
        assert_eq!(boxes, vec![expected_a, expected_b]);
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
    fn unknown_key_returns_the_state_unchanged() {
        let state = new_state(vec![node("a")], Mode::Command, vec![]);
        let result = handle_key(state.clone(), "x");
        assert_eq!(result.doc.boxes, state.doc.boxes);
        assert_eq!(result.doc.selected, state.doc.selected);
        assert_eq!(result.mode, state.mode);
        assert_eq!(result.running, state.running);
    }
}
