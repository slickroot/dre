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
use crate::palette;
use crate::state::action::ActionMode;
pub(crate) use crate::state::effect::Effect;
#[cfg(test)]
pub(crate) use crate::state::input::INTERRUPT;
pub(crate) use crate::state::mode::Mode;
use types::Tree;

const NO_NAME: &str = "[no name — press n to name it]";
const PLACEHOLDER: &str = "type a name";
const FOOTER_SUFFIX: &str = " • dre";
const MOVE: &str = "MOVE";
const WRITE: &str = "WRITE";

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

    pub(crate) fn save_to(&self) -> Option<&str> {
        self.save_to.as_deref()
    }

    pub(crate) fn set_save_to(&mut self, save_to: Option<String>) {
        self.save_to = save_to;
    }

    pub(crate) fn footer(&self) -> FooterView {
        match &self.mode {
            Mode::Insert { .. } => FooterView {
                led_colour: palette::VIOLET,
                lit: true,
                text: format!("{WRITE} {}", footer_text(self.save_to.as_deref())),
                cursor: None,
            },
            Mode::NamePrompt { name, .. } => FooterView {
                led_colour: palette::LIME,
                lit: false,
                text: format!("{MOVE} {}", name_prompt_text(name)),
                cursor: Some(name.chars().count()),
            },
            Mode::Command => FooterView {
                led_colour: palette::LIME,
                lit: false,
                text: format!("{MOVE} {}", footer_text(self.save_to.as_deref())),
                cursor: None,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FooterView {
    pub(crate) led_colour: u8,
    pub(crate) lit: bool,
    pub(crate) text: String,
    pub(crate) cursor: Option<usize>,
}

fn footer_text(save_to: Option<&str>) -> String {
    let name = save_to.map_or(NO_NAME, file_stem_without_dre);
    format!("{name}{FOOTER_SUFFIX}")
}

fn name_prompt_text(name: &str) -> String {
    if name.is_empty() {
        format!("{PLACEHOLDER}{FOOTER_SUFFIX}")
    } else {
        format!("{name}{FOOTER_SUFFIX}")
    }
}

fn file_stem_without_dre(path: &str) -> &str {
    let file_name = path.rsplit('/').next().unwrap_or(path);
    file_name.strip_suffix(".dre").unwrap_or(file_name)
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
        }
    }
}

fn apply(state: State, action: action::Action) -> State {
    history::recorded(state, &action, |state| match action.spec().mode {
        ActionMode::Insert => insert::reduce(state, action),
        ActionMode::NamePrompt => name_prompt::reduce(state, action),
        ActionMode::Command => command::reduce(state, action),
    })
}

pub fn reduce(state: State, key: Option<&str>) -> (State, Vec<Effect>) {
    let action = key.and_then(|key| input::parse(&state, key));
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
    use crate::state::action::Action;
    use crate::state::apply as reduce;
    use crate::state::text_edit::TextKey;
    use crate::test_support::handle_key;

    fn footer_of(path: &str) -> String {
        State::open(Document::default(), Some(path.to_string()))
            .footer()
            .text
    }

    fn footer_for(name: &str) -> String {
        format!("{MOVE} {name}{FOOTER_SUFFIX}")
    }

    #[test]
    fn footer_strips_the_folder_and_the_dre_extension() {
        assert_eq!(footer_of("docs/plans.dre"), footer_for("plans"));
    }

    #[test]
    fn footer_strips_the_dre_extension_of_a_bare_file_name() {
        assert_eq!(footer_of("plans.dre"), footer_for("plans"));
    }

    #[test]
    fn footer_of_a_path_without_the_dre_extension_only_strips_the_folder() {
        assert_eq!(footer_of("docs/plans"), footer_for("plans"));
    }

    #[test]
    fn footer_of_a_default_state_has_no_name() {
        assert_eq!(State::default().footer().text, footer_for(NO_NAME));
    }

    #[test]
    fn footer_of_a_default_state_pins_the_hint_text() {
        assert_eq!(
            State::default().footer().text,
            "MOVE [no name — press n to name it] • dre"
        );
    }

    #[test]
    fn footer_of_a_new_file_is_named_after_its_path() {
        assert_eq!(
            State::new_file("docs/plans.dre".to_string()).footer().text,
            footer_for("plans")
        );
    }

    #[test]
    fn set_save_to_none_after_a_path_goes_back_to_no_name() {
        let mut state = State::open(Document::default(), Some("plans.dre".to_string()));
        state.set_save_to(None);
        assert_eq!(state.footer().text, footer_for(NO_NAME));
        assert_eq!(state.save_to(), None);
    }

    #[test]
    fn set_save_to_a_path_after_none_updates_the_footer() {
        let mut state = State::default();
        state.set_save_to(Some("docs/plans.dre".to_string()));
        assert_eq!(state.footer().text, footer_for("plans"));
        assert_eq!(state.save_to(), Some("docs/plans.dre"));
    }

    #[test]
    fn command_mode_footer_shows_a_dim_lime_led_and_no_cursor() {
        let state = new_state(vec![], Mode::Command, None);
        let footer = state.footer();
        assert_eq!(footer.led_colour, palette::LIME);
        assert!(!footer.lit);
        assert_eq!(footer.cursor, None);
        assert!(footer.text.starts_with(MOVE));
    }

    #[test]
    fn insert_mode_footer_shows_a_lit_violet_led_and_no_cursor() {
        let state = new_state(vec![node("a")], Mode::Insert { cursor: 0 }, Some(vec![0]));
        let footer = state.footer();
        assert_eq!(footer.led_colour, palette::VIOLET);
        assert!(footer.lit);
        assert_eq!(footer.cursor, None);
        assert!(footer.text.starts_with(WRITE));
    }

    #[test]
    fn insert_mode_footer_text_is_prefixed_with_write() {
        let mut state = new_state(vec![], Mode::Insert { cursor: 0 }, None);
        state.set_save_to(Some("docs/plans.dre".to_string()));
        assert_eq!(state.footer().text, format!("{WRITE} plans{FOOTER_SUFFIX}"));
    }

    #[test]
    fn name_prompt_footer_shows_the_same_dim_lime_led_as_command_mode() {
        let state = new_state(
            vec![],
            Mode::NamePrompt {
                name: "ab".to_string(),
                quits: false,
            },
            None,
        );
        let footer = state.footer();
        assert_eq!(footer.led_colour, palette::LIME);
        assert!(!footer.lit);
        assert!(footer.text.starts_with(MOVE));
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
        assert_eq!(
            state.footer().text,
            format!("{MOVE} {PLACEHOLDER}{FOOTER_SUFFIX}")
        );
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
            reduce(before, Action::EditLabel),
            Action::InsertKey(TextKey::Char('b')),
        );
        let result = reduce(typed, Action::CommitAndAddChild);
        assert_eq!(result.history.len(), 2);
    }

    #[test]
    fn commit_and_add_child_after_an_edit_needs_two_undos_to_restore_the_document() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let typed = reduce(
            reduce(before.clone(), Action::EditLabel),
            Action::InsertKey(TextKey::Char('b')),
        );
        let committed = reduce(typed, Action::CommitAndAddChild);
        let once = reduce(committed, Action::Undo);
        let twice = reduce(once.clone(), Action::Undo);
        assert_ne!(*once.doc.tree(), *before.doc.tree());
        assert_eq!(*twice.doc.tree(), *before.doc.tree());
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
        assert_eq!(*once.doc.tree(), *before.doc.tree());
    }

    #[test]
    fn rename_label_leaves_one_snapshot_and_needs_one_undo_to_restore_the_document() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let renaming = reduce(before.clone(), Action::RenameLabel);
        assert_eq!(renaming.history.len(), 1);
        let once = reduce(renaming, Action::Undo);
        assert_eq!(*once.doc.tree(), *before.doc.tree());
    }

    #[test]
    fn committing_an_edit_label_without_a_change_leaves_one_snapshot() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let editing = reduce(before.clone(), Action::EditLabel);
        let committed = reduce(editing, Action::Commit);
        assert_eq!(committed.history.len(), before.history.len() + 1);
        let undone = reduce(committed, Action::Undo);
        assert_eq!(*undone.doc.tree(), *before.doc.tree());
    }

    #[test]
    fn undo_does_not_add_a_snapshot() {
        let before = selecting_first(vec![node("a")], Mode::Command);
        let edited = reduce(reduce(before, Action::NewBox), Action::Undo);
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
            let (_, effects) = crate::state::reduce(state, Some("r"));
            assert_eq!(effects, vec![Effect::Save]);
        }

        #[test]
        fn a_selection_move_returns_no_effects() {
            let state = saved_state(vec![node("a"), node("b")], Some(vec![0]));
            let (_, effects) = crate::state::reduce(state, Some("j"));
            assert_eq!(effects, vec![]);
        }

        #[test]
        fn keys_typed_in_insert_mode_return_no_effects_until_escape_exits_it() {
            let state = saved_state(vec![node("a")], Some(vec![0]));
            let (state, effects) = crate::state::reduce(state, Some("i"));
            assert_eq!(effects, vec![]);
            let (state, effects) = crate::state::reduce(state, Some("b"));
            assert_eq!(effects, vec![]);
            let (state, effects) = crate::state::reduce(state, Some("c"));
            assert_eq!(effects, vec![]);
            let (_, effects) = crate::state::reduce(state, Some("\x1b"));
            assert_eq!(effects, vec![Effect::Save]);
        }

        #[test]
        fn commit_and_add_child_stays_in_insert_and_only_the_following_escape_saves_once() {
            let state = saved_state(vec![node("a")], Some(vec![0]));
            let (state, effects) = crate::state::reduce(state, Some("i"));
            assert_eq!(effects, vec![]);
            let (state, effects) = crate::state::reduce(state, Some("b"));
            assert_eq!(effects, vec![]);
            let (state, effects) = crate::state::reduce(state, Some("\r"));
            assert_eq!(effects, vec![]);
            assert!(matches!(state.mode(), Mode::Insert { .. }));
            let (state, effects) = crate::state::reduce(state, Some("c"));
            assert_eq!(effects, vec![]);
            let (_, effects) = crate::state::reduce(state, Some("\x1b"));
            assert_eq!(effects, vec![Effect::Save]);
        }

        #[test]
        fn undo_returns_a_save_effect() {
            let state = saved_state(vec![node("a")], Some(vec![0]));
            let (state, effects) = crate::state::reduce(state, Some("r"));
            assert_eq!(effects, vec![Effect::Save]);
            let (_, effects) = crate::state::reduce(state, Some("u"));
            assert_eq!(effects, vec![Effect::Save]);
        }

        #[test]
        fn a_no_op_edit_still_leaves_a_snapshot_and_returns_a_save_effect() {
            let state = saved_state(vec![node("a")], Some(vec![0]));
            let (state, effects) = crate::state::reduce(state, Some("i"));
            assert_eq!(effects, vec![]);
            let (state, effects) = crate::state::reduce(state, Some("\x1b"));
            assert_eq!(effects, vec![Effect::Save]);
            assert_eq!(state.history.len(), 1);
        }

        #[test]
        fn a_second_change_after_a_save_returns_a_save_effect_again() {
            let state = saved_state(vec![node("a")], Some(vec![0]));
            let (state, effects) = crate::state::reduce(state, Some("r"));
            assert_eq!(effects, vec![Effect::Save]);
            let (_, effects) = crate::state::reduce(state, Some("r"));
            assert_eq!(effects, vec![Effect::Save]);
        }

        #[test]
        fn with_no_save_path_no_effects_are_ever_returned() {
            let state = new_state(vec![node("a")], Mode::Command, Some(vec![0]));
            assert_eq!(state.save_to(), None);
            let (state, effects) = crate::state::reduce(state, Some("r"));
            assert_eq!(effects, vec![]);
            let (state, effects) = crate::state::reduce(state, Some("i"));
            assert_eq!(effects, vec![]);
            let (state, effects) = crate::state::reduce(state, Some("\x1b"));
            assert_eq!(effects, vec![]);
            let (_, effects) = crate::state::reduce(state, Some("u"));
            assert_eq!(effects, vec![]);
        }

        #[test]
        fn an_interrupt_after_a_change_returns_no_effects() {
            let state = saved_state(vec![node("a")], Some(vec![0]));
            let (state, effects) = crate::state::reduce(state, Some("r"));
            assert_eq!(effects, vec![Effect::Save]);
            let (_, effects) = crate::state::reduce(state, Some(INTERRUPT));
            assert_eq!(effects, vec![]);
        }
    }
}
