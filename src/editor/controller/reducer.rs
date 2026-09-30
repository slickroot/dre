use crate::state::{self, Effect, State};

#[cfg_attr(test, mockall::automock)]
pub(crate) trait Reducer {
    fn reduce(&self, state: State, key: &str) -> (State, Vec<Effect>);
}

pub(crate) struct StateReducer;

impl Reducer for StateReducer {
    fn reduce(&self, state: State, key: &str) -> (State, Vec<Effect>) {
        state::reduce(state, key)
    }
}
