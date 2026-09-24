use crate::command_mode;
use crate::diagram::{append, at, children_at, palette, Document, Node, Path, BACKGROUND};
use crate::insert_mode;
use crate::save_prompt_mode;

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
    SavePrompt {
        filename: String,
    },
}

#[derive(Clone)]
pub struct State {
    pub(crate) doc: Document,
    pub(crate) last_selected: Option<Path>,
    history: Vec<Document>,
    pub(crate) mode: Mode,
    pub(crate) running: bool,
    pub(crate) save_to: Option<String>,
    pub(crate) new_file: bool,
    pub(crate) pending_count: Option<usize>,
    pub(crate) scroll_x: i64,
    pub(crate) dirty: bool,
    pub(crate) colour_overlay: bool,
}

const CURSOR: char = '\u{2588}';

fn count_boxes(nodes: &[Node]) -> usize {
    nodes.iter().map(|n| 1 + count_boxes(&n.children)).sum()
}

pub struct StatusLine {
    pub left: Vec<Segment>,
    pub right: Vec<Segment>,
}

pub struct Segment {
    pub text: String,
    pub style: Style,
}

#[derive(Default, Clone, Copy)]
pub struct Style {
    pub bold: bool,
    pub background: Option<(u8, u8, u8, u8)>,
    pub foreground: Option<(u8, u8, u8)>,
}

const DIM_ALPHA: u8 = 0x40;

fn dim(text: impl Into<String>) -> Segment {
    Segment {
        text: text.into(),
        style: Style {
            bold: false,
            background: Some((0, 0, 0, DIM_ALPHA)),
            foreground: None,
        },
    }
}

pub fn status_line(state: &State) -> StatusLine {
    let mode_text = match &state.mode {
        Mode::Command | Mode::SavePrompt { .. } => "COMMANDING",
        Mode::Insert => "EDITING",
    };
    let (r, g, b) = palette(0).unwrap();
    let mode = Segment {
        text: format!(" {mode_text} "),
        style: Style {
            bold: true,
            background: Some((r, g, b, 0xFF)),
            foreground: palette(BACKGROUND),
        },
    };
    let filename = match &state.mode {
        Mode::SavePrompt { filename } => format!("Save as: {filename}{CURSOR}"),
        _ => {
            let name = state.save_to.as_deref().unwrap_or(DEFAULT_FILENAME);
            let marker = if state.dirty { "[+]" } else { "" };
            format!("{name}{marker}")
        }
    };
    let box_count = count_boxes(&state.doc.boxes);
    StatusLine {
        left: vec![mode, dim(" \u{2502} "), dim(filename)],
        right: vec![
            dim(format!("{box_count} boxes")),
            dim(" \u{2022} "),
            dim("dre"),
        ],
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            doc: Document::default(),
            last_selected: None,
            history: Vec::new(),
            mode: Mode::default(),
            running: true,
            save_to: None,
            new_file: false,
            pending_count: None,
            scroll_x: 0,
            dirty: false,
            colour_overlay: false,
        }
    }
}

pub(crate) fn load(doc: Document, save_to: Option<String>) -> State {
    let mut state = State {
        doc,
        save_to,
        ..Default::default()
    };
    if !state.doc.boxes.is_empty() {
        state.doc.selected = Some(Path {
            ancestors: vec![],
            index: 0,
        });
    }
    state
}

pub(crate) fn new_file(path: String) -> State {
    State {
        save_to: Some(path),
        new_file: true,
        ..Default::default()
    }
}

pub(crate) fn hide_idle_cursor(mut state: State) -> State {
    if state.mode == Mode::Command && state.doc.selected.is_some() {
        state.last_selected = state.doc.selected.clone();
        state.doc.selected = None;
    }
    state
}

pub(crate) fn snapshot(mut state: State) -> State {
    state.history.push(state.doc.clone());
    state.dirty = true;
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
        Some(i) if palette(i + 1).is_some() => Some(i + 1),
        Some(_) => None,
    }
}

pub(crate) fn colour_row(boxes: &mut Vec<Node>, path: &Path) {
    let siblings = children_at(boxes, &path.ancestors);
    let first_colour = siblings[0].colour;
    let uniform = siblings.iter().all(|b| b.colour == first_colour);
    let new_colour = if uniform {
        next_colour(first_colour)
    } else {
        Some(0)
    };
    for sibling in siblings.iter_mut() {
        sibling.colour = new_colour;
    }
}

pub(crate) fn blank_box() -> Node {
    Node {
        label: PAD.to_string(),
        ..Default::default()
    }
}

pub(crate) fn add_child_box(mut state: State, selected: Option<Path>) -> State {
    state.doc.selected = match selected {
        Some(mut path) => {
            let index = append(&mut at(&mut state.doc.boxes, &path).children, blank_box());
            path.ancestors.push(path.index);
            Some(Path {
                ancestors: path.ancestors,
                index,
            })
        }
        None => {
            let index = append(&mut state.doc.boxes, blank_box());
            Some(Path {
                ancestors: Vec::new(),
                index,
            })
        }
    };
    state.mode = Mode::Insert;
    state
}

pub(crate) fn handle_key(mut state: State, key: &str) -> State {
    if let Some(selected) = state.last_selected.take() {
        state.doc.selected = Some(selected);
    }
    if let Some(command @ command_mode::Command::ScrollBy(_)) = command_mode::parse(key) {
        return command_mode::reduce(state, command);
    }
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
                if state.colour_overlay {
                    let count = state.pending_count.take().unwrap();
                    state.colour_overlay = false;
                    if (1..=7).contains(&count) {
                        let path = state
                            .doc
                            .selected
                            .clone()
                            .expect("colour overlay implies a selection");
                        state = snapshot(state);
                        at(&mut state.doc.boxes, &path).colour = Some((count - 1) as u8);
                    }
                }
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
    State {
        doc: Document { boxes, selected },
        last_selected: None,
        history: Vec::new(),
        mode,
        running: true,
        save_to: None,
        new_file: false,
        pending_count: None,
        scroll_x: 0,
        dirty: false,
        colour_overlay: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{node, node_with_children};

    #[test]
    fn a_default_state_is_not_dirty() {
        assert!(!State::default().dirty);
    }

    #[test]
    fn snapshot_marks_the_state_as_dirty() {
        let state = new_state(vec![], Mode::Command, None);
        assert!(!state.dirty);
        let result = snapshot(state);
        assert!(result.dirty);
    }

    #[test]
    fn load_selects_the_first_box() {
        let state = load(
            Document {
                boxes: vec![node("a"), node("b")],
                selected: None,
            },
            None,
        );
        assert_eq!(state.doc.boxes, vec![node("a"), node("b")]);
        assert_eq!(
            state.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
    }

    #[test]
    fn load_of_an_empty_document_selects_nothing() {
        let state = load(Document::default(), None);
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
    }

    #[test]
    fn load_records_where_to_save_back_to() {
        let state = load(Document::default(), Some("diagram.dre".to_string()));
        assert_eq!(state.save_to, Some("diagram.dre".to_string()));
    }

    #[test]
    fn load_is_not_a_new_file() {
        let state = load(Document::default(), Some("diagram.dre".to_string()));
        assert!(!state.new_file);
    }

    #[test]
    fn new_file_is_empty_with_the_path_to_save_to() {
        let state = new_file("diagram.dre".to_string());
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
        assert_eq!(state.save_to, Some("diagram.dre".to_string()));
        assert!(state.new_file);
    }

    #[test]
    fn blank_box_is_a_default_box_labelled_with_the_pad() {
        assert_eq!(
            blank_box(),
            Node {
                label: PAD.to_string(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn add_child_box_without_a_selection_grows_a_top_level_box_and_enters_insert_mode() {
        let state = new_state(vec![], Mode::Command, None);
        let result = add_child_box(state, None);
        assert_eq!(result.doc.boxes, vec![node(PAD)]);
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn add_child_box_with_a_selection_grows_a_child_and_descends_the_path() {
        let state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = add_child_box(
            state,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
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
    fn next_colour_cycles_through_the_palette_and_back_to_plain() {
        let mut colour = None;
        for _ in (0..).take_while(|&i| palette(i).is_some()) {
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
        colour_row(
            &mut boxes,
            &Path {
                ancestors: vec![],
                index: 0,
            },
        );
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
        colour_row(
            &mut boxes,
            &Path {
                ancestors: vec![],
                index: 0,
            },
        );
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
        let state = new_state(
            boxes.clone(),
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 1,
            }),
        );
        for key in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
            let result = handle_key(state.clone(), key);
            assert_eq!(result.doc.boxes, boxes);
            assert_eq!(
                result.doc.selected,
                Some(Path {
                    ancestors: vec![],
                    index: 1
                })
            );
        }
    }

    #[test]
    fn a_default_state_starts_with_no_scroll() {
        let state = new_state(vec![], Mode::Command, None);
        assert_eq!(state.scroll_x, 0);
    }

    #[test]
    fn a_synthesized_scroll_key_shifts_scroll_x_and_leaves_selection_and_count_untouched() {
        let boxes = vec![node("a"), node("b")];
        let state = new_state(
            boxes.clone(),
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 1,
            }),
        );
        let result = handle_key(state, "\x1bSCROLL12");
        assert_eq!(result.scroll_x, 12);
        assert_eq!(result.doc.boxes, boxes);
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 1
            })
        );
        assert_eq!(result.pending_count, None);
    }

    #[test]
    fn a_negative_synthesized_scroll_key_shifts_scroll_x_backwards() {
        let state = new_state(vec![], Mode::Command, None);
        let result = handle_key(state, "\x1bSCROLL-7");
        assert_eq!(result.scroll_x, -7);
    }

    #[test]
    fn a_synthesized_scroll_key_takes_effect_in_insert_mode() {
        let state = new_state(
            vec![node("a")],
            Mode::Insert,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let result = handle_key(state, "\x1bSCROLL5");
        assert_eq!(result.scroll_x, 5);
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn digits_and_count_prefixed_movement_leave_mode_and_running_unchanged() {
        let boxes: Vec<Node> = (0..5).map(|i| node(&i.to_string())).collect();
        let state = new_state(
            boxes.clone(),
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
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
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let state = handle_key(state, "3");
        let result = handle_key(state, "2");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
        let result = handle_key(result, "j");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 32
            })
        );
    }

    #[test]
    fn repeated_digits_saturate_without_panic() {
        let boxes = vec![node("a"), node("b")];
        let state = new_state(
            boxes,
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let mut state = state;
        for _ in 0..20 {
            state = handle_key(state, "9");
        }
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
    fn an_idle_hide_in_command_mode_clears_the_selection_and_stashes_it() {
        let selected = Path {
            ancestors: vec![],
            index: 0,
        };
        let state = new_state(vec![node("a")], Mode::Command, Some(selected.clone()));
        let hidden = hide_idle_cursor(state);
        assert_eq!(hidden.doc.selected, None);
        assert_eq!(hidden.last_selected, Some(selected));
    }

    #[test]
    fn an_idle_hide_in_insert_mode_leaves_the_selection_alone() {
        let selected = Path {
            ancestors: vec![],
            index: 0,
        };
        let state = new_state(vec![node("a")], Mode::Insert, Some(selected.clone()));
        let hidden = hide_idle_cursor(state);
        assert_eq!(hidden.doc.selected, Some(selected));
        assert_eq!(hidden.last_selected, None);
    }

    #[test]
    fn an_idle_hide_in_save_prompt_mode_leaves_the_selection_alone() {
        let selected = Path {
            ancestors: vec![],
            index: 0,
        };
        let state = new_state(
            vec![node("a")],
            Mode::SavePrompt {
                filename: "a.dre".to_string(),
            },
            Some(selected.clone()),
        );
        let hidden = hide_idle_cursor(state);
        assert_eq!(hidden.doc.selected, Some(selected));
        assert_eq!(hidden.last_selected, None);
    }

    #[test]
    fn an_idle_hide_with_nothing_selected_is_a_noop() {
        let state = new_state(vec![node("a")], Mode::Command, None);
        let hidden = hide_idle_cursor(state);
        assert_eq!(hidden.doc.selected, None);
        assert_eq!(hidden.last_selected, None);
    }

    #[test]
    fn repeating_idle_hides_keep_the_stashed_selection() {
        let selected = Path {
            ancestors: vec![],
            index: 0,
        };
        let state = new_state(vec![node("a")], Mode::Command, Some(selected.clone()));
        let hidden = hide_idle_cursor(hide_idle_cursor(state));
        assert_eq!(hidden.doc.selected, None);
        assert_eq!(hidden.last_selected, Some(selected.clone()));
        let restored = handle_key(hidden, "z");
        assert_eq!(restored.doc.selected, Some(selected));
    }

    #[test]
    fn the_next_key_after_a_hide_lands_every_command_on_the_hidden_box() {
        let state = new_state(
            vec![node("a"), node("b")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        let hidden = hide_idle_cursor(state);
        let result = handle_key(hidden, "j");
        assert_eq!(
            result.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 1
            })
        );
    }

    #[test]
    fn any_key_after_a_hide_restores_the_selection() {
        let selected = Path {
            ancestors: vec![],
            index: 0,
        };
        let state = new_state(vec![node("a")], Mode::Command, Some(selected.clone()));
        let hidden = hide_idle_cursor(state);
        let result = handle_key(hidden, "z");
        assert_eq!(result.doc.selected, Some(selected));
    }

    #[test]
    fn an_idle_hide_is_not_undoable_and_does_not_pollute_history() {
        let mut state = new_state(
            vec![node("a")],
            Mode::Command,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        );
        state = snapshot(state);
        state = snapshot(state);
        let hidden = hide_idle_cursor(state);
        assert_eq!(hidden.history.len(), 2);
        assert_eq!(hidden.doc.selected, None);
    }

    fn texts(segments: &[Segment]) -> Vec<&str> {
        segments.iter().map(|s| s.text.as_str()).collect()
    }

    fn save_prompt_state() -> State {
        let mode = Mode::SavePrompt {
            filename: "diagram.dre".to_string(),
        };
        new_state(vec![], mode, None)
    }

    #[test]
    fn status_line_pads_the_mode_label_with_a_space_on_each_side() {
        let command = new_state(vec![], Mode::Command, None);
        let insert = new_state(vec![], Mode::Insert, None);
        assert_eq!(status_line(&command).left[0].text, " COMMANDING ");
        assert_eq!(status_line(&insert).left[0].text, " EDITING ");
    }

    #[test]
    fn status_line_keeps_showing_commanding_during_the_save_prompt() {
        let line = status_line(&save_prompt_state());
        assert_eq!(line.left[0].text, " COMMANDING ");
    }

    #[test]
    fn status_line_styles_the_mode_segment_bold_lime_on_the_app_background() {
        let style = status_line(&new_state(vec![], Mode::Command, None)).left[0].style;
        let (r, g, b) = palette(0).unwrap();
        assert!(style.bold);
        assert_eq!(style.background, Some((r, g, b, 0xFF)));
        assert_eq!(style.foreground, palette(BACKGROUND));
    }

    #[test]
    fn status_line_styles_every_other_segment_dim() {
        let line = status_line(&save_prompt_state());
        let dim: Vec<&Segment> = line.left[1..].iter().chain(line.right.iter()).collect();
        assert_eq!(dim.len(), 5);
        for segment in dim {
            assert!(!segment.style.bold);
            assert!(segment.style.background.is_some());
            assert_eq!(
                segment.style.background.map(|(r, g, b, _)| (r, g, b)),
                Some((0, 0, 0))
            );
            assert_eq!(segment.style.foreground, None);
        }
    }

    #[test]
    fn status_line_separates_the_mode_from_the_filename_with_a_vertical_bar() {
        let state = new_state(vec![], Mode::Command, None);
        assert_eq!(status_line(&state).left[1].text, " \u{2502} ");
    }

    #[test]
    fn status_line_shows_default_filename_in_command_mode() {
        let state = new_state(vec![], Mode::Command, None);
        assert_eq!(status_line(&state).left[2].text, "diagram.dre");
    }

    #[test]
    fn status_line_marks_dirty_state_with_a_plus_marker() {
        let mut state = new_state(vec![], Mode::Command, None);
        state.dirty = true;
        assert_eq!(status_line(&state).left[2].text, "diagram.dre[+]");
    }

    #[test]
    fn status_line_shows_the_save_to_path_as_is() {
        let mut state = new_state(vec![], Mode::Command, None);
        state.save_to = Some("/foo/bar.dre".to_string());
        assert_eq!(status_line(&state).left[2].text, "/foo/bar.dre");
    }

    #[test]
    fn status_line_shows_the_mode_and_filename_in_insert_mode() {
        let state = new_state(vec![], Mode::Insert, None);
        let line = status_line(&state);
        assert_eq!(
            texts(&line.left),
            [" EDITING ", " \u{2502} ", "diagram.dre"]
        );
    }

    #[test]
    fn status_line_shows_zero_boxes_when_empty() {
        let state = new_state(vec![], Mode::Command, None);
        assert_eq!(status_line(&state).right[0].text, "0 boxes");
    }

    #[test]
    fn status_line_counts_nested_children_in_box_count() {
        let state = new_state(
            vec![node_with_children("a", vec![node("b"), node("c")])],
            Mode::Command,
            None,
        );
        assert_eq!(status_line(&state).right[0].text, "3 boxes");
    }

    #[test]
    fn status_line_always_pluralizes_box_count() {
        let state = new_state(vec![node("a")], Mode::Command, None);
        assert_eq!(status_line(&state).right[0].text, "1 boxes");
    }

    #[test]
    fn status_line_ends_with_a_bullet_and_dre() {
        let state = new_state(vec![], Mode::Command, None);
        let line = status_line(&state);
        assert_eq!(texts(&line.right), ["0 boxes", " \u{2022} ", "dre"]);
    }

    #[test]
    fn status_line_shows_save_prompt_text_and_keeps_the_right_side() {
        let line = status_line(&save_prompt_state());
        assert_eq!(line.left[2].text, format!("Save as: diagram.dre{CURSOR}"));
        assert_eq!(texts(&line.right), ["0 boxes", " \u{2022} ", "dre"]);
    }
}
