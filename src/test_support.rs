use crate::state::{reduce, State};

pub(crate) fn handle_key(state: State, key: &str) -> State {
    reduce(state, Some(key))
}
