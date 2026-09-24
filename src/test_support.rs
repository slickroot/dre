use crate::reduce::reduce;
use crate::state::input::parse;
use crate::state::State;

pub(crate) fn handle_key(state: State, key: &str) -> State {
    match parse(&state, key) {
        Some(action) => reduce(state, action),
        None => state,
    }
}
