use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[cfg_attr(test, mockall::automock)]
pub(crate) trait Closer: Send + Sync {
    fn close(&self);
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Token(u64);

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Default)]
struct Registry {
    next_token: u64,
    live: HashMap<String, (Token, Arc<dyn Closer>)>,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Default)]
pub(crate) struct Sessions {
    registry: Arc<Mutex<Registry>>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl Sessions {
    pub(crate) fn take_over(&self, fingerprint: &str) {
        let previous = self.registry.lock().unwrap().live.remove(fingerprint);
        if let Some((_, closer)) = previous {
            closer.close();
        }
    }

    pub(crate) fn register(&self, fingerprint: &str, closer: Arc<dyn Closer>) -> Token {
        let mut registry = self.registry.lock().unwrap();
        let token = Token(registry.next_token);
        registry.next_token += 1;
        registry
            .live
            .insert(fingerprint.to_string(), (token, closer));
        token
    }

    pub(crate) fn unregister(&self, fingerprint: &str, token: Token) {
        let mut registry = self.registry.lock().unwrap();
        if registry
            .live
            .get(fingerprint)
            .is_some_and(|(current, _)| *current == token)
        {
            registry.live.remove(fingerprint);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn closer_expecting(closes: usize) -> Arc<dyn Closer> {
        let mut closer = MockCloser::new();
        closer.expect_close().times(closes).return_const(());
        Arc::new(closer)
    }

    #[test]
    fn take_over_closes_the_previous_connection() {
        let sessions = Sessions::default();
        sessions.register("key", closer_expecting(1));
        sessions.take_over("key");
    }

    #[test]
    fn take_over_closes_a_connection_only_once() {
        let sessions = Sessions::default();
        sessions.register("key", closer_expecting(1));
        sessions.take_over("key");
        sessions.take_over("key");
    }

    #[test]
    fn take_over_on_an_unknown_key_does_nothing() {
        Sessions::default().take_over("key");
    }

    #[test]
    fn different_keys_never_close_each_other() {
        let sessions = Sessions::default();
        sessions.register("alice", closer_expecting(0));
        sessions.register("bob", closer_expecting(1));
        sessions.take_over("bob");
    }

    #[test]
    fn a_stale_unregister_is_ignored_and_the_replacement_is_still_closed() {
        let sessions = Sessions::default();
        let old = sessions.register("key", closer_expecting(1));
        sessions.take_over("key");
        sessions.register("key", closer_expecting(1));
        sessions.unregister("key", old);
        sessions.take_over("key");
    }

    #[test]
    fn unregister_of_the_current_connection_removes_it() {
        let sessions = Sessions::default();
        let token = sessions.register("key", closer_expecting(0));
        sessions.unregister("key", token);
        sessions.take_over("key");
    }

    #[test]
    fn clones_share_the_same_registry() {
        let sessions = Sessions::default();
        sessions.register("key", closer_expecting(1));
        sessions.clone().take_over("key");
    }
}
