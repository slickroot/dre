use crate::command_mode;
use crate::diagram::{append, at, children_at, Node, Path};
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

pub(crate) fn colour_row(boxes: &mut Vec<Node>, path: &Path) {
    let siblings = children_at(boxes, &path.ancestors);
    let first_colour = siblings[0].colour;
    let uniform = siblings.iter().all(|b| b.colour == first_colour);
    let new_colour = if uniform { next_colour(first_colour) } else { Some(0) };
    for sibling in siblings.iter_mut() {
        sibling.colour = new_colour;
    }
}

pub(crate) fn blank_box() -> Node {
    Node { label: PAD.to_string(), ..Default::default() }
}

pub(crate) fn add_child_box(mut state: State, selected: Option<Path>) -> State {
    state.doc.selected = match selected {
        Some(mut path) => {
            let index = append(&mut at(&mut state.doc.boxes, &path).children, blank_box());
            path.ancestors.push(path.index);
            Some(Path { ancestors: path.ancestors, index })
        }
        None => {
            let index = append(&mut state.doc.boxes, blank_box());
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
    use crate::diagram::{node, node_with_children};

    #[test]
    fn blank_box_is_a_default_box_labelled_with_the_pad() {
        assert_eq!(blank_box(), Node { label: PAD.to_string(), ..Default::default() });
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
    fn a_bare_digit_in_command_mode_leaves_the_document_unchanged() {
        let boxes = vec![node("a"), node("b")];
        let state = new_state(boxes.clone(), Mode::Command, Some(Path { ancestors: vec![], index: 1 }));
        for key in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
            let result = handle_key(state.clone(), key);
            assert_eq!(result.doc.boxes, boxes);
            assert_eq!(result.doc.selected, Some(Path { ancestors: vec![], index: 1 }));
        }
    }

    #[test]
    fn digits_and_count_prefixed_movement_leave_mode_and_running_unchanged() {
        let boxes: Vec<Node> = (0..5).map(|i| node(&i.to_string())).collect();
        let state = new_state(boxes.clone(), Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
        for key in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
            let result = handle_key(state.clone(), key);
            assert_eq!(result.mode, Mode::Command);
            assert_eq!(result.running, state.running);
        }
        let result = handle_key(handle_key(handle_key(state.clone(), "3"), "2"), "j");
        assert_eq!(result.mode, Mode::Command);
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
