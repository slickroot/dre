use std::io;

use crate::state::State;

#[cfg_attr(test, mockall::automock)]
pub(crate) trait KeySource {
    fn next_key(&mut self) -> io::Result<Option<String>>;
}

#[cfg_attr(test, mockall::automock)]
pub(crate) trait Screen {
    fn render(&mut self, state: &State) -> io::Result<()>;
    fn resize(&mut self) -> io::Result<()>;
}

#[cfg_attr(test, mockall::automock)]
pub(crate) trait Reducer {
    // automock needs the lifetime named: it cannot mock an elided reference inside Option.
    #[allow(clippy::needless_lifetimes)]
    fn reduce<'a>(&self, state: State, key: Option<&'a str>) -> State;
}

pub(crate) struct DreController {
    keys: Box<dyn KeySource>,
    screen: Box<dyn Screen>,
    reducer: Box<dyn Reducer>,
}

impl DreController {
    pub(crate) fn new(
        keys: Box<dyn KeySource>,
        screen: Box<dyn Screen>,
        reducer: Box<dyn Reducer>,
    ) -> Self {
        Self {
            keys,
            screen,
            reducer,
        }
    }

    pub(crate) fn run(&mut self, state: State) -> io::Result<State> {
        let mut state = state;
        while state.running {
            self.screen.render(&state)?;
            match self.keys.next_key()? {
                Some(key) if key == crate::terminal::RESIZE => self.screen.resize()?,
                key => state = self.reducer.reduce(state, key.as_deref()),
            }
        }
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::RESIZE;
    use mockall::Sequence;

    fn controller(keys: MockKeySource, screen: MockScreen, reducer: MockReducer) -> DreController {
        DreController::new(Box::new(keys), Box::new(screen), Box::new(reducer))
    }

    fn keys_reading(keys: Vec<Option<&str>>) -> MockKeySource {
        let mut source = MockKeySource::new();
        let mut seq = Sequence::new();
        for key in keys {
            let key = key.map(str::to_string);
            source
                .expect_next_key()
                .times(1)
                .in_sequence(&mut seq)
                .return_once(move || Ok(key));
        }
        source
    }

    fn any_screen() -> MockScreen {
        let mut screen = MockScreen::new();
        screen.expect_render().returning(|_| Ok(()));
        screen
    }

    fn stopped(mut state: State) -> State {
        state.running = false;
        state
    }

    fn reducer_stopping_on(key: &'static str) -> MockReducer {
        let mut reducer = MockReducer::new();
        reducer
            .expect_reduce()
            .withf(move |_, k| *k == Some(key))
            .times(1)
            .returning(|state, _| stopped(state));
        reducer
    }

    fn marked(count: usize) -> State {
        let mut state = State::default();
        state.pending_count = Some(count);
        state
    }

    #[test]
    fn a_frame_is_rendered_before_the_first_key_is_read() {
        let mut seq = Sequence::new();
        let mut screen = MockScreen::new();
        screen
            .expect_render()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        let mut keys = MockKeySource::new();
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok(Some("q".to_string())));

        controller(keys, screen, reducer_stopping_on("q"))
            .run(State::default())
            .unwrap();
    }

    #[test]
    fn a_key_read_from_the_key_source_is_passed_to_reduce() {
        controller(
            keys_reading(vec![Some("x")]),
            any_screen(),
            reducer_stopping_on("x"),
        )
        .run(State::default())
        .unwrap();
    }

    #[test]
    fn an_idle_none_is_passed_to_reduce_as_none() {
        let mut reducer = MockReducer::new();
        reducer
            .expect_reduce()
            .withf(|_, key| key.is_none())
            .times(1)
            .returning(|state, _| stopped(state));

        controller(keys_reading(vec![None]), any_screen(), reducer)
            .run(State::default())
            .unwrap();
    }

    #[test]
    fn the_state_reduce_returns_is_the_one_rendered_next() {
        let mut seq = Sequence::new();
        let mut screen = MockScreen::new();
        screen
            .expect_render()
            .withf(|state| state.pending_count == Some(1))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| state.pending_count == Some(2))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        let mut reducer = MockReducer::new();
        reducer
            .expect_reduce()
            .withf(|_, key| *key == Some("a"))
            .times(1)
            .returning(|_, _| marked(2));
        reducer
            .expect_reduce()
            .withf(|_, key| *key == Some("q"))
            .times(1)
            .returning(|state, _| stopped(state));

        controller(keys_reading(vec![Some("a"), Some("q")]), screen, reducer)
            .run(marked(1))
            .unwrap();
    }

    #[test]
    fn the_loop_stops_when_reduce_returns_a_stopped_state_and_run_returns_it() {
        let mut reducer = MockReducer::new();
        reducer
            .expect_reduce()
            .times(1)
            .returning(|_, _| stopped(marked(9)));
        let mut screen = MockScreen::new();
        screen.expect_render().times(1).returning(|_| Ok(()));

        let state = controller(keys_reading(vec![Some("q")]), screen, reducer)
            .run(State::default())
            .unwrap();

        assert!(!state.running);
        assert_eq!(state.pending_count, Some(9));
    }

    #[test]
    fn a_resize_key_resizes_the_screen_once_and_never_reaches_reduce() {
        let mut screen = any_screen();
        screen.expect_resize().times(1).returning(|| Ok(()));

        controller(
            keys_reading(vec![Some(RESIZE), Some("q")]),
            screen,
            reducer_stopping_on("q"),
        )
        .run(State::default())
        .unwrap();
    }

    #[test]
    fn a_failed_resize_propagates() {
        let mut screen = any_screen();
        screen
            .expect_resize()
            .times(1)
            .returning(|| Err(io::Error::other("resize failed")));
        let mut reducer = MockReducer::new();
        reducer.expect_reduce().never();

        let error = controller(keys_reading(vec![Some(RESIZE)]), screen, reducer)
            .run(State::default())
            .err()
            .unwrap();

        assert_eq!(error.to_string(), "resize failed");
    }

    #[test]
    fn a_failed_next_key_propagates() {
        let mut keys = MockKeySource::new();
        keys.expect_next_key()
            .times(1)
            .returning(|| Err(io::Error::other("keys failed")));
        let mut reducer = MockReducer::new();
        reducer.expect_reduce().never();

        let error = controller(keys, any_screen(), reducer)
            .run(State::default())
            .err()
            .unwrap();

        assert_eq!(error.to_string(), "keys failed");
    }

    #[test]
    fn a_failed_render_propagates_and_no_key_is_read() {
        let mut screen = MockScreen::new();
        screen
            .expect_render()
            .times(1)
            .returning(|_| Err(io::Error::other("render failed")));
        let mut keys = MockKeySource::new();
        keys.expect_next_key().never();

        let error = controller(keys, screen, MockReducer::new())
            .run(State::default())
            .err()
            .unwrap();

        assert_eq!(error.to_string(), "render failed");
    }

    #[test]
    fn a_state_that_is_not_running_renders_nothing_reads_no_key_and_is_returned_as_given() {
        let mut screen = MockScreen::new();
        screen.expect_render().never();
        let mut keys = MockKeySource::new();
        keys.expect_next_key().never();
        let mut reducer = MockReducer::new();
        reducer.expect_reduce().never();

        let state = controller(keys, screen, reducer)
            .run(stopped(marked(7)))
            .unwrap();

        assert_eq!(state.pending_count, Some(7));
    }
}
