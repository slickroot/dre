mod action;
mod command;
mod history;
mod input;
mod insert;
mod mode;
mod save_prompt;

use crate::diagram::{Document, Node};
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
use types::Tree;

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
    pub(crate) selected: Option<Vec<usize>>,
    pub(crate) last_selected: Option<Vec<usize>>,
    history: Vec<Document>,
    pub(crate) clipboard: Option<Tree<Node>>,
    pub(crate) mode: Mode,
    pub(crate) running: bool,
    pub(crate) save_to: Option<String>,
    pub(crate) new_file: bool,
    pub(crate) pending_count: Option<usize>,
    pub(crate) dirty: bool,
}

const CURSOR: char = '\u{2588}';

impl State {
    pub(crate) fn open(doc: Document, save_to: Option<String>) -> State {
        let mut state = State {
            doc,
            save_to,
            ..Default::default()
        };
        if state.doc.tree().contains(&[0]) {
            state.selected = Some(vec![0]);
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
            box_count: self.doc.tree().walk().count(),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            doc: Document::default(),
            selected: None,
            last_selected: None,
            history: Vec::new(),
            clipboard: None,
            mode: Mode::default(),
            running: true,
            save_to: None,
            new_file: false,
            pending_count: None,
            dirty: false,
        }
    }
}

fn apply(mut state: State, action: action::Action) -> State {
    if let Some(selected) = state.last_selected.take() {
        state.selected = Some(selected);
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

fn add_child_box(mut state: State, selected: Option<Vec<usize>>) -> State {
    let parent = selected.unwrap_or_default();
    let new = state.doc.insert(&parent, &Tree::leaf(Node::default()));
    state.doc.set_label(&new, PAD.to_string());
    state.selected = Some(new);
    state.mode = Mode::Insert;
    state
}

#[cfg(test)]
pub(crate) fn new_state(boxes: Vec<Tree<Node>>, mode: Mode, selected: Option<Vec<usize>>) -> State {
    State {
        doc: Document {
            root: Tree::root(boxes),
        },
        selected,
        last_selected: None,
        history: Vec::new(),
        clipboard: None,
        mode,
        running: true,
        save_to: None,
        new_file: false,
        pending_count: None,
        dirty: false,
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
                root: Tree::root(vec![node("a"), node("b")]),
            },
            None,
        );
        assert_eq!(state.doc.root, Tree::root(vec![node("a"), node("b")]));
        assert_eq!(state.selected, Some(vec![0]));
    }

    #[test]
    fn open_of_an_empty_document_selects_nothing() {
        let state = State::open(Document::default(), None);
        assert_eq!(state.doc, Document::default());
        assert_eq!(state.selected, None);
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
        assert_eq!(state.doc, Document::default());
        assert_eq!(state.selected, None);
        assert_eq!(state.save_to, Some("diagram.dre".to_string()));
        assert!(state.new_file);
    }

    #[test]
    fn add_child_box_without_a_selection_grows_a_top_level_box_and_enters_insert_mode() {
        let state = new_state(vec![], Mode::Command, None);
        let result = add_child_box(state, None);
        assert_eq!(result.doc.root, Tree::root(vec![node(PAD)]));
        assert_eq!(result.selected, Some(vec![0]));
        assert_eq!(result.mode, Mode::Insert);
    }

    #[test]
    fn add_child_box_with_a_selection_grows_a_child_and_descends_the_path() {
        let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let result = add_child_box(state, Some(vec![0]));
        assert_eq!(
            result.doc.root,
            Tree::root(vec![node_with_children("a", vec![node(PAD)])])
        );
        assert_eq!(result.selected, Some(vec![0, 0]));
        assert_eq!(result.mode, Mode::Insert);
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
        assert_eq!(result.doc.root, state.doc.root);
        assert_eq!(result.selected, state.selected);
        assert_eq!(result.mode, state.mode);
        assert_eq!(result.running, state.running);
    }

    #[test]
    fn a_bare_digit_in_command_mode_leaves_the_document_unchanged() {
        let boxes = vec![node("a"), node("b")];
        let state = new_state(boxes.clone(), Mode::Command, Some(vec![1]));
        for key in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
            let result = handle_key(state.clone(), key);
            assert_eq!(result.doc.root, Tree::root(boxes.clone()));
            assert_eq!(result.selected, Some(vec![1]));
        }
    }

    #[test]
    fn digits_and_count_prefixed_movement_leave_mode_and_running_unchanged() {
        let boxes: Vec<Tree<Node>> = (0..5).map(|i| node(&i.to_string())).collect();
        let state = new_state(boxes.clone(), Mode::Command, Some(vec![0]));
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
        let boxes: Vec<Tree<Node>> = (0..40).map(|i| node(&i.to_string())).collect();
        let state = new_state(boxes, Mode::Command, Some(vec![0]));
        let state = handle_key(state, "3");
        let result = handle_key(state, "2");
        assert_eq!(result.selected, Some(vec![0]));
        let result = handle_key(result, "j");
        assert_eq!(result.selected, Some(vec![32]));
    }

    #[test]
    fn repeated_digits_saturate_without_panic() {
        let boxes = vec![node("a"), node("b")];
        let state = new_state(boxes, Mode::Command, Some(vec![0]));
        let mut state = state;
        for _ in 0..20 {
            state = handle_key(state, "9");
        }
        let state = handle_key(state, "x");
        let result = handle_key(state, "j");
        assert_eq!(result.selected, Some(vec![1]));
    }

    #[test]
    fn an_idle_hide_in_command_mode_clears_the_selection_and_stashes_it() {
        let selected = vec![0];
        let state = new_state(vec![node("a")], Mode::Command, Some(selected.clone()));
        let hidden = reduce(state, Action::Idle);
        assert_eq!(hidden.selected, None);
        assert_eq!(hidden.last_selected, Some(selected));
    }

    #[test]
    fn an_idle_hide_in_insert_mode_leaves_the_selection_alone() {
        let selected = vec![0];
        let state = new_state(vec![node("a")], Mode::Insert, Some(selected.clone()));
        let hidden = reduce(state, Action::Idle);
        assert_eq!(hidden.selected, Some(selected));
        assert_eq!(hidden.last_selected, None);
    }

    #[test]
    fn an_idle_hide_in_save_prompt_mode_leaves_the_selection_alone() {
        let selected = vec![0];
        let state = new_state(
            vec![node("a")],
            Mode::SavePrompt {
                filename: "a.dre".to_string(),
            },
            Some(selected.clone()),
        );
        let hidden = reduce(state, Action::Idle);
        assert_eq!(hidden.selected, Some(selected));
        assert_eq!(hidden.last_selected, None);
    }

    #[test]
    fn an_idle_hide_with_nothing_selected_is_a_noop() {
        let state = new_state(vec![node("a")], Mode::Command, None);
        let hidden = reduce(state, Action::Idle);
        assert_eq!(hidden.selected, None);
        assert_eq!(hidden.last_selected, None);
    }

    #[test]
    fn repeating_idle_hides_keep_the_stashed_selection() {
        let selected = vec![0];
        let state = new_state(vec![node("a")], Mode::Command, Some(selected.clone()));
        let hidden = reduce(reduce(state, Action::Idle), Action::Idle);
        assert_eq!(hidden.selected, None);
        assert_eq!(hidden.last_selected, Some(selected.clone()));
        let restored = handle_key(hidden, "z");
        assert_eq!(restored.selected, Some(selected));
    }

    #[test]
    fn the_next_key_after_a_hide_lands_every_command_on_the_hidden_box() {
        let state = new_state(vec![node("a"), node("b")], Mode::Command, Some(vec![0]));
        let hidden = reduce(state, Action::Idle);
        let result = handle_key(hidden, "j");
        assert_eq!(result.selected, Some(vec![1]));
    }

    #[test]
    fn any_key_after_a_hide_restores_the_selection() {
        let selected = vec![0];
        let state = new_state(vec![node("a")], Mode::Command, Some(selected.clone()));
        let hidden = reduce(state, Action::Idle);
        let result = handle_key(hidden, "z");
        assert_eq!(result.selected, Some(selected));
    }

    #[test]
    fn an_idle_hide_is_not_undoable_and_does_not_pollute_history() {
        let mut state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        state = reduce(state, Action::ToggleRounded);
        state = reduce(state, Action::ToggleRounded);
        let hidden = reduce(state, Action::Idle);
        assert_eq!(hidden.history.len(), 2);
        assert_eq!(hidden.selected, None);
    }

    fn selecting_first(boxes: Vec<Tree<Node>>, mode: Mode) -> State {
        new_state(boxes, mode, Some(vec![0]))
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
        assert_ne!(once.doc.root, before.doc.root);
        assert_eq!(twice.doc.root, before.doc.root);
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
        assert_eq!(once.doc.root, before.doc.root);
    }

    #[test]
    fn rename_label_leaves_one_snapshot_and_needs_one_undo_to_restore_the_document() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let renaming = reduce(before.clone(), Action::RenameLabel);
        assert_eq!(renaming.history.len(), 1);
        let once = reduce(renaming, Action::Undo);
        assert_eq!(once.doc.root, before.doc.root);
    }

    #[test]
    fn committing_an_edit_label_without_a_change_leaves_history_unchanged() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let editing = reduce(before.clone(), Action::EditLabel);
        let committed = reduce(editing, Action::Commit);
        assert_eq!(committed.history.len(), before.history.len());
        assert_eq!(committed.doc.root, before.doc.root);
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
