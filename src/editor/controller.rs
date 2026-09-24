use std::io;

use crate::state::{reduce, State};

pub(crate) trait KeySource {
    fn next_key(&mut self) -> io::Result<Option<String>>;
}

pub(crate) trait Screen {
    fn render(&mut self, state: &State) -> io::Result<()>;
    fn resize(&mut self) -> io::Result<()>;
}

pub(crate) struct DreController {
    keys: Box<dyn KeySource>,
    screen: Box<dyn Screen>,
}

impl DreController {
    pub(crate) fn new(keys: Box<dyn KeySource>, screen: Box<dyn Screen>) -> Self {
        Self { keys, screen }
    }

    pub(crate) fn run(&mut self, state: State) -> io::Result<State> {
        let mut state = state;
        while state.running {
            self.screen.render(&state)?;
            match self.keys.next_key()? {
                Some(key) if key == crate::terminal::RESIZE => self.screen.resize()?,
                key => state = reduce(state, key.as_deref()),
            }
        }
        Ok(state)
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::terminal::RESIZE;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    pub(in crate::editor) struct ScriptedKeys {
        pub(in crate::editor) script: std::vec::IntoIter<Option<String>>,
    }

    impl KeySource for ScriptedKeys {
        fn next_key(&mut self) -> io::Result<Option<String>> {
            self.script
                .next()
                .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "script ended"))
        }
    }

    struct CountingKeys {
        keys: ScriptedKeys,
        reads: Rc<Cell<usize>>,
    }

    impl KeySource for CountingKeys {
        fn next_key(&mut self) -> io::Result<Option<String>> {
            self.reads.set(self.reads.get() + 1);
            self.keys.next_key()
        }
    }

    struct BrokenKeys;

    impl KeySource for BrokenKeys {
        fn next_key(&mut self) -> io::Result<Option<String>> {
            Err(io::Error::other("keys failed"))
        }
    }

    #[derive(Default)]
    pub(in crate::editor) struct Screenings {
        pub(in crate::editor) frames: Vec<State>,
        pub(in crate::editor) resizes: usize,
        pub(in crate::editor) fail_resize: bool,
        pub(in crate::editor) fail_render: bool,
    }

    pub(in crate::editor) struct RecordingScreen {
        pub(in crate::editor) seen: Rc<RefCell<Screenings>>,
    }

    impl Screen for RecordingScreen {
        fn render(&mut self, state: &State) -> io::Result<()> {
            let mut seen = self.seen.borrow_mut();
            seen.frames.push(state.clone());
            if seen.fail_render {
                return Err(io::Error::other("render failed"));
            }
            Ok(())
        }

        fn resize(&mut self) -> io::Result<()> {
            let mut seen = self.seen.borrow_mut();
            seen.resizes += 1;
            if seen.fail_resize {
                return Err(io::Error::other("resize failed"));
            }
            Ok(())
        }
    }

    struct Run {
        result: io::Result<State>,
        seen: Rc<RefCell<Screenings>>,
        key_reads: usize,
    }

    fn run_with(screenings: Screenings, state: State, script: Vec<Option<&str>>) -> Run {
        let seen = Rc::new(RefCell::new(screenings));
        let reads = Rc::new(Cell::new(0));
        let keys = CountingKeys {
            keys: ScriptedKeys {
                script: script
                    .into_iter()
                    .map(|key| key.map(str::to_string))
                    .collect::<Vec<_>>()
                    .into_iter(),
            },
            reads: reads.clone(),
        };
        let mut controller = DreController::new(
            Box::new(keys),
            Box::new(RecordingScreen { seen: seen.clone() }),
        );
        let result = controller.run(state);
        Run {
            result,
            seen,
            key_reads: reads.get(),
        }
    }

    fn run_script(state: State, script: Vec<Option<&str>>) -> Run {
        run_with(Screenings::default(), state, script)
    }

    fn state_that_saves_on_quit() -> State {
        State::open(Default::default(), Some("a.dre".to_string()))
    }

    fn state_with_pending_count(count: usize) -> State {
        let mut state = state_that_saves_on_quit();
        state.pending_count = Some(count);
        state
    }

    fn stopped_state_with_pending_count(count: usize) -> State {
        let mut state = state_with_pending_count(count);
        state.running = false;
        state
    }

    #[test]
    fn a_frame_is_rendered_before_the_first_key_is_read() {
        let run = run_script(state_that_saves_on_quit(), vec![]);
        assert_eq!(
            run.result.err().unwrap().kind(),
            io::ErrorKind::UnexpectedEof
        );
        assert_eq!(run.seen.borrow().frames.len(), 1);
    }

    #[test]
    fn a_state_that_is_not_running_renders_nothing_reads_no_key_and_is_returned_as_given() {
        let run = run_script(stopped_state_with_pending_count(7), vec![Some("q")]);
        assert_eq!(run.result.unwrap().pending_count, Some(7));
        assert_eq!(run.seen.borrow().frames.len(), 0);
        assert_eq!(run.key_reads, 0);
    }

    #[test]
    fn one_frame_is_rendered_per_key_read_until_the_state_stops_running() {
        let run = run_script(
            state_that_saves_on_quit(),
            vec![None, None, Some("q"), None],
        );
        assert!(!run.result.unwrap().running);
        assert_eq!(run.seen.borrow().frames.len(), 3);
        assert_eq!(run.key_reads, 3);
    }

    #[test]
    fn a_resize_key_resizes_the_screen_once_and_does_not_reach_the_reducer() {
        let run = run_script(state_with_pending_count(4), vec![Some(RESIZE), Some("q")]);
        assert!(run.result.is_ok());
        let seen = run.seen.borrow();
        assert_eq!(seen.resizes, 1);
        assert_eq!(seen.frames.len(), 2);
        assert_eq!(seen.frames[1].pending_count, Some(4));
    }

    #[test]
    fn a_key_other_than_resize_is_reduced_and_its_result_becomes_the_state() {
        let run = run_script(state_that_saves_on_quit(), vec![Some("3"), Some("q")]);
        assert!(run.result.is_ok());
        assert_eq!(run.seen.borrow().frames[1].pending_count, Some(3));
    }

    #[test]
    fn a_failed_resize_propagates() {
        let screenings = Screenings {
            fail_resize: true,
            ..Default::default()
        };
        let run = run_with(
            screenings,
            state_that_saves_on_quit(),
            vec![Some(RESIZE), Some("q")],
        );
        assert_eq!(run.result.err().unwrap().to_string(), "resize failed");
    }

    #[test]
    fn a_failed_render_propagates_and_no_key_is_read() {
        let screenings = Screenings {
            fail_render: true,
            ..Default::default()
        };
        let run = run_with(screenings, state_that_saves_on_quit(), vec![Some("q")]);
        assert_eq!(run.result.err().unwrap().to_string(), "render failed");
        assert_eq!(run.key_reads, 0);
    }

    #[test]
    fn a_failing_key_source_propagates_its_error() {
        let mut controller = DreController::new(
            Box::new(BrokenKeys),
            Box::new(RecordingScreen {
                seen: Rc::new(RefCell::new(Screenings::default())),
            }),
        );
        let error = controller.run(state_that_saves_on_quit()).err().unwrap();
        assert_eq!(error.to_string(), "keys failed");
    }
}
