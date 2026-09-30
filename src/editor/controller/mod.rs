pub(crate) mod effects;
pub(crate) mod reducer;
pub(crate) mod screen;

use std::io;
use std::thread;
use std::time::{Duration, Instant};

use crate::key_source::KeySource;
use crate::state::{self, State};
use crate::tty;
use effects::EffectExecutor;
use reducer::Reducer;
use screen::Screen;

#[cfg_attr(test, mockall::automock)]
pub(crate) trait Controller {
    fn run(&mut self, state: State) -> io::Result<State>;
}

pub(crate) struct DreController {
    keys: Box<dyn KeySource>,
    screen: Box<dyn Screen>,
    reducer: Box<dyn Reducer>,
    executor: Box<dyn EffectExecutor>,
    flash_time: u64,
}

impl DreController {
    pub(crate) fn new(
        keys: Box<dyn KeySource>,
        screen: Box<dyn Screen>,
        reducer: Box<dyn Reducer>,
        executor: Box<dyn EffectExecutor>,
    ) -> Self {
        Self {
            keys,
            screen,
            reducer,
            executor,
            flash_time: 20,
        }
    }
}

impl Controller for DreController {
    fn run(&mut self, state: State) -> io::Result<State> {
        let mut state = state;
        while state.is_running() {
            self.screen.render(&state)?;
            let key = self.keys.next_key()?;
            if key == tty::RESIZE {
                self.screen.resize()?;
            } else {
                self.screen.render(&state::flash(state.clone()))?;
                thread::sleep(Duration::from_millis(self.flash_time));
                let label = state::command_label(&state, &key);
                let start = label.is_some().then(Instant::now);
                let (next, effects) = self.reducer.reduce(state, &key);
                self.executor.execute(effects, &next)?;
                state = match (label, start) {
                    (Some((label_key, name)), Some(start)) => {
                        self.screen.render(&next)?;
                        let elapsed = start.elapsed();
                        state::set_command_status(next, label_key, name, elapsed)
                    }
                    _ => next,
                };
            }
        }
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::effects::MockEffectExecutor;
    use super::reducer::MockReducer;
    use super::screen::MockScreen;
    use super::*;
    use crate::key_source::MockKeySource;
    use crate::state::{new_state, CommandStatus, Effect, Mode};
    use crate::tty::RESIZE;
    use mockall::Sequence;

    fn controller(
        keys: MockKeySource,
        screen: MockScreen,
        reducer: MockReducer,
        executor: MockEffectExecutor,
    ) -> DreController {
        let mut controller = DreController::new(
            Box::new(keys),
            Box::new(screen),
            Box::new(reducer),
            Box::new(executor),
        );
        controller.flash_time = 0;
        controller
    }

    fn any_executor() -> MockEffectExecutor {
        let mut executor = MockEffectExecutor::new();
        executor.expect_execute().returning(|_, _| Ok(()));
        executor
    }

    fn keys_reading(keys: Vec<&str>) -> MockKeySource {
        let mut source = MockKeySource::new();
        let mut seq = Sequence::new();
        for key in keys {
            let key = key.to_string();
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

    fn stopped(state: State) -> State {
        state.with_running(false)
    }

    fn reducer_stopping_on(key: &'static str) -> MockReducer {
        let mut reducer = MockReducer::new();
        reducer
            .expect_reduce()
            .withf(move |_, k| k == key)
            .times(1)
            .returning(|state, _| (stopped(state), vec![]));
        reducer
    }

    fn marked(count: usize) -> State {
        State::default().with_pending_count(count)
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
            .returning(|| Ok("x".to_string()));
        screen
            .expect_render()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));

        controller(keys, screen, reducer_stopping_on("x"), any_executor())
            .run(State::default())
            .unwrap();
    }

    #[test]
    fn a_key_read_from_the_key_source_is_passed_to_reduce() {
        controller(
            keys_reading(vec!["x"]),
            any_screen(),
            reducer_stopping_on("x"),
            any_executor(),
        )
        .run(State::default())
        .unwrap();
    }

    #[test]
    fn the_state_reduce_returns_is_the_one_rendered_next() {
        let mut seq = Sequence::new();
        let mut screen = MockScreen::new();
        screen
            .expect_render()
            .withf(|state| state.pending_count() == Some(1) && !state.led_flash())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| state.pending_count() == Some(1) && state.led_flash())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| state.pending_count() == Some(2) && !state.led_flash())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| state.pending_count() == Some(2) && state.led_flash())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        let mut reducer = MockReducer::new();
        reducer
            .expect_reduce()
            .withf(|_, key| key == "a")
            .times(1)
            .returning(|_, _| (marked(2), vec![]));
        reducer
            .expect_reduce()
            .withf(|_, key| key == "x")
            .times(1)
            .returning(|state, _| (stopped(state), vec![]));

        controller(
            keys_reading(vec!["a", "x"]),
            screen,
            reducer,
            any_executor(),
        )
        .run(marked(1))
        .unwrap();
    }

    #[test]
    fn the_loop_stops_when_reduce_returns_a_stopped_state_and_run_returns_it() {
        let mut reducer = MockReducer::new();
        reducer
            .expect_reduce()
            .times(1)
            .returning(|_, _| (stopped(marked(9)), vec![]));
        let mut screen = MockScreen::new();
        screen.expect_render().times(2).returning(|_| Ok(()));

        let state = controller(keys_reading(vec!["x"]), screen, reducer, any_executor())
            .run(State::default())
            .unwrap();

        assert!(!state.is_running());
        assert_eq!(state.pending_count(), Some(9));
    }

    #[test]
    fn a_resize_key_resizes_the_screen_once_and_never_reaches_reduce() {
        let mut screen = any_screen();
        screen.expect_resize().times(1).returning(|| Ok(()));

        controller(
            keys_reading(vec![RESIZE, "q"]),
            screen,
            reducer_stopping_on("q"),
            any_executor(),
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

        let error = controller(keys_reading(vec![RESIZE]), screen, reducer, any_executor())
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

        let error = controller(keys, any_screen(), reducer, any_executor())
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

        let error = controller(keys, screen, MockReducer::new(), any_executor())
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

        let state = controller(keys, screen, reducer, any_executor())
            .run(stopped(marked(7)))
            .unwrap();

        assert_eq!(state.pending_count(), Some(7));
    }

    #[test]
    fn the_effects_returned_by_reduce_are_passed_to_execute_with_the_new_state() {
        let mut reducer = MockReducer::new();
        reducer
            .expect_reduce()
            .withf(|_, key| key == "s")
            .times(1)
            .returning(|_, _| (stopped(marked(4)), vec![Effect::Save]));
        let mut executor = MockEffectExecutor::new();
        executor
            .expect_execute()
            .withf(|effects, state| {
                *effects == vec![Effect::Save] && state.pending_count() == Some(4)
            })
            .times(1)
            .returning(|_, _| Ok(()));

        controller(keys_reading(vec!["s"]), any_screen(), reducer, executor)
            .run(State::default())
            .unwrap();
    }

    #[test]
    fn an_error_from_execute_stops_the_loop_and_is_returned() {
        let mut reducer = MockReducer::new();
        reducer
            .expect_reduce()
            .times(1)
            .returning(|state, _| (state, vec![Effect::Save]));
        let mut executor = MockEffectExecutor::new();
        executor
            .expect_execute()
            .times(1)
            .returning(|_, _| Err(io::Error::other("save failed")));
        let mut keys = MockKeySource::new();
        keys.expect_next_key()
            .times(1)
            .returning(|| Ok("s".to_string()));

        let error = controller(keys, any_screen(), reducer, executor)
            .run(State::default())
            .err()
            .unwrap();

        assert_eq!(error.to_string(), "save failed");
    }

    #[test]
    fn a_command_mode_keystroke_flashes_the_led_before_reducing() {
        let mut seq = Sequence::new();
        let mut screen = MockScreen::new();
        screen
            .expect_render()
            .withf(|state| !state.led_flash())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| state.led_flash())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| !state.led_flash())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| state.led_flash())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));

        let mut reducer = MockReducer::new();
        reducer
            .expect_reduce()
            .withf(|_, key| key == "a")
            .times(1)
            .returning(|state, _| (state, vec![]));
        reducer
            .expect_reduce()
            .withf(|_, key| key == "x")
            .times(1)
            .returning(|state, _| (stopped(state), vec![]));

        controller(
            keys_reading(vec!["a", "x"]),
            screen,
            reducer,
            any_executor(),
        )
        .run(State::default())
        .unwrap();
    }

    #[test]
    fn a_resize_key_in_command_mode_does_not_flash() {
        let mut seq = Sequence::new();
        let mut screen = MockScreen::new();
        screen.expect_resize().times(1).returning(|| Ok(()));
        screen
            .expect_render()
            .withf(|state| !state.led_flash())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| !state.led_flash())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| state.led_flash())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));

        controller(
            keys_reading(vec![RESIZE, "x"]),
            screen,
            reducer_stopping_on("x"),
            any_executor(),
        )
        .run(State::default())
        .unwrap();
    }

    fn assert_keystroke_flashes_before_reducing(mode: Mode) {
        let mut seq = Sequence::new();
        let mut screen = MockScreen::new();
        screen
            .expect_render()
            .withf(|state| !state.led_flash())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| state.led_flash())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        let mut reducer = MockReducer::new();
        reducer
            .expect_reduce()
            .withf(|state, key| key == "a" && !state.led_flash())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|state, _| (stopped(state), vec![]));
        screen
            .expect_render()
            .withf(|state| !state.led_flash())
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));

        controller(keys_reading(vec!["a"]), screen, reducer, any_executor())
            .run(new_state(vec![], mode, None))
            .unwrap();
    }

    #[test]
    fn an_insert_mode_keystroke_flashes_the_led_before_reducing() {
        assert_keystroke_flashes_before_reducing(Mode::Insert { cursor: 0 });
    }

    #[test]
    fn a_name_prompt_keystroke_flashes_the_led_before_reducing() {
        assert_keystroke_flashes_before_reducing(Mode::NamePrompt {
            name: String::new(),
            quits: false,
        });
    }

    fn has_command_status(state: &State, key: &str, name: &str) -> bool {
        matches!(
            state.command_status(),
            Some(CommandStatus { key: k, name: n, .. }) if k == key && n == name
        )
    }

    #[test]
    fn a_labeled_keystroke_renders_the_reduced_state_immediately_and_records_command_status() {
        let mut seq = Sequence::new();
        let mut screen = MockScreen::new();
        screen
            .expect_render()
            .withf(|state| state.command_status().is_none())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| state.led_flash() && state.command_status().is_none())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| !state.led_flash() && state.command_status().is_none())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| has_command_status(state, "b", "Add a child box"))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| state.led_flash() && has_command_status(state, "b", "Add a child box"))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));

        let mut reducer = MockReducer::new();
        reducer
            .expect_reduce()
            .withf(|_, key| key == "b")
            .times(1)
            .returning(|state, _| (state, vec![]));
        reducer
            .expect_reduce()
            .withf(|_, key| key == "x")
            .times(1)
            .returning(|state, _| (stopped(state), vec![]));

        let state = controller(
            keys_reading(vec!["b", "x"]),
            screen,
            reducer,
            any_executor(),
        )
        .run(State::default())
        .unwrap();

        assert!(has_command_status(&state, "b", "Add a child box"));
    }

    #[test]
    fn a_keystroke_without_a_command_label_does_not_render_early_or_set_command_status() {
        let mut seq = Sequence::new();
        let mut screen = MockScreen::new();
        screen
            .expect_render()
            .withf(|state| state.command_status().is_none())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| state.led_flash() && state.command_status().is_none())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| !state.led_flash() && state.command_status().is_none())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        screen
            .expect_render()
            .withf(|state| state.led_flash() && state.command_status().is_none())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));

        let mut reducer = MockReducer::new();
        reducer
            .expect_reduce()
            .withf(|_, key| key == "7")
            .times(1)
            .returning(|state, _| (state, vec![]));
        reducer
            .expect_reduce()
            .withf(|_, key| key == "x")
            .times(1)
            .returning(|state, _| (stopped(state), vec![]));

        let state = controller(
            keys_reading(vec!["7", "x"]),
            screen,
            reducer,
            any_executor(),
        )
        .run(State::default())
        .unwrap();

        assert_eq!(state.command_status(), None);
    }

    #[test]
    fn a_resize_key_never_touches_command_status() {
        let mut screen = any_screen();
        screen.expect_resize().times(1).returning(|| Ok(()));

        let state = controller(
            keys_reading(vec![RESIZE, "x"]),
            screen,
            reducer_stopping_on("x"),
            any_executor(),
        )
        .run(State::default())
        .unwrap();

        assert_eq!(state.command_status(), None);
    }
}
