mod action;
mod command;
mod effect;
mod history;
mod input;
mod insert;
mod mode;
mod name_prompt;
mod text_edit;

use crate::diagram::{Document, Node};
use crate::state::action::Action;
pub(crate) use crate::state::effect::Effect;
pub(crate) use crate::state::input::command_label;
#[cfg(test)]
pub(crate) use crate::state::input::INTERRUPT;
pub(crate) use crate::state::mode::Mode;
use types::Tree;

#[allow(dead_code)]
struct KeyBinding<C> {
    pub(crate) keys: &'static [&'static str],
    pub(crate) command: C,
    pub(crate) description: &'static str,
}

#[derive(Clone)]
pub struct State {
    doc: Document,
    selected: Option<Vec<usize>>,
    history: Vec<Document>,
    clipboard: Option<Tree<Node>>,
    mode: Mode,
    running: bool,
    save_to: Option<String>,
    new_file: bool,
    pending_count: Option<usize>,
    saved_len: usize,
    led_flash: bool,
    command_status: Option<CommandStatus>,
}

impl State {
    pub(crate) fn open(doc: Document, save_to: Option<String>) -> State {
        let mut state = State {
            doc,
            ..Default::default()
        };
        state.set_save_to(save_to);
        if state.doc.tree().contains(&[0]) {
            state.selected = Some(vec![0]);
        }
        state
    }

    pub(crate) fn new_file(path: String) -> State {
        let mut state = State {
            new_file: true,
            ..Default::default()
        };
        state.set_save_to(Some(path));
        state
    }

    pub(crate) fn doc(&self) -> &Document {
        &self.doc
    }

    pub(crate) fn mode(&self) -> &Mode {
        &self.mode
    }

    pub(crate) fn selected(&self) -> Option<&[usize]> {
        self.selected.as_deref()
    }

    pub(crate) fn is_running(&self) -> bool {
        self.running
    }

    #[cfg(test)]
    pub(crate) fn pending_count(&self) -> Option<usize> {
        self.pending_count
    }

    #[cfg(test)]
    pub(crate) fn is_new_file(&self) -> bool {
        self.new_file
    }

    #[cfg(test)]
    pub(crate) fn led_flash(&self) -> bool {
        self.led_flash
    }

    pub(crate) fn save_to(&self) -> Option<&str> {
        self.save_to.as_deref()
    }

    pub(crate) fn set_save_to(&mut self, save_to: Option<String>) {
        self.save_to = save_to;
    }

    pub(crate) fn command_status(&self) -> Option<CommandStatus> {
        self.command_status.clone()
    }

    pub(crate) fn footer(&self) -> FooterModel {
        match &self.mode {
            Mode::Insert { .. } => FooterModel {
                mode: FooterMode::Write,
                filename: self.save_to.as_deref().map(file_stem_without_dre),
                cursor: None,
                flash: false,
            },
            Mode::NamePrompt { name, .. } => FooterModel {
                mode: FooterMode::Naming,
                filename: (!name.is_empty()).then(|| name.clone()),
                cursor: Some(name.chars().count()),
                flash: false,
            },
            Mode::Command => FooterModel {
                mode: FooterMode::Move,
                filename: self.save_to.as_deref().map(file_stem_without_dre),
                cursor: None,
                flash: self.led_flash,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FooterMode {
    Move,
    Write,
    Naming,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FooterModel {
    pub(crate) mode: FooterMode,
    pub(crate) filename: Option<String>,
    pub(crate) cursor: Option<usize>,
    pub(crate) flash: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CommandStatus {
    pub(crate) key: String,
    pub(crate) name: &'static str,
    pub(crate) duration_ms: u128,
}

fn file_stem_without_dre(path: &str) -> String {
    let file_name = path.rsplit('/').next().unwrap_or(path);
    file_name
        .strip_suffix(".dre")
        .unwrap_or(file_name)
        .to_string()
}

impl Default for State {
    fn default() -> Self {
        State {
            doc: Document::default(),
            selected: None,
            history: Vec::new(),
            clipboard: None,
            mode: Mode::default(),
            running: true,
            save_to: None,
            new_file: false,
            pending_count: None,
            saved_len: 0,
            led_flash: false,
            command_status: None,
        }
    }
}

pub(crate) fn flash(state: State) -> State {
    let mut state = state;
    state.led_flash = true;
    state
}

pub(crate) fn set_command_status(
    state: State,
    key: String,
    name: &'static str,
    duration: std::time::Duration,
) -> State {
    let mut state = state;
    state.command_status = Some(CommandStatus {
        key,
        name,
        duration_ms: duration.as_millis(),
    });
    state
}

fn apply(state: State, action: Action) -> State {
    history::recorded(state, &action, |state| match action {
        Action::Insert(a) => insert::reduce(state, a),
        Action::NamePrompt(a) => name_prompt::reduce(state, a),
        Action::Selection(a) => command::reduce_selection(state, a),
        Action::Immediate(a) => command::reduce(state, a),
    })
}

pub fn reduce(state: State, key: &str) -> (State, Vec<Effect>) {
    let action = input::parse(&state, key);
    let mut state = match action {
        Some(action) => apply(state, action),
        None => state,
    };
    let effects = if state.save_to.is_some()
        && !matches!(state.mode, Mode::Insert { .. })
        && state.history.len() != state.saved_len
    {
        state.saved_len = state.history.len();
        vec![Effect::Save]
    } else {
        vec![]
    };
    state.led_flash = false;
    (state, effects)
}

fn add_child_box(mut state: State, selected: Option<Vec<usize>>) -> State {
    let parent = selected.unwrap_or_default();
    let new = state.doc.insert(&parent, &Tree::leaf(Node::default()));
    state.selected = Some(new);
    state.mode = Mode::Insert { cursor: 0 };
    state
}

#[cfg(test)]
pub(crate) fn new_state(boxes: Vec<Tree<Node>>, mode: Mode, selected: Option<Vec<usize>>) -> State {
    State {
        doc: Document::with_boxes(boxes),
        selected,
        history: Vec::new(),
        clipboard: None,
        mode,
        running: true,
        save_to: None,
        new_file: false,
        pending_count: None,
        saved_len: 0,
        led_flash: false,
        command_status: None,
    }
}

#[cfg(test)]
impl State {
    pub(crate) fn with_pending_count(mut self, count: usize) -> State {
        self.pending_count = Some(count);
        self
    }

    pub(crate) fn with_running(mut self, running: bool) -> State {
        self.running = running;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{node, node_with_children};
    use crate::state::action::{InsertAction, SelectionCommand};
    use crate::state::apply as reduce;
    use crate::state::text_edit::TextKey;
    use crate::test_support::handle_key;

    fn footer_filename_of(path: &str) -> Option<String> {
        State::open(Document::default(), Some(path.to_string()))
            .footer()
            .filename
    }

    #[test]
    fn footer_strips_the_folder_and_the_dre_extension() {
        assert_eq!(
            footer_filename_of("docs/plans.dre"),
            Some("plans".to_string())
        );
    }

    #[test]
    fn footer_strips_the_dre_extension_of_a_bare_file_name() {
        assert_eq!(footer_filename_of("plans.dre"), Some("plans".to_string()));
    }

    #[test]
    fn footer_of_a_path_without_the_dre_extension_only_strips_the_folder() {
        assert_eq!(footer_filename_of("docs/plans"), Some("plans".to_string()));
    }

    #[test]
    fn footer_of_a_default_state_has_no_name() {
        assert_eq!(State::default().footer().filename, None);
    }

    #[test]
    fn footer_of_a_new_file_is_named_after_its_path() {
        assert_eq!(
            State::new_file("docs/plans.dre".to_string())
                .footer()
                .filename,
            Some("plans".to_string())
        );
    }

    #[test]
    fn set_save_to_none_after_a_path_goes_back_to_no_name() {
        let mut state = State::open(Document::default(), Some("plans.dre".to_string()));
        state.set_save_to(None);
        assert_eq!(state.footer().filename, None);
        assert_eq!(state.save_to(), None);
    }

    #[test]
    fn set_save_to_a_path_after_none_updates_the_footer() {
        let mut state = State::default();
        state.set_save_to(Some("docs/plans.dre".to_string()));
        assert_eq!(state.footer().filename, Some("plans".to_string()));
        assert_eq!(state.save_to(), Some("docs/plans.dre"));
    }

    #[test]
    fn command_mode_footer_is_move_with_no_cursor() {
        let state = new_state(vec![], Mode::Command, None);
        let footer = state.footer();
        assert_eq!(footer.mode, FooterMode::Move);
        assert_eq!(footer.cursor, None);
    }

    #[test]
    fn command_mode_footer_flash_is_false_by_default() {
        let state = new_state(vec![], Mode::Command, None);
        assert!(!state.footer().flash);
    }

    #[test]
    fn command_mode_footer_flash_is_true_after_flash() {
        let state = flash(new_state(vec![], Mode::Command, None));
        assert!(state.footer().flash);
    }

    #[test]
    fn insert_mode_footer_flash_is_always_false() {
        let state = flash(new_state(
            vec![node("a")],
            Mode::Insert { cursor: 0 },
            Some(vec![0]),
        ));
        assert!(!state.footer().flash);
    }

    #[test]
    fn name_prompt_footer_flash_is_always_false() {
        let mut state = new_state(vec![], Mode::Command, None);
        state.mode = Mode::NamePrompt {
            name: String::new(),
            quits: false,
        };
        let state = flash(state);
        assert!(!state.footer().flash);
    }

    #[test]
    fn flash_sets_led_flash_to_true() {
        let state = new_state(vec![], Mode::Command, None);
        assert!(!state.led_flash());
        let flashed = flash(state);
        assert!(flashed.led_flash());
    }

    #[test]
    fn reduce_always_resets_led_flash_to_false() {
        let state = flash(new_state(vec![], Mode::Command, None));
        assert!(state.led_flash());
        let result = crate::state::reduce(state, "j").0;
        assert!(!result.led_flash());
    }

    #[test]
    fn a_default_state_has_no_command_status() {
        assert_eq!(State::default().command_status(), None);
    }

    #[test]
    fn set_command_status_stores_the_key_name_and_duration() {
        let state = new_state(vec![], Mode::Command, None);
        let state = set_command_status(
            state,
            "r".to_string(),
            "Rename",
            std::time::Duration::from_millis(42),
        );
        assert_eq!(
            state.command_status(),
            Some(CommandStatus {
                key: "r".to_string(),
                name: "Rename",
                duration_ms: 42,
            })
        );
    }

    #[test]
    fn command_status_persists_across_reduce_unlike_led_flash() {
        let state = set_command_status(
            new_state(vec![], Mode::Command, None),
            "r".to_string(),
            "Rename",
            std::time::Duration::from_millis(42),
        );
        let result = crate::state::reduce(state.clone(), "j").0;
        assert_eq!(result.command_status(), state.command_status());
    }

    #[test]
    fn insert_mode_footer_is_write_with_no_cursor() {
        let state = new_state(vec![node("a")], Mode::Insert { cursor: 0 }, Some(vec![0]));
        let footer = state.footer();
        assert_eq!(footer.mode, FooterMode::Write);
        assert_eq!(footer.cursor, None);
    }

    #[test]
    fn insert_mode_footer_filename_reflects_save_to() {
        let mut state = new_state(vec![], Mode::Insert { cursor: 0 }, None);
        state.set_save_to(Some("docs/plans.dre".to_string()));
        assert_eq!(state.footer().filename, Some("plans".to_string()));
    }

    #[test]
    fn name_prompt_footer_is_naming() {
        let state = new_state(
            vec![],
            Mode::NamePrompt {
                name: "ab".to_string(),
                quits: false,
            },
            None,
        );
        assert_eq!(state.footer().mode, FooterMode::Naming);
    }

    #[test]
    fn name_prompt_footer_cursor_sits_at_the_end_of_the_typed_name() {
        let state = new_state(
            vec![],
            Mode::NamePrompt {
                name: "ab".to_string(),
                quits: false,
            },
            None,
        );
        assert_eq!(state.footer().cursor, Some(2));
        assert_eq!(state.footer().filename, Some("ab".to_string()));
    }

    #[test]
    fn an_empty_name_prompt_cursor_sits_at_index_zero() {
        let state = new_state(
            vec![],
            Mode::NamePrompt {
                name: String::new(),
                quits: false,
            },
            None,
        );
        assert_eq!(state.footer().cursor, Some(0));
        assert_eq!(state.footer().filename, None);
    }

    #[test]
    fn open_selects_the_first_box() {
        let state = State::open(Document::with_boxes(vec![node("a"), node("b")]), None);
        assert_eq!(*state.doc.tree(), Tree::root(vec![node("a"), node("b")]));
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
        assert_eq!(state.save_to(), Some("diagram.dre"));
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
        assert_eq!(state.save_to(), Some("diagram.dre"));
        assert!(state.new_file);
    }

    #[test]
    fn add_child_box_without_a_selection_grows_a_top_level_box_and_enters_insert_mode() {
        let state = new_state(vec![], Mode::Command, None);
        let result = add_child_box(state, None);
        assert_eq!(*result.doc.tree(), Tree::root(vec![node("")]));
        assert_eq!(result.selected, Some(vec![0]));
        assert_eq!(result.mode, Mode::Insert { cursor: 0 });
    }

    #[test]
    fn add_child_box_with_a_selection_grows_a_child_and_descends_the_path() {
        let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
        let result = add_child_box(state, Some(vec![0]));
        assert_eq!(
            *result.doc.tree(),
            Tree::root(vec![node_with_children("a", vec![node("")])])
        );
        assert_eq!(result.selected, Some(vec![0, 0]));
        assert_eq!(result.mode, Mode::Insert { cursor: 0 });
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
        assert_eq!(*result.doc.tree(), *state.doc.tree());
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
            assert_eq!(*result.doc.tree(), Tree::root(boxes.clone()));
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

    fn selecting_first(boxes: Vec<Tree<Node>>, mode: Mode) -> State {
        new_state(boxes, mode, Some(vec![0]))
    }

    #[test]
    fn commit_and_add_child_after_an_edit_leaves_two_snapshots() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let typed = reduce(
            reduce(before, Action::Selection(SelectionCommand::EditLabel)),
            Action::Insert(InsertAction::InsertKey(TextKey::Char('b'))),
        );
        let result = reduce(typed, Action::Insert(InsertAction::CommitAndAddChild));
        assert_eq!(result.history.len(), 2);
    }

    #[test]
    fn commit_and_add_child_after_an_edit_needs_two_undos_to_restore_the_document() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let typed = reduce(
            reduce(
                before.clone(),
                Action::Selection(SelectionCommand::EditLabel),
            ),
            Action::Insert(InsertAction::InsertKey(TextKey::Char('b'))),
        );
        let committed = reduce(typed, Action::Insert(InsertAction::CommitAndAddChild));
        let once = reduce(committed, Action::Selection(SelectionCommand::Undo));
        let twice = reduce(once.clone(), Action::Selection(SelectionCommand::Undo));
        assert_ne!(*once.doc.tree(), *before.doc.tree());
        assert_eq!(*twice.doc.tree(), *before.doc.tree());
    }

    #[test]
    fn commit_and_add_child_without_a_change_keeps_the_edit_and_the_commit_snapshots() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let result = reduce(
            reduce(before, Action::Selection(SelectionCommand::EditLabel)),
            Action::Insert(InsertAction::CommitAndAddChild),
        );
        assert_eq!(result.history.len(), 2);
    }

    #[test]
    fn new_sibling_leaves_one_snapshot_and_needs_one_undo_to_restore_the_document() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let created = reduce(
            before.clone(),
            Action::Selection(SelectionCommand::NewSibling),
        );
        assert_eq!(created.history.len(), 1);
        let once = reduce(created, Action::Selection(SelectionCommand::Undo));
        assert_eq!(*once.doc.tree(), *before.doc.tree());
    }

    #[test]
    fn rename_label_leaves_one_snapshot_and_needs_one_undo_to_restore_the_document() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let renaming = reduce(
            before.clone(),
            Action::Selection(SelectionCommand::RenameLabel),
        );
        assert_eq!(renaming.history.len(), 1);
        let once = reduce(renaming, Action::Selection(SelectionCommand::Undo));
        assert_eq!(*once.doc.tree(), *before.doc.tree());
    }

    #[test]
    fn committing_an_edit_label_without_a_change_leaves_one_snapshot() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let editing = reduce(
            before.clone(),
            Action::Selection(SelectionCommand::EditLabel),
        );
        let committed = reduce(editing, Action::Insert(InsertAction::Commit));
        assert_eq!(committed.history.len(), before.history.len() + 1);
        let undone = reduce(committed, Action::Selection(SelectionCommand::Undo));
        assert_eq!(*undone.doc.tree(), *before.doc.tree());
    }

    #[test]
    fn undo_does_not_add_a_snapshot() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let edited = reduce(
            reduce(before, Action::Selection(SelectionCommand::NewBox)),
            Action::Selection(SelectionCommand::Undo),
        );
        assert_eq!(edited.history.len(), 0);
    }

    mod effects {
        use super::*;
        use crate::state::effect::Effect;

        fn saved_state(boxes: Vec<Tree<Node>>, selected: Option<Vec<usize>>) -> State {
            let mut state = new_state(boxes, Mode::Command, selected);
            state.set_save_to(Some("a.dre".to_string()));
            state
        }

        #[test]
        fn a_change_that_grows_history_returns_a_save_effect_when_save_to_is_set() {
            let state = saved_state(vec![node("a")], Some(vec![0]));
            let (_, effects) = crate::state::reduce(state, "r");
            assert_eq!(effects, vec![Effect::Save]);
        }

        #[test]
        fn a_selection_move_returns_no_effects() {
            let state = saved_state(vec![node("a"), node("b")], Some(vec![0]));
            let (_, effects) = crate::state::reduce(state, "j");
            assert_eq!(effects, vec![]);
        }

        #[test]
        fn keys_typed_in_insert_mode_return_no_effects_until_escape_exits_it() {
            let state = saved_state(vec![node("a")], Some(vec![0]));
            let (state, effects) = crate::state::reduce(state, "i");
            assert_eq!(effects, vec![]);
            let (state, effects) = crate::state::reduce(state, "b");
            assert_eq!(effects, vec![]);
            let (state, effects) = crate::state::reduce(state, "c");
            assert_eq!(effects, vec![]);
            let (_, effects) = crate::state::reduce(state, "\x1b");
            assert_eq!(effects, vec![Effect::Save]);
        }

        #[test]
        fn commit_and_add_child_stays_in_insert_and_only_the_following_escape_saves_once() {
            let state = saved_state(vec![node("a")], Some(vec![0]));
            let (state, effects) = crate::state::reduce(state, "i");
            assert_eq!(effects, vec![]);
            let (state, effects) = crate::state::reduce(state, "b");
            assert_eq!(effects, vec![]);
            let (state, effects) = crate::state::reduce(state, "\r");
            assert_eq!(effects, vec![]);
            assert!(matches!(state.mode(), Mode::Insert { .. }));
            let (state, effects) = crate::state::reduce(state, "c");
            assert_eq!(effects, vec![]);
            let (_, effects) = crate::state::reduce(state, "\x1b");
            assert_eq!(effects, vec![Effect::Save]);
        }

        #[test]
        fn undo_returns_a_save_effect() {
            let state = saved_state(vec![node("a")], Some(vec![0]));
            let (state, effects) = crate::state::reduce(state, "r");
            assert_eq!(effects, vec![Effect::Save]);
            let (_, effects) = crate::state::reduce(state, "u");
            assert_eq!(effects, vec![Effect::Save]);
        }

        #[test]
        fn a_no_op_edit_still_leaves_a_snapshot_and_returns_a_save_effect() {
            let state = saved_state(vec![node("a")], Some(vec![0]));
            let (state, effects) = crate::state::reduce(state, "i");
            assert_eq!(effects, vec![]);
            let (state, effects) = crate::state::reduce(state, "\x1b");
            assert_eq!(effects, vec![Effect::Save]);
            assert_eq!(state.history.len(), 1);
        }

        #[test]
        fn a_second_change_after_a_save_returns_a_save_effect_again() {
            let state = saved_state(vec![node("a")], Some(vec![0]));
            let (state, effects) = crate::state::reduce(state, "r");
            assert_eq!(effects, vec![Effect::Save]);
            let (_, effects) = crate::state::reduce(state, "r");
            assert_eq!(effects, vec![Effect::Save]);
        }

        #[test]
        fn with_no_save_path_no_effects_are_ever_returned() {
            let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
            assert_eq!(state.save_to(), None);
            let (state, effects) = crate::state::reduce(state, "r");
            assert_eq!(effects, vec![]);
            let (state, effects) = crate::state::reduce(state, "i");
            assert_eq!(effects, vec![]);
            let (state, effects) = crate::state::reduce(state, "\x1b");
            assert_eq!(effects, vec![]);
            let (_, effects) = crate::state::reduce(state, "u");
            assert_eq!(effects, vec![]);
        }

        #[test]
        fn an_interrupt_after_a_change_returns_no_effects() {
            let state = saved_state(vec![node("a")], Some(vec![0]));
            let (state, effects) = crate::state::reduce(state, "r");
            assert_eq!(effects, vec![Effect::Save]);
            let (_, effects) = crate::state::reduce(state, INTERRUPT);
            assert_eq!(effects, vec![]);
        }
    }
}
