mod action;
mod command;
mod history;
mod input;
mod insert;
mod mode;
mod save_prompt;

use crate::diagram::{append, at, children_at, Document, Node, Path};
use crate::palette::palette;
use crate::state::action::ActionMode;
#[cfg(not(test))]
use crate::state::input::INTERRUPT;
#[cfg(test)]
pub(crate) use crate::state::input::INTERRUPT;
#[cfg(test)]
pub(crate) use crate::state::mode::Mode;
#[cfg(not(test))]
use crate::state::mode::Mode;
use crate::status_line::{ModeLabel, StatusInput};

const PAD: &str = " ";
const DEFAULT_FILENAME: &str = "diagram.dre";

#[allow(dead_code)]
struct KeyBinding<C> {
    pub(crate) keys: &'static [&'static str],
    pub(crate) command: C,
    pub(crate) description: &'static str,
}

#[derive(Clone)]
pub struct State {
    pub(crate) doc: Document,
    pub(crate) last_selected: Option<Path>,
    history: Vec<Document>,
    pub(crate) clipboard: Option<Node>,
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

impl State {
    pub(crate) fn open(doc: Document, save_to: Option<String>) -> State {
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

    pub(crate) fn status_input(&self) -> StatusInput {
        let mode = match &self.mode {
            Mode::Command | Mode::SavePrompt { .. } => ModeLabel::Commanding,
            Mode::Insert => ModeLabel::Editing,
        };
        let filename = match &self.mode {
            Mode::SavePrompt { filename } => format!("Save as: {filename}{CURSOR}"),
            _ => {
                let name = self.save_to.as_deref().unwrap_or(DEFAULT_FILENAME);
                let marker = if self.dirty { "[+]" } else { "" };
                format!("{name}{marker}")
            }
        };
        StatusInput {
            mode,
            filename,
            box_count: count_boxes(&self.doc.boxes),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            doc: Document::default(),
            last_selected: None,
            history: Vec::new(),
            clipboard: None,
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

fn apply(mut state: State, action: action::Action) -> State {
    if let Some(selected) = state.last_selected.take() {
        state.doc.selected = Some(selected);
    }
    history::recorded(state, &action, |state| match action.mode() {
        ActionMode::Insert => insert::reduce(state, action),
        ActionMode::SavePrompt => save_prompt::reduce(state, action),
        ActionMode::Command => command::reduce(state, action),
    })
}

pub fn reduce(state: State, key: Option<&str>) -> State {
    let action = match key {
        None => Some(action::Action::Idle),
        Some(INTERRUPT) => Some(action::Action::Interrupt),
        Some(key) => input::parse(&state, key),
    };
    match action {
        Some(action) => apply(state, action),
        None => state,
    }
}

fn next_colour(colour: Option<u8>) -> Option<u8> {
    match colour {
        None => Some(0),
        Some(i) if palette(i + 1).is_some() => Some(i + 1),
        Some(_) => None,
    }
}

fn colour_row(boxes: &mut Vec<Node>, path: &Path) {
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

fn blank_box() -> Node {
    Node {
        label: PAD.to_string(),
        ..Default::default()
    }
}

fn add_child_box(mut state: State, selected: Option<Path>) -> State {
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

#[cfg(test)]
pub(crate) fn new_state(boxes: Vec<Node>, mode: Mode, selected: Option<Path>) -> State {
    State {
        doc: Document { boxes, selected },
        last_selected: None,
        history: Vec::new(),
        clipboard: None,
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
    use crate::state::action::Action;
    use crate::state::apply as reduce;
    use crate::test_support::handle_key;

    #[test]
    fn a_default_state_is_not_dirty() {
        assert!(!State::default().dirty);
    }

    #[test]
    fn open_selects_the_first_box() {
        let state = State::open(
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
    fn open_of_an_empty_document_selects_nothing() {
        let state = State::open(Document::default(), None);
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
    }

    #[test]
    fn open_records_where_to_save_back_to() {
        let state = State::open(Document::default(), Some("diagram.dre".to_string()));
        assert_eq!(state.save_to, Some("diagram.dre".to_string()));
    }

    #[test]
    fn open_is_not_a_new_file() {
        let state = State::open(Document::default(), Some("diagram.dre".to_string()));
        assert!(!state.new_file);
    }

    #[test]
    fn new_file_is_empty_with_the_path_to_save_to() {
        let state = State::new_file("diagram.dre".to_string());
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
        let result = reduce(state, Action::ScrollBy(12));
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
        let result = reduce(state, Action::ScrollBy(-7));
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
        let result = reduce(state, Action::ScrollBy(5));
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
        let hidden = reduce(state, Action::Idle);
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
        let hidden = reduce(state, Action::Idle);
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
        let hidden = reduce(state, Action::Idle);
        assert_eq!(hidden.doc.selected, Some(selected));
        assert_eq!(hidden.last_selected, None);
    }

    #[test]
    fn an_idle_hide_with_nothing_selected_is_a_noop() {
        let state = new_state(vec![node("a")], Mode::Command, None);
        let hidden = reduce(state, Action::Idle);
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
        let hidden = reduce(reduce(state, Action::Idle), Action::Idle);
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
        let hidden = reduce(state, Action::Idle);
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
        let hidden = reduce(state, Action::Idle);
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
        state = reduce(state, Action::ToggleRounded);
        state = reduce(state, Action::ToggleRounded);
        let hidden = reduce(state, Action::Idle);
        assert_eq!(hidden.history.len(), 2);
        assert_eq!(hidden.doc.selected, None);
    }

    fn selecting_first(boxes: Vec<Node>, mode: Mode) -> State {
        new_state(
            boxes,
            mode,
            Some(Path {
                ancestors: vec![],
                index: 0,
            }),
        )
    }

    #[test]
    fn commit_and_add_child_after_an_edit_leaves_two_snapshots() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let typed = reduce(reduce(before, Action::EditLabel), Action::InsertAppend('b'));
        let result = reduce(typed, Action::CommitAndAddChild);
        assert_eq!(result.history.len(), 2);
    }

    #[test]
    fn commit_and_add_child_after_an_edit_needs_two_undos_to_restore_the_document() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let typed = reduce(
            reduce(before.clone(), Action::EditLabel),
            Action::InsertAppend('b'),
        );
        let committed = reduce(typed, Action::CommitAndAddChild);
        let once = reduce(committed, Action::Undo);
        let twice = reduce(once.clone(), Action::Undo);
        assert_ne!(once.doc.boxes, before.doc.boxes);
        assert_eq!(twice.doc.boxes, before.doc.boxes);
    }

    #[test]
    fn commit_and_add_child_without_a_change_keeps_the_edit_and_the_commit_snapshots() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let result = reduce(reduce(before, Action::EditLabel), Action::CommitAndAddChild);
        assert_eq!(result.history.len(), 2);
    }

    #[test]
    fn new_sibling_leaves_one_snapshot_and_needs_one_undo_to_restore_the_document() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let created = reduce(before.clone(), Action::NewSibling);
        assert_eq!(created.history.len(), 1);
        let once = reduce(created, Action::Undo);
        assert_eq!(once.doc.boxes, before.doc.boxes);
    }

    #[test]
    fn rename_label_leaves_one_snapshot_and_needs_one_undo_to_restore_the_document() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let renaming = reduce(before.clone(), Action::RenameLabel);
        assert_eq!(renaming.history.len(), 1);
        let once = reduce(renaming, Action::Undo);
        assert_eq!(once.doc.boxes, before.doc.boxes);
    }

    #[test]
    fn committing_an_edit_label_without_a_change_leaves_history_unchanged() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let editing = reduce(before.clone(), Action::EditLabel);
        let committed = reduce(editing, Action::Commit);
        assert_eq!(committed.history.len(), before.history.len());
        assert_eq!(committed.doc.boxes, before.doc.boxes);
    }

    #[test]
    fn undo_does_not_add_a_snapshot() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let edited = reduce(reduce(before, Action::NewBox), Action::Undo);
        assert_eq!(edited.history.len(), 0);
    }

    fn save_prompt_state() -> State {
        let mode = Mode::SavePrompt {
            filename: "diagram.dre".to_string(),
        };
        new_state(vec![], mode, None)
    }

    #[test]
    fn status_input_labels_command_mode_as_commanding() {
        let input = new_state(vec![], Mode::Command, None).status_input();
        assert!(matches!(input.mode, ModeLabel::Commanding));
    }

    #[test]
    fn status_input_labels_insert_mode_as_editing() {
        let input = new_state(vec![], Mode::Insert, None).status_input();
        assert!(matches!(input.mode, ModeLabel::Editing));
    }

    #[test]
    fn status_input_keeps_commanding_during_the_save_prompt() {
        let input = save_prompt_state().status_input();
        assert!(matches!(input.mode, ModeLabel::Commanding));
    }

    #[test]
    fn status_input_uses_the_default_filename_without_save_to() {
        let input = new_state(vec![], Mode::Command, None).status_input();
        assert_eq!(input.filename, DEFAULT_FILENAME);
    }

    #[test]
    fn status_input_marks_dirty_state_with_a_plus_marker() {
        let mut state = new_state(vec![], Mode::Command, None);
        state.dirty = true;
        assert_eq!(
            state.status_input().filename,
            format!("{DEFAULT_FILENAME}[+]")
        );
    }

    #[test]
    fn status_input_uses_the_save_to_path_as_is() {
        let mut state = new_state(vec![], Mode::Insert, None);
        state.save_to = Some("/foo/bar.dre".to_string());
        assert_eq!(state.status_input().filename, "/foo/bar.dre");
    }

    #[test]
    fn status_input_shows_the_save_prompt_text_with_a_cursor() {
        let input = save_prompt_state().status_input();
        assert_eq!(
            input.filename,
            format!("Save as: {DEFAULT_FILENAME}{CURSOR}")
        );
    }

    #[test]
    fn status_input_counts_zero_boxes_when_empty() {
        let input = new_state(vec![], Mode::Command, None).status_input();
        assert_eq!(input.box_count, 0);
    }

    #[test]
    fn status_input_counts_nested_children() {
        let state = new_state(
            vec![node_with_children("a", vec![node("b"), node("c")])],
            Mode::Command,
            None,
        );
        assert_eq!(state.status_input().box_count, 3);
    }
}
