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
    use crate::diagram::{self, Path};
    use crate::state::INTERRUPT;
    use crate::terminal::RESIZE;
    use std::cell::RefCell;
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

    #[derive(Default)]
    pub(in crate::editor) struct Screenings {
        pub(in crate::editor) frames: Vec<State>,
        pub(in crate::editor) resizes: usize,
        pub(in crate::editor) fail_resize: bool,
    }

    pub(in crate::editor) struct RecordingScreen {
        pub(in crate::editor) seen: Rc<RefCell<Screenings>>,
    }

    impl Screen for RecordingScreen {
        fn render(&mut self, state: &State) -> io::Result<()> {
            self.seen.borrow_mut().frames.push(state.clone());
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

    fn run_script(
        state: State,
        script: Vec<Option<&str>>,
        fail_resize: bool,
    ) -> (io::Result<State>, Rc<RefCell<Screenings>>) {
        let seen = Rc::new(RefCell::new(Screenings {
            fail_resize,
            ..Default::default()
        }));
        let keys = ScriptedKeys {
            script: script
                .into_iter()
                .map(|key| key.map(str::to_string))
                .collect::<Vec<_>>()
                .into_iter(),
        };
        let mut controller = DreController::new(
            Box::new(keys),
            Box::new(RecordingScreen { seen: seen.clone() }),
        );
        (controller.run(state), seen)
    }

    fn state_saving_to(path: &str) -> State {
        State::open(Default::default(), Some(path.to_string()))
    }

    #[test]
    fn an_interrupt_clears_where_to_save() {
        let (result, _) = run_script(state_saving_to("a.dre"), vec![Some(INTERRUPT)], false);
        assert_eq!(result.unwrap().save_to, None);
    }

    #[test]
    fn a_quit_stops_the_loop_and_keeps_where_to_save() {
        let (result, _) = run_script(state_saving_to("a.dre"), vec![Some("q")], false);
        let state = result.unwrap();
        assert!(!state.running);
        assert_eq!(state.save_to, Some("a.dre".to_string()));
    }

    #[test]
    fn a_frame_is_rendered_before_the_first_key_is_read() {
        let (result, seen) = run_script(state_saving_to("a.dre"), vec![], false);
        assert_eq!(result.err().unwrap().kind(), io::ErrorKind::UnexpectedEof);
        assert_eq!(seen.borrow().frames.len(), 1);
    }

    #[test]
    fn a_resize_key_resizes_the_screen_once_and_leaves_the_state_alone() {
        let (result, seen) = run_script(
            state_saving_to("a.dre"),
            vec![Some(RESIZE), Some("q")],
            false,
        );
        let state = result.unwrap();
        assert!(!state.running);
        assert_eq!(state.save_to, Some("a.dre".to_string()));
        let seen = seen.borrow();
        assert_eq!(seen.resizes, 1);
        assert_eq!(seen.frames.len(), 2);
        assert!(seen.frames[1].running);
        assert_eq!(seen.frames[1].save_to, Some("a.dre".to_string()));
    }

    #[test]
    fn a_failed_resize_propagates() {
        let (result, _) = run_script(
            state_saving_to("a.dre"),
            vec![Some(RESIZE), Some("q")],
            true,
        );
        let error = result.err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(error.to_string(), "resize failed");
    }

    #[test]
    fn an_idle_second_hides_the_selection_and_the_next_key_restores_it() {
        let selected = Path {
            ancestors: vec![],
            index: 0,
        };
        let state = State::open(
            diagram::Document {
                boxes: vec![diagram::node("a"), diagram::node("b")],
                selected: None,
            },
            Some("a.dre".to_string()),
        );
        let (result, seen) = run_script(state, vec![None, Some("q")], false);
        assert_eq!(result.unwrap().doc.selected, Some(selected.clone()));
        let seen = seen.borrow();
        assert_eq!(seen.frames[0].doc.selected, Some(selected.clone()));
        assert_eq!(seen.frames[1].doc.selected, None);
    }
}
