use std::io;

use crate::state::{self, State};

#[cfg_attr(test, mockall::automock)]
pub(crate) trait Reducer {
    // automock needs the lifetime named: it cannot mock an elided reference inside Option.
    #[allow(clippy::needless_lifetimes)]
    fn reduce<'a>(&self, state: State, key: Option<&'a str>) -> io::Result<State>;
}

pub(crate) struct StateReducer;

impl Reducer for StateReducer {
    fn reduce(&self, state: State, key: Option<&str>) -> io::Result<State> {
        Ok(state::reduce(state, key))
    }
}
