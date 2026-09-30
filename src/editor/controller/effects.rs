use std::io;

use crate::editor::store::StateStore;
use crate::state::{Effect, State};

#[cfg_attr(test, mockall::automock)]
pub(crate) trait EffectExecutor {
    fn execute(&self, effects: Vec<Effect>, state: &State) -> io::Result<()>;
}

pub(crate) struct StoreEffectExecutor {
    store: Box<dyn StateStore>,
}

impl StoreEffectExecutor {
    pub(crate) fn new(store: Box<dyn StateStore>) -> Self {
        Self { store }
    }
}

impl EffectExecutor for StoreEffectExecutor {
    fn execute(&self, effects: Vec<Effect>, state: &State) -> io::Result<()> {
        for effect in effects {
            match effect {
                Effect::Save => self.store.save(state)?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::store::MockStateStore;

    fn executor(store: MockStateStore) -> StoreEffectExecutor {
        StoreEffectExecutor::new(Box::new(store))
    }

    fn marked(count: usize) -> State {
        State::default().with_pending_count(count)
    }

    #[test]
    fn a_save_effect_saves_the_given_state_once() {
        let mut store = MockStateStore::new();
        store
            .expect_save()
            .withf(|s| s.pending_count() == Some(3))
            .times(1)
            .returning(|_| Ok(()));

        executor(store)
            .execute(vec![Effect::Save], &marked(3))
            .unwrap();
    }

    #[test]
    fn no_effects_never_saves() {
        let mut store = MockStateStore::new();
        store.expect_save().never();

        executor(store).execute(vec![], &State::default()).unwrap();
    }

    #[test]
    fn a_save_failure_propagates() {
        let mut store = MockStateStore::new();
        store
            .expect_save()
            .times(1)
            .returning(|_| Err(io::Error::other("save failed")));

        let error = executor(store)
            .execute(vec![Effect::Save], &State::default())
            .err()
            .unwrap();

        assert_eq!(error.to_string(), "save failed");
    }
}
