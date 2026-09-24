use crate::state::{self, State};

#[cfg_attr(test, mockall::automock)]
pub(crate) trait Reducer {
    // automock needs the lifetime named: it cannot mock an elided reference inside Option.
    #[allow(clippy::needless_lifetimes)]
    fn reduce<'a>(&self, state: State, key: Option<&'a str>) -> State;
}

pub(crate) struct StateReducer;

impl Reducer for StateReducer {
    fn reduce(&self, state: State, key: Option<&str>) -> State {
        state::reduce(state, key)
    }
}
