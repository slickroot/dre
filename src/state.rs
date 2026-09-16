use crate::command_mode;
use crate::insert_mode;

pub(crate) const PALETTE_SIZE: u8 = 5;
pub(crate) const PAD: &str = " ";

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Node {
    pub(crate) label: String,
    pub(crate) colour: Option<u8>,
    pub(crate) fill: Option<u8>,
    pub(crate) rounded: bool,
    pub(crate) children: Vec<Node>,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            label: String::new(),
            colour: None,
            fill: None,
            rounded: false,
            children: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(crate) enum Mode {
    #[default]
    Command,
    Insert,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Path {
    pub(crate) head: usize,
    pub(crate) tail: Vec<usize>,
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

pub(crate) fn at<'a>(boxes: &'a mut [Node], path: &Path) -> &'a mut Node {
    let mut node = &mut boxes[path.head];
    for &index in &path.tail {
        node = &mut node.children[index];
    }
    node
}

pub(crate) fn colour_row(boxes: &mut Vec<Node>, path: &Path) {
    let parent = if path.tail.is_empty() {
        None
    } else {
        let mut tail = path.tail.clone();
        tail.pop();
        Some(Path { head: path.head, tail })
    };
    let siblings = match &parent {
        None => boxes,
        Some(parent) => &mut at(boxes, parent).children,
    };
    let first_colour = siblings[0].colour;
    let uniform = siblings.iter().all(|b| b.colour == first_colour);
    let new_colour = if uniform { next_colour(first_colour) } else { Some(0) };
    for sibling in siblings.iter_mut() {
        sibling.colour = new_colour;
    }
}

pub(crate) fn grow(boxes: &mut Vec<Node>, path: &Option<Path>) -> Path {
    match path {
        None => {
            boxes.push(Node { label: PAD.to_string(), ..Default::default() });
            Path { head: boxes.len() - 1, tail: Vec::new() }
        }
        Some(path) => {
            let children = &mut at(boxes, path).children;
            children.push(Node { label: PAD.to_string(), ..Default::default() });
            let new_index = children.len() - 1;
            let mut tail = path.tail.clone();
            tail.push(new_index);
            Path { head: path.head, tail }
        }
    }
}

pub(crate) fn handle_key(state: State, key: &str) -> State {
    match state.mode {
        Mode::Command => match command_mode::parse(key) {
            Some(command) => command_mode::reduce(state, command),
            None => state,
        },
        Mode::Insert => match insert_mode::parse(key) {
            Some(command) => insert_mode::reduce(state, command),
            None => state,
        },
    }
}

#[cfg(test)]
pub(crate) fn new_state(boxes: Vec<Node>, mode: Mode, selected: Option<Path>) -> State {
    State { doc: Document { boxes, selected }, history: Vec::new(), mode, running: true }
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
        assert_eq!(Node::default().fill, None);
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
        assert_eq!(*at(&mut boxes, &Path { head: 1, tail: vec![] }), node("b"));
    }

    #[test]
    fn at_a_longer_path_walks_into_children() {
        let mut boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        assert_eq!(*at(&mut boxes, &Path { head: 0, tail: vec![1] }), node("d"));
    }

    #[test]
    fn at_a_deep_path_walks_multiple_levels() {
        let mut boxes = vec![node_with_children(
            "a",
            vec![node_with_children("b", vec![node("c")])],
        )];
        assert_eq!(*at(&mut boxes, &Path { head: 0, tail: vec![0, 0] }), node("c"));
    }

    #[test]
    fn grow_on_the_canvas_appends_a_top_level_box() {
        let mut boxes = Vec::new();
        let path = grow(&mut boxes, &None);
        assert_eq!(boxes, vec![node(PAD)]);
        assert_eq!(path, Path { head: 0, tail: vec![] });
    }

    #[test]
    fn grow_on_the_canvas_appends_after_existing_boxes() {
        let mut boxes = vec![node("a")];
        let path = grow(&mut boxes, &None);
        assert_eq!(boxes, vec![node("a"), node(PAD)]);
        assert_eq!(path, Path { head: 1, tail: vec![] });
    }

    #[test]
    fn grow_on_a_box_appends_a_child() {
        let mut boxes = vec![node("a")];
        let path = grow(&mut boxes, &Some(Path { head: 0, tail: vec![] }));
        assert_eq!(boxes, vec![node_with_children("a", vec![node(PAD)])]);
        assert_eq!(path, Path { head: 0, tail: vec![0] });
    }

    #[test]
    fn grow_on_a_box_with_a_child_appends_a_second_child() {
        let mut boxes = vec![node_with_children("a", vec![node("c")])];
        let path = grow(&mut boxes, &Some(Path { head: 0, tail: vec![] }));
        assert_eq!(
            boxes,
            vec![node_with_children("a", vec![node("c"), node(PAD)])]
        );
        assert_eq!(path, Path { head: 0, tail: vec![1] });
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
        colour_row(&mut boxes, &Path { head: 0, tail: vec![] });
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
        colour_row(&mut boxes, &Path { head: 0, tail: vec![] });
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
}
