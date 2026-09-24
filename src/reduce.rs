use crate::state::action::{Action, ActionMode};
use crate::state::command;
use crate::state::insert;
use crate::state::save_prompt;
use crate::state::State;

pub(crate) fn reduce(mut state: State, action: Action) -> State {
    if let Some(selected) = state.last_selected.take() {
        state.doc.selected = Some(selected);
    }
    match action.mode() {
        ActionMode::Insert => insert::reduce(state, action),
        ActionMode::SavePrompt => save_prompt::reduce(state, action),
        ActionMode::Command => command::reduce(state, action),
    }
}
