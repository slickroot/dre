use super::state::{FlexBox, FlexEffect, FlexMode, FlexState};
use types::Tree;

#[derive(Debug, Clone)]
pub(crate) struct Snapshot {
    boxes: Tree<FlexBox>,
    selected: Vec<usize>,
}

pub(super) fn undoable(mode: FlexMode, key: &str) -> bool {
    mode == FlexMode::Move && matches!(key, "a" | "A" | "o" | "i" | "s" | "r" | "f" | "]")
}

pub(super) fn recorded(
    mut state: FlexState,
    key: &str,
    reduce: impl FnOnce(FlexState) -> (FlexState, Option<FlexEffect>),
) -> (FlexState, Option<FlexEffect>) {
    if undoable(state.mode, key) {
        record(&mut state);
    }
    reduce(state)
}

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
    fn no_key_is_undoable_in_write_mode() {
        for key in [
            "a", "A", "o", "s", "i", "w", "g", "d", "r", "f", "p", "]", "y", "u", "h", "q", "\r",
            "\x7f",
        ] {
            assert!(!undoable(FlexMode::Write, key), "{key}");
        }
    }

    #[test]
    fn record_snapshots_the_state_so_undo_restores_it() {
        let original = FlexState::default();
        let mut state = original.clone();
        record(&mut state);
        state.selected = vec![0];
        let undone = undo(state);
        assert_eq!(undone, original);
    }

    #[test]
    fn undo_with_an_empty_history_leaves_the_state_unchanged() {
        let state = FlexState::default();
        let undone = undo(state.clone());
        assert_eq!(undone, state);
        assert!(undone.history.is_empty());
    }
}
