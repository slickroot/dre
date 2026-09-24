use crate::action::{Action, ActionMode};
use crate::command_mode;
use crate::insert_mode;
use crate::save_prompt_mode;
use crate::state::State;

pub(crate) fn reduce(mut state: State, action: Action) -> State {
    if let Some(selected) = state.last_selected.take() {
        state.doc.selected = Some(selected);
    }
    match action.mode() {
        ActionMode::Insert => insert_mode::reduce(state, action),
        ActionMode::SavePrompt => save_prompt_mode::reduce(state, action),
        ActionMode::Command => command_mode::reduce(state, action),
    }
}
