use std::cell::Cell;
use std::io;

use crate::editor::store::StateStore;
use crate::state::{self, Mode, State};

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

pub(crate) struct AutosavingReducer {
    inner: Box<dyn Reducer>,
    store: Box<dyn StateStore>,
    saved_len: Cell<usize>,
}

impl AutosavingReducer {
    pub(crate) fn new(inner: Box<dyn Reducer>, store: Box<dyn StateStore>) -> Self {
        Self {
            inner,
            store,
            saved_len: Cell::new(0),
        }
    }
}

impl Reducer for AutosavingReducer {
    fn reduce(&self, state: State, key: Option<&str>) -> io::Result<State> {
        let state = self.inner.reduce(state, key)?;
        let changed = state.history_len() != self.saved_len.get();
        if state.save_to().is_some() && state.mode != Mode::Insert && changed {
            self.store.save(&state)?;
            self.saved_len.set(state.history_len());
        }
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{node, Document};
    use crate::editor::store::MockStateStore;
    use crate::state::INTERRUPT;

    const ESC: &str = "\x1b";
    const ENTER: &str = "\r";

    fn named_state() -> State {
        State::open(
            Document::with_boxes(vec![node("a"), node("b")]),
            Some("diagram.dre".to_string()),
        )
    }

    fn store_saving(times: usize) -> MockStateStore {
        let mut store = MockStateStore::new();
        store.expect_save().times(times).returning(|_| Ok(()));
        store
    }

    fn autosaving(store: MockStateStore) -> AutosavingReducer {
        AutosavingReducer::new(Box::new(StateReducer), Box::new(store))
    }

    fn reduce_all(reducer: &AutosavingReducer, mut state: State, keys: &[&str]) -> State {
        for key in keys {
            state = reducer.reduce(state, Some(key)).unwrap();
        }
        state
    }

    #[test]
    fn a_key_that_changes_the_history_length_saves_once() {
        let reducer = autosaving(store_saving(1));
        reduce_all(&reducer, named_state(), &["c"]);
    }

    #[test]
    fn a_selection_move_does_not_save() {
        let reducer = autosaving(store_saving(0));
        reduce_all(&reducer, named_state(), &["j", "k"]);
    }

    #[test]
    fn keys_typed_in_insert_do_not_save_and_leaving_insert_saves_once() {
        let reducer = autosaving(store_saving(1));
        let typing = reduce_all(&reducer, named_state(), &["i", "x", "y"]);
        assert_eq!(typing.mode, Mode::Insert);
        let left = reduce_all(&reducer, typing, &[ESC]);
        assert_eq!(left.mode, Mode::Command);
    }

    #[test]
    fn enter_starting_the_next_child_does_not_save_and_the_following_esc_saves_once() {
        let reducer = autosaving(store_saving(1));
        let next_child = reduce_all(&reducer, named_state(), &["i", "x", ENTER, "y"]);
        assert_eq!(next_child.mode, Mode::Insert);
        reduce_all(&reducer, next_child, &[ESC]);
    }

    #[test]
    fn without_a_file_to_save_to_nothing_is_saved() {
        let reducer = autosaving(store_saving(0));
        let mut state = named_state();
        state.set_save_to(None);
        reduce_all(&reducer, state, &["c"]);
    }

    #[test]
    fn a_failed_save_makes_reduce_return_that_error() {
        let mut store = MockStateStore::new();
        store
            .expect_save()
            .returning(|_| Err(io::Error::new(io::ErrorKind::PermissionDenied, "read-only")));
        let reducer = autosaving(store);
        let error = reducer.reduce(named_state(), Some("c")).err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn ctrl_c_after_a_change_leaves_no_file_to_save_to_and_does_not_save_again() {
        let reducer = autosaving(store_saving(1));
        let changed = reduce_all(&reducer, named_state(), &["c"]);
        let interrupted = reduce_all(&reducer, changed, &[INTERRUPT]);
        assert_eq!(interrupted.save_to(), None);
    }
}
