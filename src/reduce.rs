use crate::action::Action;
use crate::command_mode;
use crate::insert_mode;
use crate::save_prompt_mode;
use crate::state::State;

pub(crate) fn reduce(mut state: State, action: Action) -> State {
    if let Some(selected) = state.last_selected.take() {
        state.doc.selected = Some(selected);
    }
    match action {
        Action::Command(command) => command_mode::reduce(state, command),
        Action::Insert(command) => insert_mode::reduce(state, command),
        Action::SavePrompt(command) => save_prompt_mode::reduce(state, command),
    }
}
