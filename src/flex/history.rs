use super::state::{FlexBox, FlexEffect, FlexMode, FlexState};
use types::Tree;

#[derive(Debug, Clone)]
pub(crate) struct Snapshot {
    boxes: Tree<FlexBox>,
    selected: Vec<usize>,
}

pub(super) fn undoable(mode: FlexMode, key: &str) -> bool {
    matches!(
        (mode, key),
        (
            FlexMode::Move,
            "a" | "A" | "o" | "i" | "s" | "r" | "f" | "]"
        ) | (FlexMode::Write, "\r")
    )
}

pub(super) fn recorded(
    mut state: FlexState,
    key: &str,
    reduce: impl FnOnce(FlexState) -> (FlexState, Option<FlexEffect>),
) -> (FlexState, Option<FlexEffect>) {
    if undoable(state.mode, key) {
        state.history.push(Snapshot {
            boxes: state.boxes.clone(),
            selected: state.selected.clone(),
        });
    }
    reduce(state)
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn record(state: &mut FlexState) {
    state.history.push(Snapshot {
        boxes: state.boxes.clone(),
        selected: state.selected.clone(),
    });
}

pub(super) fn undo(mut state: FlexState) -> FlexState {
    if let Some(previous) = state.history.pop() {
        state.boxes = previous.boxes;
        state.selected = previous.selected;
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_text_and_toggle_keys_are_undoable_in_move_mode() {
        for key in ["a", "A", "o", "i", "s", "r", "f", "]"] {
            assert!(undoable(FlexMode::Move, key), "{key}");
        }
    }

    #[test]
    fn selection_undo_and_quit_keys_are_not_undoable_in_move_mode() {
        for key in ["h", "j", "k", "l", "u", "q", "w", "d", "y", "p"] {
            assert!(!undoable(FlexMode::Move, key), "{key}");
        }
    }

    #[test]
    fn no_key_other_than_enter_is_undoable_in_write_mode() {
        for key in [
            "a", "A", "o", "s", "i", "w", "g", "d", "r", "f", "p", "]", "y", "u", "h", "q", "\x7f",
        ] {
            assert!(!undoable(FlexMode::Write, key), "{key}");
        }
    }

    #[test]
    fn enter_is_undoable_in_write_mode() {
        assert!(undoable(FlexMode::Write, "\r"));
    }

    #[test]
    fn enter_is_not_undoable_in_move_mode() {
        assert!(!undoable(FlexMode::Move, "\r"));
    }

    #[test]
    fn recorded_pushes_a_snapshot_for_enter_in_write_mode_regardless_of_text() {
        for text in [Some(String::new()), Some("Hello".to_string())] {
            let mut state = FlexState {
                mode: FlexMode::Write,
                ..FlexState::default()
            };
            state.boxes.value_mut(&state.selected).text = text.clone();
            let before_len = state.history.len();
            let (state, _) = recorded(state, "\r", |s| (s, None));
            assert_eq!(state.history.len(), before_len + 1, "{text:?}");
        }
    }

    #[test]
    fn record_pushes_a_snapshot_that_undo_can_restore() {
        let before = FlexState {
            mode: FlexMode::Write,
            ..FlexState::default()
        };
        let before_len = before.history.len();

        let mut state = before.clone();
        record(&mut state);
        assert_eq!(state.history.len(), before_len + 1);

        state.boxes.value_mut(&state.selected).text = Some("changed".to_string());
        state.selected = vec![];

        let undone = undo(state);
        assert_eq!(undone.boxes, before.boxes);
        assert_eq!(undone.selected, before.selected);
    }

    #[test]
    fn undo_with_an_empty_history_leaves_the_state_unchanged() {
        let state = FlexState::default();
        let undone = undo(state.clone());
        assert_eq!(undone, state);
        assert!(undone.history.is_empty());
    }
}
